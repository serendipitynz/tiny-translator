<script lang="ts">
  import {
    type Catalog,
    type CatalogModel,
    cancelFetch,
    catalogOptions,
    type FetchProgress,
    fetchModel,
    firstChoice,
    LICENCE,
    LICENCE_LANGUAGE_OPTIONS,
    type LicenceLanguage,
    loadCatalog,
    progressReadout,
  } from "./lib/models";
  import { announce } from "./lib/ui/announce";
  import Button from "./lib/ui/Button.svelte";
  import Checkbox from "./lib/ui/Checkbox.svelte";
  import Modal from "./lib/ui/Modal.svelte";
  import Notice from "./lib/ui/Notice.svelte";
  import Panel from "./lib/ui/Panel.svelte";
  import Progress from "./lib/ui/Progress.svelte";
  import Select from "./lib/ui/Select.svelte";

  interface Props {
    /** A model is now in the models directory; the caller closes the modal. */
    onfetched: () => void;
    onclose: () => void;
    /** Tells the result of an operation done here (snz-design doc-9 §6.5). */
    notify: (words: string) => void;
  }

  let { onfetched, onclose, notify }: Props = $props();

  let catalog = $state<Catalog | null>(null);
  let loadFailure = $state<string | null>(null);
  let choice = $state("");
  let licenceLanguage = $state<string>("ja");
  let agreed = $state(false);
  /** The model being fetched and how far it has got; null while nothing is fetched. */
  let running = $state<{ model: CatalogModel; progress: FetchProgress } | null>(null);
  let failure = $state<string | null>(null);

  async function reload() {
    try {
      const loaded = await loadCatalog();
      catalog = loaded;
      if (!loaded.models.some((model) => model.name === choice && !model.present)) {
        choice = firstChoice(loaded.models);
      }
    } catch (error) {
      loadFailure = `入手できるモデルを読み込めませんでした: ${error}`;
    }
  }

  void reload();

  let options = $derived(catalog === null ? [] : catalogOptions(catalog.models));
  let chosen = $derived(catalog?.models.find((model) => model.name === choice) ?? null);

  let blocker = $derived(
    catalog === null
      ? loadFailure === null
        ? "入手できるモデルを読み込んでいます"
        : "入手できるモデルを読み込めませんでした"
      : chosen === null
        ? "入手できるモデルがありません"
        : agreed
          ? null
          : "ライセンスに同意すると入手できます",
  );

  async function start() {
    if (chosen === null || blocker !== null || running !== null) {
      return;
    }
    const model = chosen;
    failure = null;
    running = { model, progress: { phase: "fetching", bytes: model.partial } };
    let outcome: "done" | "cancelled";
    try {
      outcome = await fetchModel(model.name, (progress) => {
        if (running !== null) {
          running = { model, progress };
        }
      });
    } catch (error) {
      running = null;
      failure = `モデルを入手できませんでした: ${error}`;
      announce(failure);
      await reload();
      return;
    }
    running = null;
    if (outcome === "cancelled") {
      announce("入手を停止しました。次に入手するときは続きから取得します");
      await reload();
      return;
    }
    notify(`${model.size} のモデルを入手しました`);
    onfetched();
  }

  function stop() {
    void cancelFetch();
  }

  function progressLabel(progress: FetchProgress): string {
    return progress.phase === "fetching" ? "モデルを取得しています" : "SHA-256 を照合しています";
  }

  let licence = $derived(LICENCE.texts[licenceLanguage as LicenceLanguage] ?? LICENCE.texts.ja);
</script>

<Modal
  heading="モデルを入手"
  closeBlocker={running === null ? null : "入手を停止すると閉じられます"}
  onclose={onclose}
>
  <div class="sections">
    {#if loadFailure !== null}
      <Notice level="failure" words={loadFailure} />
    {/if}
    {#if failure !== null}
      <Notice level="failure" words={failure} />
    {/if}

    {#if running !== null}
      <!-- While it runs, the choice and the licence are settled and only the progress is news; shown
           alone, it is not pushed out of view below them. -->
      <p class="running">
        {running.model.size} のモデルを Hugging Face の {running.model.repo} から入手しています。「停止」で止めても、次は続きから取得します。
      </p>
      <Progress
        label={progressLabel(running.progress)}
        total={running.model.bytes}
        done={running.progress.bytes}
        readout={progressReadout(running.progress.bytes, running.model.bytes)}
      />
    {:else if catalog !== null}
      {#if options.length === 0}
        <Notice level="info" words={`入手できるモデルは、すべて ${catalog.dir} にあります。`} />
      {:else}
        <div class="fields">
          <Select label="モデル" options={options} bind:value={choice} />
          {#if chosen !== null}
            <p class="hint">
              Hugging Face の {chosen.repo} から取得し、SHA-256 を照合してから {catalog.dir} に置きます。入手した後は、翻訳にネットワークを使いません。
            </p>
            {#if chosen.partial > 0}
              <p class="hint">前回の入手で取得した {progressReadout(chosen.partial, chosen.bytes)} の続きから取得します。</p>
            {/if}
          {/if}
        </div>

        <Panel heading="PLaMo Community License">
          {#snippet children(headingId)}
            <div class="licence-body">
              <Select label="言語" options={LICENCE_LANGUAGE_OPTIONS} bind:value={licenceLanguage} />
              <!-- Focusable so the text can be scrolled from the keyboard. -->
              <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
              <div class="licence" role="region" aria-labelledby={headingId} tabindex="0">{licence}</div>
              <p class="hint">{LICENCE.revised}。入手するモデルと同じコミットにある条文です。</p>
            </div>
          {/snippet}
        </Panel>

        <Checkbox
          label="PLaMo Community License に同意する"
          description="18 歳以上か、居住国で単独で契約できる年齢に達していること、またはそうでないときは親権者か法定代理人が同意していることを含みます (第 2 条)。"
          bind:checked={agreed}
        />
      {/if}
    {/if}
  </div>

  {#snippet actions()}
    <!-- One button that changes its job, so the focus stays on it when a press of 停止 ends the run. -->
    <Button label={running === null ? "閉じる" : "停止"} onpress={running === null ? onclose : stop} />
    <Button label="入手" busyLabel="入手中…" variant="primary" busy={running !== null} disabledReason={blocker} onpress={start} />
  {/snippet}
</Modal>

<style lang="scss">
  .sections {
    display: grid;
    gap: var(--snz-space-md);
  }

  .fields {
    display: grid;
    justify-items: start;
    gap: var(--snz-space-sm);
  }

  .licence-body {
    display: grid;
    justify-items: start;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-sm) var(--snz-space-md);
  }

  .licence {
    justify-self: stretch;
    block-size: 14rem;
    padding: var(--snz-space-xs) var(--snz-space-sm);
    overflow-y: auto;
    font-size: var(--snz-font-size-small);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    background: var(--snz-surface-nested);
    border: var(--snz-border-line) solid var(--snz-line-divider);
    border-radius: var(--snz-radius-sm);

    &:focus-visible {
      outline: var(--snz-border-focus) solid var(--snz-focus);
      outline-offset: var(--snz-border-focus-offset);
    }
  }

  .running {
    margin: 0;
  }

  .hint {
    margin: 0;
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }
</style>
