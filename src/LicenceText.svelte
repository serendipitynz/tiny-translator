<script lang="ts">
  import { LICENCE, LICENCE_LANGUAGE_OPTIONS, type LicenceLanguage } from "./lib/models";
  import Select from "./lib/ui/Select.svelte";

  interface Props {
    /** The heading that names the text, read as the scrollable region's name. */
    labelledby: string;
  }

  let { labelledby }: Props = $props();

  let language = $state<string>("ja");
  let text = $derived(LICENCE.texts[language as LicenceLanguage] ?? LICENCE.texts.ja);
</script>

<div class="body">
  <Select label="言語" options={LICENCE_LANGUAGE_OPTIONS} bind:value={language} />
  <!-- Focusable so the text can be scrolled from the keyboard. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="text" role="region" aria-labelledby={labelledby} tabindex="0">{text}</div>
  <p class="hint">{LICENCE.revised}。入手できるモデルと同じコミットにある条文です。</p>
</div>

<style lang="scss">
  .body {
    display: grid;
    justify-items: start;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-sm) var(--snz-space-md);
  }

  .text {
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

  .hint {
    margin: 0;
    font-size: var(--snz-font-size-small);
    color: var(--snz-fg-muted);
  }
</style>
