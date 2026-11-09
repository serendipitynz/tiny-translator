<script lang="ts">
  interface Props {
    label: string;
    value: string;
    placeholder?: string;
    /** Where a modal puts the focus when it opens (`Modal.svelte`). */
    autofocus?: boolean;
  }

  let { label, value = $bindable(), placeholder, autofocus = false }: Props = $props();

  let input: HTMLInputElement;

  /** Moves the keyboard here, for a caller whose own control has just gone away. */
  export function focus() {
    input.focus();
  }

  // doc-8 §6.3: nothing is changed while an IME composes; the composed text arrives whole when it is
  // committed. Otherwise every half-converted reading would be searched for.
  function edited(event: Event & { currentTarget: HTMLInputElement }) {
    if (event instanceof InputEvent && event.isComposing) {
      return;
    }
    value = event.currentTarget.value;
  }

  function composed(event: CompositionEvent & { currentTarget: HTMLInputElement }) {
    value = event.currentTarget.value;
  }
</script>

<!-- The single-line field of doc-8 §6.3, laid out as Select is: the name beside its frame. type="text"
     rather than "search": WebKit's search field empties itself on Escape, which here closes the
     dialog instead (snz-design doc-9 §6.1 does not lift a filter with Escape). -->
<label class="field">
  <span class="name">{label}</span>
  <input
    bind:this={input}
    type="text"
    {value}
    {placeholder}
    data-autofocus={autofocus ? "" : undefined}
    enterkeyhint="search"
    autocomplete="off"
    spellcheck="false"
    oninput={edited}
    oncompositionend={composed}
  />
</label>

<style lang="scss">
  @use "../../styles/mixins" as *;

  .field {
    display: flex;
    align-items: center;
    gap: var(--snz-space-sm);
    min-inline-size: 0;
  }

  .name {
    flex: none;
    color: var(--snz-fg);
  }

  input {
    box-sizing: border-box;
    flex: 1;
    min-inline-size: 0;
    min-block-size: var(--snz-size-control);
    margin: 0;
    padding: 0 var(--snz-space-sm);
    font: inherit;
    color: var(--snz-fg);
    background: var(--snz-surface);
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-sm);
    outline: none;
    transition: background-color var(--snz-motion-state);

    &::placeholder {
      color: var(--snz-fg-faint);
      opacity: 1;
    }

    &:hover {
      background: var(--snz-surface-hover);
    }

    &:focus-visible {
      @include focus-ring;
    }
  }
</style>
