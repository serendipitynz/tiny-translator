<script lang="ts">
  import { onMount } from "svelte";
  import { clearBlocker, copyBlocker, stopBlocker, swapBlocker, translateBlocker } from "./lib/blockers";
  import {
    AUTO,
    DEFAULT_PRIORITIES,
    detectionLabel,
    promptLanguages,
    SOURCE_OPTIONS,
    TARGET_OPTIONS,
  } from "./lib/languages";
  import { copyText, launchClipboard, onLaunchedAgain, pasteClipboard } from "./lib/clipboard";
  import { formatOptions } from "./lib/formats";
  import { type HistoryEntry, restore } from "./lib/history";
  import HistoryModal from "./HistoryModal.svelte";
  import { isCopyTranslationShortcut, isTranslateShortcut } from "./lib/shortcuts";
  import {
    DEFAULT_UNTRANSLATED_KEYS,
    detectFormat,
    detectLanguage,
    type Format,
    stopTranslation,
    translate as translateText,
  } from "./lib/translate";
  import { announce } from "./lib/ui/announce";
  import Button from "./lib/ui/Button.svelte";
  import IconButton from "./lib/ui/IconButton.svelte";
  import Panel from "./lib/ui/Panel.svelte";
  import Select from "./lib/ui/Select.svelte";
  import TextArea from "./lib/ui/TextArea.svelte";
  import Toast from "./lib/ui/Toast.svelte";

  let source = $state("");
  let target = $state("");
  let sourceLanguage = $state(AUTO);
  let targetLanguage = $state(AUTO);
  /** The format selector's choice: AUTO, or the Format to read the source as (doc-1 §7.2). */
  let sourceFormat = $state(AUTO);
  let translating = $state(false);
  /** Why the last translation, paste or copy could not be made, shown until the next one starts. */
  let failure = $state<string | null>(null);
  /**
   * Whether 起動時のクリップボード翻訳 left the clipboard unread because macOS would have asked the
   * user (doc-1 §6.3); the empty source pane then says how to let the app read it.
   */
  let launchUnread = $state(false);
  let toast: Toast;
  /** Counts translations, so pieces and results of one that was stopped no longer reach the screen. */
  let current = 0;
  /**
   * The source language detected last (null for doc-1 §6.1's 判定なし), or null before the first
   * detection: shown only once there is one, so typing the first letters does not flash 「判定: 不明」.
   */
  let detection = $state<{ language: string | null } | null>(null);
  /** Counts detections, so an answer that arrives after a newer one was asked for is dropped. */
  let detecting = 0;
  /**
   * The format detected last from a source that is not blank. Detected whichever format is chosen,
   * so 自動 in the selector's list says what it would read the source as.
   */
  let formatDetection = $state<Format | null>(null);
  /** Counts format detections, as `detecting` does language ones. */
  let detectingFormat = 0;

  // Long enough that a word typed at speed is detected once, short enough to read as immediate.
  const DETECT_DELAY_MS = 200;

  $effect(() => {
    const text = source;
    if (sourceLanguage !== AUTO || text.trim() === "") {
      return;
    }
    const id = ++detecting;
    const timer = setTimeout(async () => {
      try {
        const language = await detectLanguage(text);
        if (id === detecting) {
          detection = { language };
        }
      } catch {
        // The label keeps the last answer; translating detects again and reports its own failure.
      }
    }, DETECT_DELAY_MS);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    const text = source;
    if (text.trim() === "") {
      return;
    }
    const id = ++detectingFormat;
    const timer = setTimeout(async () => {
      try {
        const format = await detectFormat(text);
        if (id === detectingFormat) {
          formatDetection = format;
        }
      } catch {
        // The selector keeps the last answer; translating detects again and reports its own failure.
      }
    }, DETECT_DELAY_MS);
    return () => clearTimeout(timer);
  });

  async function translate() {
    const id = ++current;
    translating = true;
    failure = null;
    target = "";
    const text = source;
    const sourceSelected = sourceLanguage;
    const targetSelected = targetLanguage;
    const formatSelected = sourceFormat;
    try {
      // Detected again from the text being translated, not taken from the labels, whose detection
      // may still be waiting out its delay.
      const [detected, format] = await Promise.all([
        sourceSelected === AUTO ? detectLanguage(text) : null,
        formatSelected === AUTO ? detectFormat(text) : (formatSelected as Format),
      ]);
      if (id !== current) {
        return;
      }
      if (sourceSelected === AUTO) {
        ++detecting;
        detection = { language: detected };
      }
      if (formatSelected === AUTO) {
        ++detectingFormat;
        formatDetection = format;
      }
      const request = {
        text,
        format,
        untranslatedKeys: DEFAULT_UNTRANSLATED_KEYS,
        ...promptLanguages(sourceSelected, targetSelected, detected, DEFAULT_PRIORITIES),
      };
      await translateText(request, (text) => {
        if (id === current) {
          target = text;
        }
      });
    } catch (error) {
      if (id === current) {
        // The busy display goes first, then the failure is told (snz-design doc-8 §5.8).
        translating = false;
        failure = `翻訳できませんでした: ${error}`;
        announce(failure);
      }
    } finally {
      if (id === current) {
        translating = false;
      }
    }
  }

  // The screen is back at rest at once, even while the engine is still loading: the translation
  // that was waiting for it sees the stop and sends nothing.
  function stop() {
    current += 1;
    translating = false;
    void stopTranslation();
    announce("翻訳を停止しました");
  }

  // doc-1 §6.3, at launch and at every launch after it while the app runs (§5).
  async function translateClipboard() {
    let found: Awaited<ReturnType<typeof launchClipboard>>;
    try {
      found = await launchClipboard();
    } catch {
      // Nothing was asked for; the user brings the text in another way.
      return;
    }
    launchUnread = found.kind === "not-allowed";
    if (found.kind === "text") {
      source = found.text;
      void translate();
    }
  }

  onMount(() => {
    void translateClipboard();
    return onLaunchedAgain(() => void translateClipboard());
  });

  function onKeydown(event: KeyboardEvent) {
    if (isCopyTranslationShortcut(event)) {
      event.preventDefault();
      const reason = copyBlocker(target, "訳文");
      if (reason !== null) {
        announce(reason);
        return;
      }
      void copyTarget();
      return;
    }
    if (!isTranslateShortcut(event)) {
      return;
    }
    event.preventDefault();
    if (translating) {
      return;
    }
    const reason = translateBlocker(source);
    if (reason !== null) {
      announce(reason);
      return;
    }
    void translate();
  }

  // doc-1 §6.2: the selectors trade values as they are, 自動 for 自動, and the translation becomes
  // the source.
  function swap() {
    [sourceLanguage, targetLanguage] = [targetLanguage, sourceLanguage];
    [source, target] = [target, source];
    announce("翻訳元と翻訳先を入れ替えました");
  }

  // Replaces the source without translating it (2026-11-07 owner decision): the user may still
  // want to edit it or pick the languages first.
  async function paste() {
    const text = await pasteClipboard().catch(() => null);
    if (text === null || text === "") {
      failure = "クリップボードに貼り付けられる文字列がありません";
      announce(failure);
      return;
    }
    failure = null;
    source = text;
  }

  async function copy(text: string, what: "原文" | "訳文") {
    try {
      await copyText(text);
    } catch (error) {
      // A failure is a situation, not the operation's result: told where failures are, not in a toast (doc-9 §6.5).
      failure = `${what}をコピーできませんでした: ${error}`;
      announce(failure);
      return;
    }
    toast.show(`${what}をコピーしました`);
  }

  const copySource = () => copy(source, "原文");
  const copyTarget = () => copy(target, "訳文");

  function clearSource() {
    source = "";
    announce("原文をクリアしました");
  }

  let historyOpen = $state(false);

  // doc-1 §9. A translation still running is stopped first: what it would write is no longer what the
  // screen is about.
  function pickFromHistory(entry: HistoryEntry) {
    const stopped = translating;
    if (stopped) {
      current += 1;
      translating = false;
      void stopTranslation();
    }
    const restored = restore(entry);
    source = restored.source;
    target = restored.target;
    sourceLanguage = restored.sourceLanguage;
    targetLanguage = restored.targetLanguage;
    failure = null;
    historyOpen = false;
    announce(stopped ? "翻訳を停止して、翻訳履歴から戻しました" : "翻訳履歴から戻しました");
  }

  // Where TASK-21 connects the settings screen.
  function openSettings() {}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="screen">
  <header class="app-header">
    <h1>tiny-translator</h1>
    <div class="tools">
      <IconButton icon="history" label="翻訳履歴" onpress={() => (historyOpen = true)} />
      <IconButton icon="settings" label="設定" onpress={openSettings} />
    </div>
  </header>

  <main>
    <div class="directions">
      <div class="source-direction">
        <Select label="翻訳元" options={SOURCE_OPTIONS} bind:value={sourceLanguage} />
        <IconButton
          icon="arrow-left-right"
          label="翻訳元と翻訳先を入れ替える"
          disabledReason={swapBlocker(translating)}
          onpress={swap}
        />
      </div>
      <Select label="翻訳先" options={TARGET_OPTIONS} bind:value={targetLanguage} />
    </div>

    <div class="panes">
      <Panel heading="原文">
        {#snippet actions()}
          <IconButton icon="clipboard-paste" label="原文に貼り付け" onpress={paste} />
          <IconButton icon="copy" label="原文をコピー" disabledReason={copyBlocker(source, "原文")} onpress={copySource} />
          <IconButton icon="x" label="原文をクリア" disabledReason={clearBlocker(source)} onpress={clearSource} />
        {/snippet}
        {#snippet children(headingId)}
          <div class="source-field">
            <TextArea bind:value={source} labelledby={headingId} placeholder="翻訳する文を入力" />
            {#if launchUnread && source === ""}
              <p class="launch-hint">
                起動時にクリップボードを翻訳するには、システム設定の「プライバシーとセキュリティ」にある「ほかのアプリからのペースト」で
                tiny-translator を許可してください。一覧に無いときは、先に貼り付けボタンを一度押して確認に答えると並びます。⌘V
                でも貼り付けられます。
              </p>
            {/if}
            <div class="source-reading">
              <Select
                label="形式"
                options={formatOptions(source.trim() === "" ? null : formatDetection)}
                bind:value={sourceFormat}
              />
              {#if sourceLanguage === AUTO && source.trim() !== "" && detection !== null}
                <p class="detection">{detectionLabel(detection.language)}</p>
              {/if}
            </div>
          </div>
        {/snippet}
      </Panel>
      <Panel heading="訳文">
        {#snippet actions()}
          <IconButton icon="copy" label="訳文をコピー" disabledReason={copyBlocker(target, "訳文")} onpress={copyTarget} />
        {/snippet}
        {#snippet children(headingId)}
          <TextArea bind:value={target} labelledby={headingId} placeholder="訳文はここに出ます" readonly />
        {/snippet}
      </Panel>
    </div>

    <div class="actions">
      {#if failure !== null}
        <p class="failure">{failure}</p>
      {/if}
      <Button label="停止" disabledReason={stopBlocker(translating)} onpress={stop} />
      <Button
        label="翻訳"
        busyLabel="翻訳中…"
        variant="primary"
        busy={translating}
        disabledReason={translateBlocker(source)}
        onpress={translate}
      />
    </div>
  </main>
</div>

{#if historyOpen}
  <HistoryModal onpick={pickFromHistory} onclose={() => (historyOpen = false)} notify={(words) => toast.show(words)} />
{/if}

<Toast bind:this={toast} />

<style lang="scss">
  .screen {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    block-size: 100vh;
  }

  .app-header {
    display: flex;
    align-items: center;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-xs) var(--snz-space-sm) var(--snz-space-xs) var(--snz-space-md);
    background: var(--snz-surface);
    border-block-end: var(--snz-border-line) solid var(--snz-line-divider);
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: var(--snz-font-size-title);
    font-weight: var(--snz-font-weight-bold);
    line-height: var(--snz-font-line-height-heading);
    color: var(--snz-fg-strong);
  }

  .tools {
    display: flex;
    gap: var(--snz-space-xs);
  }

  main {
    // The pane layout follows the room main has, not the window, so the rule stays right if a side
    // area (the history list, say) ever shares the window.
    container: main / inline-size;
    display: grid;
    // The panes may not shrink below their own minimum; when the window is shorter than that, main
    // scrolls instead of the panes overlapping the translate button.
    grid-template-rows: auto minmax(min-content, 1fr) auto;
    gap: var(--snz-space-md);
    min-block-size: 0;
    padding: var(--snz-space-md);
    overflow-y: auto;
  }

  // Laid out on the panes' own columns, so each selector stands over the pane it names; the swap
  // closes the source column, on the boundary between the two.
  .directions,
  .panes {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--snz-space-md);
  }

  .directions {
    align-items: center;
  }

  .panes {
    min-block-size: 0;
  }

  .source-direction {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--snz-space-sm);
    min-inline-size: 0;
  }

  // doc-1 §8.1: how the source is read — its format and language — sits under it, inside the pane.
  .source-field {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-block-size: 0;
  }

  .source-reading,
  .launch-hint {
    padding: var(--snz-space-xs) var(--snz-space-md);
    border-block-start: var(--snz-border-line) solid var(--snz-line-divider);
  }

  .source-reading {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--snz-space-xs) var(--snz-space-md);
  }

  .detection,
  .launch-hint {
    margin: 0;
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--snz-space-sm);
  }

  // Told in words, not colour alone (snz-design doc-8 §5.8); the buttons keep their place at the end.
  .failure {
    flex: 1;
    min-inline-size: 0;
    margin: 0;
    color: var(--snz-danger);
    overflow-wrap: anywhere;
  }

  // doc-1 §8.1: on a narrow window the source and the translation stack.
  @container main (inline-size < 40rem) {
    .panes {
      grid-template-columns: minmax(0, 1fr);
      // Stacked, each pane keeps room for its heading and a couple of lines (TextArea has no floor).
      grid-template-rows: repeat(2, minmax(8rem, 1fr));
    }
  }
</style>
