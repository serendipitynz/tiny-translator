<script lang="ts" module>
  export interface SelectOption {
    value: string;
    label: string;
  }
</script>

<script lang="ts">
  import Icon from "../icons/Icon.svelte";

  interface Props {
    label: string;
    options: readonly SelectOption[];
    value: string;
  }

  let { label, options, value = $bindable() }: Props = $props();

  // Named by the words alone: a label that wraps the select would also fold its current value
  // into the name.
  const nameId = $props.id();
</script>

<!-- WKWebView paints a select's own outline and face with the platform's look whatever the author
     sets (snz-design doc-8 §6.3), so the outline, face and chevron are drawn by the frame and the
     select inside it is made transparent. The select itself stays native, so its list, keyboard
     and reading are the platform's. -->
<label class="field">
  <span class="name" id={nameId}>{label}</span>
  <span class="frame">
    <select bind:value aria-labelledby={nameId}>
      {#each options as option (option.value)}
        <option value={option.value}>{option.label}</option>
      {/each}
    </select>
    <span class="chevron"><Icon name="chevron-down" /></span>
  </span>
</label>

<style lang="scss">
  @use "../../styles/mixins" as *;

  .field {
    display: inline-flex;
    align-items: center;
    gap: var(--snz-space-sm);
    min-inline-size: 0;
  }

  .name {
    flex: none;
    color: var(--snz-fg);
  }

  .frame {
    display: grid;
    min-inline-size: 0;
    background: var(--snz-surface);
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-sm);
    transition: background-color var(--snz-motion-state);

    &:hover {
      background: var(--snz-surface-hover);
    }

    &:has(select:focus-visible) {
      @include focus-ring;
    }
  }

  select {
    grid-area: 1 / 1;
    min-inline-size: 0;
    min-block-size: calc(var(--snz-size-control) - 2 * var(--snz-border-line));
    padding: 0 calc(var(--snz-icon-size-md) + 2 * var(--snz-space-sm)) 0 var(--snz-space-sm);
    font: inherit;
    color: var(--snz-fg);
    background: transparent;
    border: 0;
    outline: none;
    appearance: none;
    cursor: pointer;
  }

  .chevron {
    grid-area: 1 / 1;
    align-self: center;
    justify-self: end;
    margin-inline-end: var(--snz-space-sm);
    pointer-events: none;
  }
</style>
