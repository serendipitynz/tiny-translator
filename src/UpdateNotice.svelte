<script lang="ts">
  import { sizeLabel } from "./lib/settings";
  import { announce } from "./lib/ui/announce";
  import Button from "./lib/ui/Button.svelte";
  import Notice from "./lib/ui/Notice.svelte";
  import Progress from "./lib/ui/Progress.svelte";
  import { dismissUpdate, installUpdate, type UpdateState, updateWords } from "./lib/update";

  // doc-1 §14: where a newer version is offered, followed through its download to the restart. The
  // same state is drawn on the main screen and in the settings, so 今すぐ確認 and the check at launch
  // end in the same place.
  interface Props {
    state: UpdateState;
  }

  let { state }: Props = $props();

  let words = $derived(updateWords(state));

  // Told when the offer appears and when it fails, not at every step of the download: the
  // notice comes up after the screen, so nothing reads it otherwise (Notice.svelte).
  let told: string | null = null;
  $effect(() => {
    const key = state.phase === "downloading" ? null : words;
    if (key !== null && key !== told) {
      announce(key);
    }
    told = key;
  });

  function install() {
    // A failure comes back as the `failed` state, which this notice then shows.
    installUpdate().catch(() => {});
  }

  function dismiss() {
    void dismissUpdate();
  }
</script>

{#if state.phase === "available" && words !== null}
  <div class="update">
    <Notice level="info" {words} ondismiss={dismiss} />
    <Button label="更新して再起動" variant="primary" onpress={install} />
  </div>
{:else if state.phase === "downloading" && words !== null}
  {#if state.total !== null && state.total > 0}
    <Progress label={words} total={state.total} done={state.received} />
  {:else}
    <Notice level="info" words={`${words} (${sizeLabel(state.received)})`} />
  {/if}
{:else if state.phase === "installing" && words !== null}
  <Notice level="info" {words} />
{:else if state.phase === "failed" && words !== null}
  <div class="update">
    <Notice level="failure" {words} />
    <Button label="閉じる" onpress={dismiss} />
    <Button label="もう一度試す" variant="primary" onpress={install} />
  </div>
{/if}

<style lang="scss">
  .update {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--snz-space-sm);

    > :global(.notice) {
      flex: 1 1 20rem;
    }
  }
</style>
