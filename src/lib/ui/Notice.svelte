<script lang="ts" module>
  /** snz-design doc-9 §6.4's levels this app uses: 失敗, 進行中の縮退 and 案内. */
  export type NoticeLevel = "failure" | "degraded" | "info";
</script>

<script lang="ts">
  import type { IconName } from "../icons/lucide";
  import Icon from "../icons/Icon.svelte";
  import IconButton from "./IconButton.svelte";

  interface Props {
    level: NoticeLevel;
    words: string;
    /** Given only for 案内, the one level that may be closed (doc-9 §6.4); the caller stops drawing it. */
    ondismiss?: () => void;
  }

  let { level, words, ondismiss }: Props = $props();

  // doc-9 §6.4: each level has a figure of its own, so the levels are not told apart by colour alone
  // (degraded and a warning share the warning colour).
  const FIGURES: Record<NoticeLevel, { icon: IconName; name: string }> = {
    failure: { icon: "circle-x", name: "失敗" },
    degraded: { icon: "ban", name: "使えない機能があります" },
    info: { icon: "info", name: "お知らせ" },
  };

  let figure = $derived(FIGURES[level]);
</script>

<!-- Only a failure interrupts the reading (role="alert"); the rest wait their turn. The caller announces
     a notice that appears after the screen is up, since a status that is there from the start is read
     with the rest of it. -->
<div class="notice" data-level={level} role={level === "failure" ? "alert" : "status"}>
  <span class="figure" role="img" aria-label={figure.name}><Icon name={figure.icon} /></span>
  <p class="words">{words}</p>
  {#if level === "info" && ondismiss}
    <span class="close"><IconButton icon="x" label="お知らせを閉じる" onpress={ondismiss} /></span>
  {/if}
</div>

<style lang="scss">
  .notice {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-sm);
    // doc-9 §6.4's band: 3 CSS px at the start, in the level's colour.
    border-inline-start: 3px solid var(--band);
    border-radius: var(--snz-radius-sm);
    color: var(--words);
    background: var(--face);

    &[data-level="failure"] {
      --band: var(--snz-danger);
      --face: var(--snz-danger-soft);
      --words: var(--snz-on-danger-soft);
    }

    // Solarized Dark's warning colour is not put on the words (doc-9 §6.4); they take on-warn-soft.
    &[data-level="degraded"] {
      --band: var(--snz-warn);
      --face: var(--snz-warn-soft);
      --words: var(--snz-on-warn-soft);
    }

    &[data-level="info"] {
      --band: var(--snz-line-control);
      --face: var(--snz-surface-alt);
      --words: var(--snz-fg-muted);
    }
  }

  .figure {
    --icon-color: var(--band);
  }

  .words {
    grid-column: 2;
    margin: 0;
    padding-block: var(--snz-space-xs);
    overflow-wrap: anywhere;
  }

  .close {
    --icon-color: var(--words);
  }
</style>
