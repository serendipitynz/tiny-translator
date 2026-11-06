let region: HTMLElement | null = null;

function liveRegion(): HTMLElement {
  if (region === null) {
    region = document.createElement("div");
    region.className = "visually-hidden";
    region.setAttribute("aria-live", "polite");
    document.body.append(region);
  }
  return region;
}

/**
 * Reads `words` to a screen reader without moving the focus (snz-design doc-8 §5.8). Saying the same
 * words twice in a row is announced twice.
 */
export function announce(words: string): void {
  const target = liveRegion();
  // A live region only speaks when its text changes, so it is emptied first and refilled a moment
  // later; otherwise pressing the same disabled button twice would be heard once. A timer rather
  // than requestAnimationFrame, which never fires while the window is hidden.
  target.textContent = "";
  setTimeout(() => {
    target.textContent = words;
  }, 50);
}
