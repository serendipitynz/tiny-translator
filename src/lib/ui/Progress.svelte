<script lang="ts">
  import { announce } from "./announce";

  // snz-design doc-8 §6.7.1's 量が分かる進捗 only: everything this app waits on with a bar (a model
  // fetch and its check) knows its total from the start.
  interface Props {
    /** What is being waited for. */
    label: string;
    total: number;
    done: number;
    /** The amount in words beside the label; the share done when not given. */
    readout?: string;
  }

  let { label, total, done, readout }: Props = $props();

  let share = $derived(total > 0 ? Math.min(1, Math.max(0, done / total)) : 0);
  let words = $derived(readout ?? `${Math.floor(share * 100)}%`);

  // doc-8 §6.7.1: told when it starts and when what is waited for changes, never at every step, so
  // the amount does not drown out everything else; it is there to be asked for (aria-valuetext).
  $effect(() => {
    announce(label);
  });
</script>

<div class="progress">
  <p class="words">
    <span>{label}</span>
    <span class="readout">{words}</span>
  </p>
  <div
    class="bar"
    role="progressbar"
    aria-label={label}
    aria-valuemin={0}
    aria-valuemax={total}
    aria-valuenow={Math.min(done, total)}
    aria-valuetext={words}
  >
    <div class="fill" style:inline-size={`${share * 100}%`}></div>
  </div>
</div>

<style lang="scss">
  .progress {
    display: grid;
    gap: var(--snz-space-xs);
  }

  .words {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: var(--snz-space-xs) var(--snz-space-md);
    margin: 0;
    color: var(--snz-fg);
  }

  .readout {
    color: var(--snz-fg-muted);
    font-variant-numeric: tabular-nums;
  }

  // doc-8 §6.7.1's token use: the outline shows where the bar is even while the fill is 0.
  .bar {
    block-size: 0.5rem;
    overflow: hidden;
    background: var(--snz-surface-alt);
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-sm);
  }

  .fill {
    block-size: 100%;
    background: var(--snz-accent);
  }
</style>
