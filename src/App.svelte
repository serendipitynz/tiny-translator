<script lang="ts">
  import { clearBlocker, copyBlocker, stopBlocker, translateBlocker } from "./lib/blockers";
  import { AUTO, promptLanguages, SOURCE_OPTIONS, TARGET_OPTIONS } from "./lib/languages";
  import { isTranslateShortcut } from "./lib/shortcuts";
  import { stopTranslation, translate as translateText } from "./lib/translate";
  import { announce } from "./lib/ui/announce";
  import Button from "./lib/ui/Button.svelte";
  import IconButton from "./lib/ui/IconButton.svelte";
  import Panel from "./lib/ui/Panel.svelte";
  import Select from "./lib/ui/Select.svelte";
  import TextArea from "./lib/ui/TextArea.svelte";

  let source = $state("");
  let target = $state("");
  let sourceLanguage = $state(AUTO);
  let targetLanguage = $state(AUTO);
  let translating = $state(false);
  /** Why the last translation could not be made, shown until the next one starts. */
  let failure = $state<string | null>(null);
  /** Counts translations, so pieces and results of one that was stopped no longer reach the screen. */
  let current = 0;

  async function translate() {
    const id = ++current;
    translating = true;
    failure = null;
    target = "";
    try {
      const request = { text: source, ...promptLanguages(sourceLanguage, targetLanguage) };
      await translateText(request, (piece) => {
        if (id === current) {
          target += piece;
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

  function onKeydown(event: KeyboardEvent) {
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

  // Each action below is where a later task connects its work — swapping the direction (TASK-10),
  // paste, copy and clear (TASK-11), history (TASK-20) and settings (TASK-21).
  function swap() {}
  function paste() {}
  function copySource() {}
  function copyTarget() {}
  function clearSource() {}
  function openHistory() {}
  function openSettings() {}
</script>

<svelte:window onkeydown={onKeydown} />

<div class="screen">
  <header class="app-header">
    <h1>tiny-translator</h1>
    <div class="tools">
      <IconButton icon="history" label="翻訳履歴" onpress={openHistory} />
      <IconButton icon="settings" label="設定" onpress={openSettings} />
    </div>
  </header>

  <main>
    <div class="directions">
      <div class="source-direction">
        <Select label="翻訳元" options={SOURCE_OPTIONS} bind:value={sourceLanguage} />
        <IconButton icon="arrow-left-right" label="翻訳元と翻訳先を入れ替える" onpress={swap} />
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
          <TextArea bind:value={source} labelledby={headingId} placeholder="翻訳する文を入力" />
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
