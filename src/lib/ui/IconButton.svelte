<script lang="ts">
  import Icon from "../icons/Icon.svelte";
  import type { IconName } from "../icons/lucide";
  import { announce } from "./announce";

  interface Props {
    icon: IconName;
    /** The words a screen reader announces; required because the button shows none (doc-8 §6.2). */
    label: string;
    /** Disabled exactly when a reason is given (doc-8 §5.4). */
    disabledReason?: string | null;
    onpress: () => void;
  }

  let { icon, label, disabledReason = null, onpress }: Props = $props();

  const reasonId = $props.id();
  let disabled = $derived(disabledReason !== null);

  function press() {
    if (disabledReason !== null) {
      announce(disabledReason);
      return;
    }
    onpress();
  }
</script>

<!-- The quiet variant of doc-8 §6.1: no face or outline at rest, so a row of these reads as tools
     rather than as a row of buttons competing with the primary one. -->
<button
  type="button"
  class="icon-button"
  aria-label={label}
  aria-disabled={disabled ? "true" : undefined}
  aria-describedby={disabled ? reasonId : undefined}
  title={disabledReason ?? label}
  onclick={press}
>
  <Icon name={icon} />
</button>
{#if disabled}
  <span id={reasonId} class="visually-hidden">{disabledReason}</span>
{/if}

<style lang="scss">
  @use "../../styles/mixins" as *;

  .icon-button {
    display: inline-grid;
    place-items: center;
    // The press area is sized apart from the figure (doc-8 §6.2): a 20px figure in a control-high square.
    inline-size: var(--snz-size-control);
    block-size: var(--snz-size-control);
    padding: 0;
    background: none;
    border: var(--snz-border-line) solid transparent;
    border-radius: var(--snz-radius-md);
    cursor: pointer;
    transition: background-color var(--snz-motion-state);

    &:hover {
      background: var(--snz-surface-hover);
    }

    &:active {
      background: var(--snz-surface-pressed);
    }

    &:focus-visible {
      @include focus-ring;
    }

    // With no outline at rest there is nothing to break into dashes, so the disabled state draws one.
    &[aria-disabled="true"] {
      @include disabled-control;

      border-color: var(--snz-line-control);

      &:hover,
      &:active {
        background: none;
      }
    }
  }
</style>
