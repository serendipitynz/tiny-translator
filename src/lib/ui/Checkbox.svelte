<script lang="ts">
  import Icon from "../icons/Icon.svelte";

  interface Props {
    label: string;
    checked: boolean;
    /** doc-8 §6.4's 説明域, read with the box as its description. */
    description?: string;
  }

  let { label, checked = $bindable(), description }: Props = $props();

  const descriptionId = $props.id();
</script>

<!-- snz-design doc-8 §6.4: the label is part of what can be pressed, and the native input keeps the
     keyboard (Space) and the reading. The box is drawn over it, since WKWebView paints a checkbox's
     own outline whatever the author sets (as it does a select's, doc-8 §6.3). -->
<div class="choice">
  <label class="press">
    <span class="mark">
      <input type="checkbox" bind:checked aria-describedby={description ? descriptionId : undefined} />
      <!-- On or off is told by the check's shape, not the face's colour alone (1.4.1). -->
      <span class="box" aria-hidden="true">
        {#if checked}<Icon name="check" size="sm" />{/if}
      </span>
    </span>
    <span class="label">{label}</span>
  </label>
  {#if description}
    <p class="description" id={descriptionId}>{description}</p>
  {/if}
</div>

<style lang="scss">
  @use "../../styles/mixins" as *;

  .press {
    display: inline-flex;
    align-items: center;
    gap: var(--snz-space-sm);
    min-block-size: var(--snz-size-target-min);
    color: var(--snz-fg);
    cursor: pointer;
  }

  .mark {
    position: relative;
    display: grid;
    flex: none;
  }

  // The input stays in place, transparent, so the pointer and the focus land on it.
  input {
    grid-area: 1 / 1;
    inline-size: 1.125rem;
    block-size: 1.125rem;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .box {
    grid-area: 1 / 1;
    display: grid;
    place-items: center;
    box-sizing: border-box;
    inline-size: 1.125rem;
    block-size: 1.125rem;
    background: var(--snz-surface);
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-sm);
    pointer-events: none;
  }

  input:checked + .box {
    --icon-color: var(--snz-on-accent);

    background: var(--snz-accent);
    border-color: var(--snz-accent);
  }

  input:focus-visible + .box {
    @include focus-ring;
  }

  .description {
    margin: var(--snz-space-xs) 0 0;
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }
</style>
