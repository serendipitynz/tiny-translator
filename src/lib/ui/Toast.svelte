<script lang="ts">
  import Icon from "../icons/Icon.svelte";
  import { announce } from "./announce";
  import IconButton from "./IconButton.svelte";
  import { toastDuration, tokenMs } from "./toast";

  // snz-design doc-9 §6.5. One toast at a time: a newer one replaces the one still up, so pressing
  // copy twice does not stack two toasts saying the same thing.
  let toast = $state<{ id: number; body: string; leaving: boolean } | null>(null);
  let count = 0;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let remaining = 0;
  let startedAt = 0;
  // Kept apart (doc-9 §6.5): folded into one, moving the pointer away while focus is on the close
  // button would restart the timer and take the button away from under the focus.
  let hovered = false;
  let focused = false;
  /** Where the focus was before it entered the toast, to return it there when the toast is closed. */
  let before: HTMLElement | null = null;

  const bodyId = $props.id();

  /** Shows `body` as the result of the operation just done. Takes no focus; the words are read politely. */
  export function show(body: string) {
    stopTimer();
    hovered = false;
    focused = false;
    before = null;
    toast = { id: ++count, body, leaving: false };
    remaining = toastDuration(body, tokenMs("--snz-motion-toast-min", 1800), tokenMs("--snz-motion-toast-max", 6000));
    announce(body);
    run();
  }

  function stopTimer() {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function run() {
    if (toast === null || toast.leaving) {
      return;
    }
    startedAt = performance.now();
    timer = setTimeout(dismiss, remaining);
  }

  function pause() {
    if (timer === null) {
      return;
    }
    stopTimer();
    remaining -= performance.now() - startedAt;
  }

  function resumeIfFree() {
    if (!hovered && !focused) {
      run();
    }
  }

  // Terminal: the focus returned on close leaves the toast, and that must not start the timer again.
  function dismiss() {
    if (toast === null || toast.leaving) {
      return;
    }
    stopTimer();
    toast.leaving = true;
    const id = toast.id;
    // Removed after the fade rather than on transitionend, which never fires when there is no transition.
    setTimeout(
      () => {
        if (toast?.id === id) {
          toast = null;
        }
      },
      tokenMs("--snz-motion-state", 140) + 100,
    );
  }

  function close() {
    const back = before;
    dismiss();
    (back?.isConnected ? back : document.body).focus();
  }

  function onfocusin(event: FocusEvent) {
    if (!focused) {
      before = event.relatedTarget instanceof HTMLElement ? event.relatedTarget : null;
    }
    focused = true;
    pause();
  }

  function onfocusout(event: FocusEvent) {
    if (event.currentTarget instanceof Node && event.currentTarget.contains(event.relatedTarget as Node | null)) {
      return;
    }
    focused = false;
    resumeIfFree();
  }
</script>

<div class="region">
  {#if toast !== null}
    {#key toast.id}
      <div
        class="toast"
        role="group"
        aria-labelledby={bodyId}
        data-phase={toast.leaving ? "leaving" : "shown"}
        onpointerenter={() => {
          hovered = true;
          pause();
        }}
        onpointerleave={() => {
          hovered = false;
          resumeIfFree();
        }}
        {onfocusin}
        {onfocusout}
      >
        <span class="figure"><Icon name="circle-check" /></span>
        <p id={bodyId} class="body">{toast.body}</p>
        <span class="close"><IconButton icon="x" label="通知を閉じる" onpress={close} /></span>
      </div>
    {/key}
  {/if}
</div>

<style lang="scss">
  // At the start of the bottom edge: the end holds the translate button, which a toast must not cover.
  .region {
    position: fixed;
    inset-block-end: var(--snz-space-md);
    inset-inline-start: var(--snz-space-md);
    z-index: 900;
    max-inline-size: min(24rem, calc(100vw - 2 * var(--snz-space-md)));
  }

  .toast {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: center;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-md);
    color: var(--snz-on-success-soft);
    background: var(--snz-success-soft);
    // It lies over the page, so its edge against the face below is drawn (doc-9 §6.5).
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-md);
    box-shadow: var(--snz-shadow-raised);
    transition:
      opacity var(--snz-motion-state),
      translate var(--snz-motion-state);

    @starting-style {
      opacity: 0;
      translate: 0 0.5rem;
    }

    &[data-phase="leaving"] {
      opacity: 0;
      translate: 0 0.5rem;
    }

    // doc-9 §6.5: by opacity only, but it still goes away.
    @media (prefers-reduced-motion: reduce) {
      transition: opacity var(--snz-motion-state);
      translate: none;

      @starting-style {
        translate: none;
      }

      &[data-phase="leaving"] {
        translate: none;
      }
    }
  }

  .figure {
    --icon-color: var(--snz-success);
  }

  .body {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .close {
    --icon-color: var(--snz-on-success-soft);
  }
</style>
