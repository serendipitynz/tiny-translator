/**
 * Whether nothing holds the focus any more — what is left when the focused control is removed from
 * the page (snz-design doc-9 §5.1). A caller moves the focus on only then: anywhere else, the user
 * or a newer layer has put it there since.
 */
export function focusWasLost(active: Element | null, body: Element): boolean {
  return active === null || active === body;
}
