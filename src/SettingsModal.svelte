<script lang="ts">
  import { tick } from "svelte";
  import {
    type Appearance,
    appearanceNotice,
    applyAppearance,
    appearanceToStore,
    FAMILY_OPTIONS,
    MODE_OPTIONS,
    readAppearance,
    type StoredAppearance,
    writeAppearance,
  } from "./lib/appearance";
  import { type EngineStatus, watchEngine } from "./lib/engine";
  import { LANGUAGE_OPTIONS } from "./lib/languages";
  import {
    engineMemory,
    engineStateLabel,
    type FileState,
    fileNotice,
    idleReleaseOptions,
    keysFromText,
    keysToText,
    languageNotice,
    listModels,
    loadSettings,
    type Models,
    modelOptions,
    priorities,
    sameSettings,
    saveBlocker,
    saveSettings,
    type Settings,
    sizeLabel,
  } from "./lib/settings";
  import { announce } from "./lib/ui/announce";
  import Button from "./lib/ui/Button.svelte";
  import Checkbox from "./lib/ui/Checkbox.svelte";
  import Modal from "./lib/ui/Modal.svelte";
  import Notice from "./lib/ui/Notice.svelte";
  import Panel from "./lib/ui/Panel.svelte";
  import Select from "./lib/ui/Select.svelte";
  import TextArea from "./lib/ui/TextArea.svelte";
  import ModelFetchModal from "./ModelFetchModal.svelte";

  interface Props {
    /** The settings are in effect: saved, or for this launch only when saving failed (doc-7 §5.3). */
    onsaved: (settings: Settings) => void;
    onclose: () => void;
    /** Tells the result of an operation done here (snz-design doc-9 §6.5). */
    notify: (words: string) => void;
  }

  let { onsaved, onclose, notify }: Props = $props();

  // How often the memory 目安 is read while the engine runs: often enough to follow a load, which
  // takes a few seconds (doc-1 §4.5), without running `ps` all the time.
  const MEMORY_POLL_MS = 2000;

  /** The settings in effect, which the draft is compared with; null until they are read. */
  let saved = $state<Settings | null>(null);
  let file = $state<FileState>({ state: "read" });
  /** Why the settings could not be read; the form is not drawn without them. */
  let loadFailure = $state<string | null>(null);
  /** The stored languages as read, for saying which of them the selectors do not list. */
  let storedLanguages = $state<Settings | null>(null);

  let firstLanguage = $state("");
  let secondLanguage = $state("");
  let idleRelease = $state("10");
  let launchClipboard = $state(true);
  let keysText = $state("");

  let models = $state<Models | null>(null);
  let modelsFailure = $state<string | null>(null);
  /** The model selector's value; "" until the list has been read. */
  let modelValue = $state("");
  /** What the selector showed when the list arrived: left there, the saved choice stays as it was (null included). */
  let modelShown = "";

  // Read here, at open, rather than handed in: what index.html drew may differ from what is stored only
  // in the cases doc-7 §5 names, and those are what the notices below say.
  const initialAppearance = readStorage();
  let storedAppearance = $state<StoredAppearance>(initialAppearance);
  let family = $state(initialAppearance.appearance.family);
  let mode = $state(initialAppearance.appearance.mode);
  /** The appearance drawn now, which the draft is compared with. */
  let shownAppearance = $state<Appearance>(initialAppearance.appearance);
  /** Whether the appearance drawn now could not be stored, so the next save tries it again. */
  let appearanceUnstored = $state(false);

  let engine = $state<EngineStatus | null>(null);
  let memory = $state<number | null>(null);

  let saving = $state(false);
  let saveFailure = $state<string | null>(null);
  let confirmingDiscard = $state(false);
  let sections: HTMLElement;
  let fileNoticeDismissed = $state(false);
  let appearanceNoticeDismissed = $state(false);

  function readStorage(): StoredAppearance {
    try {
      return readAppearance(window.localStorage);
    } catch {
      // Reaching localStorage at all can throw where storage is blocked.
      return readAppearance({
        getItem: () => {
          throw new Error("unreadable");
        },
      });
    }
  }

  async function load() {
    try {
      const loaded = await loadSettings();
      storedLanguages = loaded.settings;
      // Seeded with the priorities translations actually use, so the selectors never show a code they lack.
      const used = priorities(loaded.settings);
      const settings = { ...loaded.settings, firstLanguage: used.first, secondLanguage: used.second };
      saved = settings;
      file = loaded.file;
      firstLanguage = settings.firstLanguage;
      secondLanguage = settings.secondLanguage;
      idleRelease = String(settings.idleReleaseMinutes);
      launchClipboard = settings.launchClipboard;
      keysText = keysToText(settings.untranslatedKeys);
    } catch (error) {
      loadFailure = `設定を読み込めませんでした: ${error}`;
      return;
    }
    await readModels();
  }

  async function readModels() {
    try {
      const listed = await listModels();
      const first = models === null;
      models = listed;
      modelsFailure = null;
      // A list read again after a fetch leaves the selector as the user set it, unless it had
      // nothing to show.
      if (first || modelValue === "") {
        modelShown = saved?.model ?? listed.chosen ?? "";
        modelValue = modelShown;
      }
    } catch (error) {
      modelsFailure = `モデルの一覧を読み込めませんでした: ${error}`;
    }
  }

  void load();

  let fetchOpen = $state(false);

  $effect(() => watchEngine((status) => (engine = status)));

  $effect(() => {
    if (engine === null || engine.state === "stopped") {
      memory = null;
      return;
    }
    let live = true;
    const read = () =>
      engineMemory()
        .then((bytes) => {
          if (live) {
            memory = bytes;
          }
        })
        .catch(() => {
          // The 目安 keeps its last reading; the next poll tries again.
        });
    void read();
    const timer = setInterval(read, MEMORY_POLL_MS);
    return () => {
      live = false;
      clearInterval(timer);
    };
  });

  /** The model as it would be saved: the saved choice (null meaning the default) unless another was picked. */
  let model = $derived(saved === null || modelValue === modelShown ? (saved?.model ?? null) : modelValue);

  let pending = $derived<Settings | null>(
    saved === null
      ? null
      : {
          firstLanguage,
          secondLanguage,
          idleReleaseMinutes: Number(idleRelease),
          model,
          launchClipboard,
          untranslatedKeys: keysFromText(keysText),
        },
  );
  let appearance = $derived<Appearance>({ family, mode });
  let appearanceChanged = $derived(family !== shownAppearance.family || mode !== shownAppearance.mode);
  let dirty = $derived(saved !== null && pending !== null && (!sameSettings(pending, saved) || appearanceChanged));

  let blocker = $derived(
    pending === null ? (loadFailure === null ? "設定を読み込んでいます" : "設定を読み込めませんでした") : saveBlocker(pending),
  );

  let fileWords = $derived(fileNoticeDismissed ? null : fileNotice(file));
  let appearanceWords = $derived(appearanceNoticeDismissed ? null : appearanceNotice(storedAppearance));
  let unlistedLanguages = $derived(storedLanguages === null ? null : languageNotice(storedLanguages));
  let missingModel = $derived(
    models !== null && saved?.model != null && !models.models.some((listed) => listed.name === saved?.model)
      ? saved.model
      : null,
  );

  async function save() {
    if (pending === null || blocker !== null) {
      return;
    }
    const settings = pending;
    const chosen = appearance;
    saving = true;
    saveFailure = null;
    const failures: string[] = [];
    try {
      await saveSettings(settings);
      file = { state: "read" };
    } catch (error) {
      failures.push(`設定を保存できませんでした。この起動の間だけ、選んだ設定で動きます。(${error})`);
    }
    if (appearanceToStore(chosen, shownAppearance, appearanceUnstored)) {
      // Drawn whether or not it is stored: the user chose a scheme, not a save (doc-7 §5.3).
      applyAppearance(chosen);
      let stored = false;
      try {
        stored = writeAppearance(window.localStorage, chosen);
      } catch {
        stored = false;
      }
      appearanceUnstored = !stored;
      if (stored) {
        storedAppearance = { appearance: chosen, state: "read", stored: { ...chosen } };
      } else {
        failures.push("配色の選択を保存できませんでした。この起動の間だけ、選んだ配色で表示します。");
      }
      shownAppearance = chosen;
    }
    // In effect either way, so it is what an unsaved change is measured against from here.
    saved = settings;
    modelShown = modelValue;
    storedLanguages = settings;
    onsaved(settings);
    saving = false;
    if (failures.length === 0) {
      notify("設定を保存しました");
      onclose();
      return;
    }
    // doc-9 §6.6: a failed run leaves the modal open with what was chosen, and says so in it.
    saveFailure = failures.join(" ");
    announce(saveFailure);
    // Shown at the top of the body, which the press on 保存 below may have scrolled out of view; the
    // focus stays where it is (doc-8 §5.8).
    await tick();
    sections.scrollIntoView({ block: "start" });
  }

  // doc-9 §5.7: every way of closing — ×, Escape, the scrim, キャンセル — goes through the same question
  // when there is something unsaved to lose.
  function requestClose() {
    if (dirty) {
      confirmingDiscard = true;
      return;
    }
    onclose();
  }

  function engineLine(status: EngineStatus | null, bytes: number | null): string {
    if (status === null) {
      return "状態を読み込んでいます";
    }
    const state = `状態: ${engineStateLabel(status.state)}`;
    if (status.state === "stopped") {
      return `${state} (メモリを使っていません)`;
    }
    return bytes === null ? state : `${state}・メモリ使用量の目安: ${sizeLabel(bytes)}`;
  }
</script>

<Modal heading="設定" closeBlocker={saving ? "保存が終わるまで閉じられません" : null} onclose={requestClose}>
  <div class="sections" bind:this={sections}>
    {#if loadFailure !== null}
      <Notice level="failure" words={loadFailure} />
    {/if}
    {#if saveFailure !== null}
      <Notice level="failure" words={saveFailure} />
    {/if}
    {#if fileWords !== null}
      <Notice
        level={fileWords.level}
        words={fileWords.words}
        ondismiss={fileWords.level === "info" ? () => (fileNoticeDismissed = true) : undefined}
      />
    {/if}

    {#if saved !== null}
      <Panel heading="翻訳の向き">
        {#snippet children()}
          <div class="fields">
            {#if unlistedLanguages !== null}
              <Notice level="info" words={unlistedLanguages} />
            {/if}
            <Select label="第一優先言語" options={LANGUAGE_OPTIONS} bind:value={firstLanguage} />
            <Select label="第二優先言語" options={LANGUAGE_OPTIONS} bind:value={secondLanguage} />
            <p class="hint">
              翻訳先が「自動 (優先順位)」のとき、原文が第一優先言語なら第二優先言語へ、それ以外なら第一優先言語へ翻訳します。
            </p>
          </div>
        {/snippet}
      </Panel>

      <Panel heading="翻訳エンジン">
        {#snippet children()}
          <div class="fields">
            <!-- doc-1 §4.5: polite, so a load finishing while the user is elsewhere in the form is still heard. -->
            <p class="engine" aria-live="polite">{engineLine(engine, memory)}</p>
            {#if engine?.error}
              <Notice level="degraded" words={`翻訳エンジンを起動できませんでした: ${engine.error}`} />
            {/if}
            {#if modelsFailure !== null}
              <Notice level="failure" words={modelsFailure} />
            {:else if models !== null && models.models.length === 0}
              <Notice
                level="degraded"
                words={`モデルの GGUF がありません。モデルを入手するか、${models.dir} に置くと、次の翻訳から使えます。`}
              />
            {:else if models !== null}
              {#if missingModel !== null}
                <Notice
                  level="info"
                  words={`保存されているモデル「${missingModel}」が見つからないため、${models.chosen ?? "既定のモデル"} を使っています。`}
                />
              {/if}
              <Select label="モデル" options={modelOptions(models.models, missingModel)} bind:value={modelValue} />
              <p class="hint">
                {models.dir} にある GGUF から選びます。括弧の中のサイズが、読み込んだときのメモリ使用量の目安です。保存すると、翻訳エンジンが選んだモデルを読み込み直します。
              </p>
            {/if}
            {#if models !== null}
              <Button label="モデルを入手" onpress={() => (fetchOpen = true)} />
            {/if}
            <Select
              label="自動解放までの時間"
              options={idleReleaseOptions(Number(idleRelease))}
              bind:value={idleRelease}
            />
            <p class="hint">最後の翻訳からこの時間が過ぎると翻訳エンジンを終了し、使っていたメモリを返します。次の翻訳で起動し直します。</p>
          </div>
        {/snippet}
      </Panel>

      <Panel heading="起動時">
        {#snippet children()}
          <div class="fields">
            <Checkbox
              label="起動時にクリップボードを翻訳する"
              description="起動したときと、起動中のアプリをもう一度開いたときに、クリップボードの文字列を原文に入れて翻訳します。"
              bind:checked={launchClipboard}
            />
          </div>
        {/snippet}
      </Panel>

      <Panel heading="JSON の翻訳しないキー">
        {#snippet children(headingId)}
          <div class="keys">
            <TextArea bind:value={keysText} labelledby={headingId} placeholder="1 行に 1 つ (例: id)" />
          </div>
        {/snippet}
      </Panel>
      <p class="hint keys-hint">
        JSON を翻訳するとき、ここに書いた名前のキーの値は訳さずに残します。キーの名前が完全に一致すれば、どの深さでも残し、値が配列やオブジェクトなら中身もすべて残します。
      </p>
    {/if}

    <!-- doc-9 §7: the panel 外観 holds the two axes and what doc-7 §5 says about them. Both families
         carry both sides, so the mode never narrows and the group needs no 説明域 here. -->
    <Panel heading="外観">
      {#snippet children()}
        <div class="fields">
          {#if appearanceWords !== null}
            <Notice
              level={appearanceWords.level}
              words={appearanceWords.words}
              ondismiss={appearanceWords.level === "info" ? () => (appearanceNoticeDismissed = true) : undefined}
            />
          {/if}
          <Select label="配色系統" options={FAMILY_OPTIONS} bind:value={family} />
          <Select label="明暗" options={MODE_OPTIONS} bind:value={mode} />
        </div>
      {/snippet}
    </Panel>
  </div>

  {#snippet actions()}
    <Button label="キャンセル" disabledReason={saving ? "保存が終わるまで取り消せません" : null} onpress={requestClose} />
    <Button label="保存" busyLabel="保存中…" variant="primary" busy={saving} disabledReason={blocker} onpress={save} />
  {/snippet}
</Modal>

{#if fetchOpen}
  <ModelFetchModal
    onfetched={() => {
      fetchOpen = false;
      void readModels();
    }}
    onclose={() => (fetchOpen = false)}
    {notify}
  />
{/if}

{#if confirmingDiscard}
  <Modal heading="変更を破棄しますか" onclose={() => (confirmingDiscard = false)}>
    <p class="confirm">設定の変更をまだ保存していません。破棄すると、変更前の設定のままになります。</p>
    {#snippet actions()}
      <!-- doc-9 §5.7: the default is to stay, so the focus starts there. -->
      <Button label="編集に戻る" autofocus onpress={() => (confirmingDiscard = false)} />
      <Button label="破棄して閉じる" variant="danger" onpress={onclose} />
    {/snippet}
  </Modal>
{/if}

<style lang="scss">
  .sections {
    display: grid;
    gap: var(--snz-space-md);
  }

  .fields {
    display: grid;
    justify-items: start;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-sm) var(--snz-space-md);

    // A notice spans the panel, not the width of its words.
    > :global(.notice) {
      justify-self: stretch;
    }
  }

  .keys {
    block-size: 6rem;
  }

  .hint,
  .engine,
  .confirm {
    margin: 0;
  }

  .hint {
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }

  // Under its panel rather than inside it: the field fills the panel's body (TextArea).
  .keys-hint {
    margin-block-start: calc(-1 * var(--snz-space-sm));
  }
</style>
