<script lang="ts">
  import type { Snippet } from "svelte";
  import IconButton from "./IconButton.svelte";

  interface Props {
    heading: string;
    /**
     * Why the modal cannot be closed right now (while what it started is still running), or null when
     * it can. While there is a reason, Escape and the scrim do nothing and × says why (doc-9 §6.6).
     */
    closeBlocker?: string | null;
    /** Asked for by ×, Escape and the scrim. The caller closes the modal by no longer drawing it. */
    onclose: () => void;
    children: Snippet;
    /** doc-9 §6.6's 操作域, laid out at the end: cancel first, then the operation that runs. */
    actions: Snippet;
  }

  let { heading, closeBlocker = null, onclose, children, actions }: Props = $props();

  const headingId = $props.id();

  // snz-design doc-9 §5.1: where the focus was when the modal opened, read while the component is
  // created — still inside the press that opened it. Read later, the modal would already hold it.
  const opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;

  let panel: HTMLDivElement;

  // `aria-disabled` controls stay in: they keep the focus so their reason can be read (doc-8 §5.4).
  const FOCUSABLE = [
    "a[href]",
    "button:not([disabled])",
    "input:not([disabled])",
    "select:not([disabled])",
    "textarea:not([disabled])",
    '[tabindex]:not([tabindex="-1"])',
  ].join(", ");

  function focusable(): HTMLElement[] {
    return [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
      (element) => element.getClientRects().length > 0,
    );
  }

  $effect(() => {
    // doc-9 §6.6: the focus goes where the caller marked — the first field of a form, the cancel of a
    // 破壊前確認 — and never to the operation that runs. Without a mark, to the panel itself.
    const marked = panel.querySelector<HTMLElement>("[data-autofocus]");
    (marked ?? panel).focus();
    return () => {
      // doc-9 §5.1: back where it was. A control that went while the modal was open leaves the focus
      // on the page, which is all that remains to give it to.
      if (opener?.isConnected) {
        opener.focus();
      }
    };
  });

  function close() {
    if (closeBlocker === null) {
      onclose();
    }
  }

  function keydown(event: KeyboardEvent) {
    // Every key stops here: the screen behind takes no operation while a modal is up (doc-9 §6.6),
    // its shortcuts included, and a modal opened over this one answers its own keys first (§5.2).
    event.stopPropagation();
    // An IME's Escape cancels the conversion (doc-8 §5.6); WebKit reports it as keyCode 229.
    if (event.isComposing || event.keyCode === 229) {
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== "Tab") {
      return;
    }
    // doc-9 §5.3: the focus stays inside; Tab and Shift+Tab go round.
    event.preventDefault();
    const items = focusable();
    if (items.length === 0) {
      panel.focus();
      return;
    }
    const at = items.indexOf(document.activeElement as HTMLElement);
    const next = event.shiftKey ? (at <= 0 ? items.length - 1 : at - 1) : (at + 1) % items.length;
    items[next].focus();
  }
</script>

<div class="layer">
  <!-- The scrim takes the pointer for everything behind it; pressing it closes the modal (doc-9 §6.6).
       Escape and × do the same from the keyboard, so the scrim itself needs no key of its own. -->
  <div class="scrim" aria-hidden="true" onclick={close}></div>
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby={headingId}
    tabindex="-1"
    bind:this={panel}
    onkeydown={keydown}
  >
    <header class="head">
      <h2 id={headingId}>{heading}</h2>
      <IconButton icon="x" label="閉じる" disabledReason={closeBlocker} onpress={close} />
    </header>
    <div class="body">{@render children()}</div>
    <footer class="actions">{@render actions()}</footer>
  </div>
</div>

<style lang="scss">
  @use "../../styles/mixins" as *;

  // Under the toasts (z-index 900), which report what an operation in the modal did.
  .layer {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    // One track the size of the window less the padding: an auto track grows with the panel, and the
    // panel's max-block-size would then never stop it at the window's height (doc-9 §6.6).
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
    place-items: center;
    padding: var(--snz-space-md);
  }

  // doc-9 §6.6: the ground's own colour made translucent, so no new colour is made for it.
  .scrim {
    position: absolute;
    inset: 0;
    background: var(--snz-canvas);
    opacity: var(--snz-opacity-scrim);
  }

  // Centred by the layer's grid, not by a transform: a transformed panel would become the box a
  // fixed-position descendant is placed in.
  .panel {
    position: relative;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    box-sizing: border-box;
    inline-size: min(48rem, 100%);
    max-block-size: 100%;
    color: var(--snz-fg);
    background: var(--snz-surface);
    border: var(--snz-border-line) solid var(--snz-line-control);
    border-radius: var(--snz-radius-md);
    box-shadow: var(--snz-shadow-modal);

    &:focus-visible {
      @include focus-ring;
    }
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-xs) var(--snz-space-md);
    border-block-end: var(--snz-border-line) solid var(--snz-line-divider);
  }

  h2 {
    flex: 1;
    margin: 0;
    font-size: var(--snz-font-size-title);
    font-weight: var(--snz-font-weight-bold);
    line-height: var(--snz-font-line-height-heading);
    color: var(--snz-fg-strong);
  }

  // doc-9 §6.6: only the body scrolls, so the heading and the actions stay in reach on a short window.
  .body {
    min-block-size: 0;
    padding: var(--snz-space-md);
    overflow: auto;
  }

  // doc-9 §6.6: at the end, in document order cancel → run, and still at the end when they wrap.
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--snz-space-sm);
    padding: var(--snz-space-sm) var(--snz-space-md);
    border-block-start: var(--snz-border-line) solid var(--snz-line-divider);
  }
</style>
