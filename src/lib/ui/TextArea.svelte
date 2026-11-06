<script lang="ts">
  interface Props {
    value: string;
    /** The id of the visible words that name the field (doc-8 §6.3's name area). */
    labelledby: string;
    placeholder: string;
    readonly?: boolean;
  }

  let { value = $bindable(), labelledby, placeholder, readonly = false }: Props = $props();
</script>

<!-- The multi-line field of doc-8 §6.3, filling a panel body: the panel's outline is its outline.
     The window is what the user resizes, so the field has no resize handle of its own. -->
<textarea bind:value aria-labelledby={labelledby} {placeholder} {readonly} spellcheck="false"></textarea>

<style lang="scss">
  textarea {
    box-sizing: border-box;
    inline-size: 100%;
    block-size: 100%;
    min-block-size: calc(var(--snz-size-control) * 2);
    margin: 0;
    padding: var(--snz-space-sm) var(--snz-space-md);
    font: inherit;
    color: var(--snz-fg);
    background: var(--snz-surface);
    border: 0;
    outline: none;
    resize: none;
    transition: background-color var(--snz-motion-state);

    // Words that stand for an absent value are faint but readable (doc-8 §5.5).
    &::placeholder {
      color: var(--snz-fg-faint);
      opacity: 1;
    }

    // Hover says "this takes input", which a read-only field does not.
    &:not([readonly]):hover {
      background: var(--snz-surface-hover);
    }
  }
</style>
