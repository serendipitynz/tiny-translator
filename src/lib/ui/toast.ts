// The per-character rate is not in snz-design doc-9 §6.5, which fixes only the bounds; this is the
// rate its Storybook example uses, so a toast here stays up as long as the reference one.
const MS_PER_CHAR = 90;

/** How long a toast saying `body` stays up: from the length of its words, within `minMs`–`maxMs` (doc-9 §6.5). */
export function toastDuration(body: string, minMs: number, maxMs: number): number {
  return Math.min(maxMs, Math.max(minMs, [...body].length * MS_PER_CHAR));
}

/** A `--snz-motion-*` token in milliseconds, or `fallback` when it is missing or not in `ms`. */
export function tokenMs(name: string, fallback: number): number {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const ms = value.endsWith("ms") ? Number.parseFloat(value) : Number.NaN;
  return Number.isFinite(ms) ? ms : fallback;
}
