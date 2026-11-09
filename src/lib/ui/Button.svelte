<script lang="ts">
  import { announce } from "./announce";

  interface Props {
    label: string;
    /** The words shown while busy. They share a cell with `label`, so the width never changes. */
    busyLabel?: string;
    /** doc-8 §6.1's 主操作, 通常 and 破壊的操作 (for what cannot be undone). */
    variant?: "primary" | "normal" | "danger";
    /** Disabled exactly when a reason is given: a disabled control without one is not made (doc-8 §5.4). */
    disabledReason?: string | null;
    busy?: boolean;
    /** Where a modal puts the focus when it opens (`Modal.svelte`). */
    autofocus?: boolean;
    onpress: () => void;
  }

  let {
    label,
    busyLabel = label,
    variant = "normal",
    disabledReason = null,
    busy = false,
    autofocus = false,
    onpress,
  }: Props = $props();

  const reasonId = $props.id();
  let disabled = $derived(disabledReason !== null);

  // The spinner reaches only the eyes, so what is being waited for is also spoken (doc-8 §6.7).
  let wasBusy = false;
  $effect(() => {
    if (busy && !wasBusy) {
      announce(busyLabel);
    }
    wasBusy = busy;
  });

  function press() {
    if (disabledReason !== null) {
      announce(disabledReason);
      return;
    }
    if (busy) {
      return;
    }
    onpress();
  }
</script>

<!-- aria-disabled rather than disabled: the button keeps the focus, so the reason reaches the
     keyboard (doc-8 §5.4). -->
<button
  type="button"
  class="button {variant}"
  aria-disabled={disabled ? "true" : undefined}
  aria-describedby={disabled ? reasonId : undefined}
  aria-busy={busy ? "true" : undefined}
  title={disabledReason ?? undefined}
  data-autofocus={autofocus ? "" : undefined}
  onclick={press}
>
  <span class="words" class:hidden={busy}>{label}</span>
  <span class="words" class:hidden={!busy}><span class="spinner"></span>{busyLabel}</span>
</button>
{#if disabled}
  <span id={reasonId} class="visually-hidden">{disabledReason}</span>
{/if}

<style lang="scss">
  @use "../../styles/mixins" as *;

  .button {
    display: inline-grid;
    min-block-size: var(--snz-size-control);
    padding: 0 var(--snz-space-lg);
    font: inherit;
    font-weight: var(--snz-font-weight-semibold);
    border: var(--snz-border-line) solid;
    border-radius: var(--snz-radius-md);
    cursor: pointer;
    transition:
      background-color var(--snz-motion-state),
      border-color var(--snz-motion-state);

    &:focus-visible {
      @include focus-ring;
    }

    &[aria-busy="true"] {
      cursor: progress;
    }
  }

  // The resting and the busy words sit in one grid cell, so the button is as wide as the longer of
  // the two in both states; the hidden one keeps its room (doc-8 §6.1, doc-1 §8.2).
  .words {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--snz-space-xs);
  }

  .hidden {
    visibility: hidden;
  }

  // doc-8 §5.3: hover and press move the accent face itself, and the words on it are restated with
  // each face rather than left over from the resting one.
  .primary {
    color: var(--snz-on-accent);
    background: var(--snz-accent);
    border-color: var(--snz-accent);

    &:hover {
      color: var(--snz-on-accent);
      background: var(--snz-accent-hover);
      border-color: var(--snz-accent-hover);
    }

    &:active {
      color: var(--snz-on-accent);
      background: var(--snz-accent-pressed);
      border-color: var(--snz-accent-pressed);
    }
  }

  .normal,
  .danger {
    color: var(--snz-fg);
    background: var(--snz-surface);
    border-color: var(--snz-line-control);

    &:hover {
      background: var(--snz-surface-hover);
    }

    &:active {
      background: var(--snz-surface-pressed);
    }
  }

  // doc-8 §6.1: only the words and the outline take the danger colour. A danger face would read as
  // the failure messages do, and the operation would look like a report.
  .danger {
    color: var(--snz-danger);
    border-color: var(--snz-danger);
  }

  // Busy and disabled take no hover or press: the button does not act in either state.
  .primary[aria-busy="true"],
  .primary[aria-disabled="true"] {
    &,
    &:hover,
    &:active {
      background: var(--snz-accent);
      border-color: var(--snz-accent);
    }
  }

  .normal[aria-busy="true"],
  .normal[aria-disabled="true"],
  .danger[aria-busy="true"],
  .danger[aria-disabled="true"] {
    &,
    &:hover,
    &:active {
      background: var(--snz-surface);
    }
  }

  .button[aria-disabled="true"] {
    @include disabled-control;
  }

  // doc-8 §6.7, drawn as the snz-design Storybook draws it: an arc of the figure colour, which on the
  // primary face is the colour that sits on the accent (§5.3).
  .spinner {
    box-sizing: border-box;
    inline-size: var(--snz-icon-size-sm);
    block-size: var(--snz-icon-size-sm);
    border: var(--snz-border-focus) solid var(--snz-figure);
    border-inline-end-color: transparent;
    border-radius: 50%;
    animation: spin var(--snz-motion-spin) linear infinite;
  }

  .danger .spinner {
    border-color: var(--snz-danger);
    border-inline-end-color: transparent;
  }

  .primary .spinner {
    border-color: var(--snz-on-accent);
    border-inline-end-color: transparent;
  }

  // Slowed, never stopped: a stopped spinner cannot be told from a stalled one (doc-8 §6.7).
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: var(--snz-motion-spin-reduced);
    }
  }

  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }
</style>
