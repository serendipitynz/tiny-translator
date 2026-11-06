<script lang="ts">
  // Always aria-hidden: an icon never stands alone (snz-design doc-8 §6.6), so the words a screen
  // reader announces belong on the control that holds it, e.g. an icon-only button's aria-label.
  import { ICONS, ICON_SVG_ATTRS, type IconName } from "./lucide";

  interface Props {
    name: IconName;
    size?: "sm" | "md";
  }

  let { name, size = "md" }: Props = $props();
</script>

<svg {...ICON_SVG_ATTRS} class={size} aria-hidden="true">
  {#each ICONS[name] as [tag, attrs]}
    <svelte:element this={tag} {...attrs} />
  {/each}
</svg>

<style lang="scss">
  svg {
    // Block so an inline SVG's line box cannot add height the holder did not ask for.
    display: block;
    flex: none;
    // doc-8 §6.6: --snz-figure by default; a holder with a face of its own sets --icon-color to the
    // colour that sits on that face (--snz-on-accent on a primary button) and swaps it with the face.
    // A custom property rather than `color` from the holder, because it inherits past this
    // component's scoped styles without the holder reaching in with :global.
    color: var(--icon-color, var(--snz-figure));
  }

  .sm {
    width: var(--snz-icon-size-sm);
    height: var(--snz-icon-size-sm);
  }

  .md {
    width: var(--snz-icon-size-md);
    height: var(--snz-icon-size-md);
  }
</style>
