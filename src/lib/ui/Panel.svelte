<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    heading: string;
    actions?: Snippet;
    /** Receives the heading's id, so a control in the body can take the heading as its name. */
    children: Snippet<[string]>;
  }

  let { heading, actions, children }: Props = $props();

  const headingId = $props.id();
</script>

<section class="panel" aria-labelledby={headingId}>
  <header class="head">
    <h2 id={headingId}>{heading}</h2>
    {#if actions}
      <div class="actions">{@render actions()}</div>
    {/if}
  </header>
  <div class="body">{@render children(headingId)}</div>
</section>

<style lang="scss">
  @use "../../styles/mixins" as *;

  // snz-design doc-9 §6.3, the non-collapsible form.
  .panel {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-inline-size: 0;
    min-block-size: 0;
    background: var(--snz-surface);
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-md);

    // A field that fills the body takes the panel's outline as its own, so its focus ring goes
    // round the panel (doc-8 §5.1: outside the outline). The heading's buttons draw their own.
    &:has(.body :global(:focus-visible)) {
      @include focus-ring;
    }
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-md);
    border-block-end: var(--snz-border-line) solid var(--snz-line-divider);
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: var(--snz-font-size-ground);
    font-weight: var(--snz-font-weight-semibold);
    line-height: var(--snz-font-line-height-heading);
    color: var(--snz-fg-strong);
  }

  .actions {
    display: flex;
    gap: var(--snz-space-xs);
  }

  .body {
    display: grid;
    min-block-size: 0;
    // The corners below the heading follow the panel's rounding, so a body that paints its own face
    // (a field on hover) does not poke out of it.
    border-end-start-radius: inherit;
    border-end-end-radius: inherit;
    overflow: hidden;
  }
</style>
