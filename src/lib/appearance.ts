import type { SelectOption } from "./ui/Select.svelte";

// The two colour axes (snz-design doc-7 §4.1, doc-1 §8.3), under the keys and values the script in
// index.html reads before the first paint. Both families carry a light and a dark side, so every
// pair is recorded and the mode's options never narrow (doc-7 §4.2).

export const FAMILY_OPTIONS: readonly SelectOption[] = [
  { value: "standard", label: "標準" },
  { value: "solarized", label: "Solarized" },
];

export const MODE_OPTIONS: readonly SelectOption[] = [
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "auto", label: "OS 追従" },
];

export interface Appearance {
  family: string;
  mode: string;
}

/** doc-7 §4.4. */
export const INITIAL_APPEARANCE: Appearance = { family: "standard", mode: "auto" };

const FAMILY_KEY = "colorFamily";
const MODE_KEY = "colorMode";

/**
 * The stored choice as the screen draws it, and how the storage stood: `unrecorded` when it holds a
 * value this version does not have (doc-7 §5.1), `unreadable` when it threw (§5.2). Both draw the
 * initial choice, as index.html does.
 */
export interface StoredAppearance {
  appearance: Appearance;
  state: "read" | "unrecorded" | "unreadable";
  /** The stored values, for saying which one this version lacks. */
  stored: { family: string | null; mode: string | null };
}

const has = (options: readonly SelectOption[], value: string) => options.some((option) => option.value === value);

/** Reads the choice the way index.html does: a missing key is that axis's initial value (doc-7 §4.4). */
export function readAppearance(storage: Pick<Storage, "getItem">): StoredAppearance {
  let family: string | null;
  let mode: string | null;
  try {
    family = storage.getItem(FAMILY_KEY);
    mode = storage.getItem(MODE_KEY);
  } catch {
    return { appearance: INITIAL_APPEARANCE, state: "unreadable", stored: { family: null, mode: null } };
  }
  const appearance = { family: family ?? INITIAL_APPEARANCE.family, mode: mode ?? INITIAL_APPEARANCE.mode };
  if (!has(FAMILY_OPTIONS, appearance.family) || !has(MODE_OPTIONS, appearance.mode)) {
    return { appearance: INITIAL_APPEARANCE, state: "unrecorded", stored: { family, mode } };
  }
  return { appearance, state: "read", stored: { family, mode } };
}

/**
 * Stores both axes; false when the storage threw (doc-7 §5.3). Both, even when one changed: with
 * the other holding a value this version lacks, storing one alone would leave the pair drawn as the
 * initial choice.
 */
export function writeAppearance(storage: Pick<Storage, "setItem">, appearance: Appearance): boolean {
  try {
    storage.setItem(FAMILY_KEY, appearance.family);
    storage.setItem(MODE_KEY, appearance.mode);
    return true;
  } catch {
    return false;
  }
}

/**
 * Whether saving writes the appearance: when the chosen one differs from the one drawn, or when an
 * earlier save drew it but could not store it — drawn, it no longer differs, and a retry would
 * otherwise report it saved while the next launch draws the old one.
 */
export function appearanceToStore(chosen: Appearance, shown: Appearance, unstored: boolean): boolean {
  return unstored || chosen.family !== shown.family || chosen.mode !== shown.mode;
}

declare global {
  interface Window {
    /** Set by the script in index.html. */
    tinyColorScheme?: { choose: (family: string, mode: string) => void };
  }
}

/** Draws `appearance` now, whether or not it could be stored (doc-7 §5.3). */
export function applyAppearance(appearance: Appearance): void {
  window.tinyColorScheme?.choose(appearance.family, appearance.mode);
}

const labelOf = (options: readonly SelectOption[], value: string) =>
  options.find((option) => option.value === value)?.label ?? value;

/** What the settings screen says about a stored choice it cannot draw, or null when there is nothing to say. */
export function appearanceNotice(stored: StoredAppearance): { level: "info" | "degraded"; words: string } | null {
  if (stored.state === "unreadable") {
    // doc-9 §7: 進行中の縮退, and not dismissible — closing it would hide that choices made now do not last.
    return {
      level: "degraded",
      words: "配色の保存先を読めません。ここで選んだ配色は、次回の起動時には残らないことがあります。",
    };
  }
  if (stored.state === "unrecorded") {
    const lacking = [stored.stored.family, stored.stored.mode].filter(
      (value, index): value is string =>
        value !== null && !has(index === 0 ? FAMILY_OPTIONS : MODE_OPTIONS, value),
    );
    return {
      level: "info",
      words: `保存されている配色の選択「${lacking.join("・")}」はこのバージョンにありません。${labelOf(FAMILY_OPTIONS, INITIAL_APPEARANCE.family)}・${labelOf(MODE_OPTIONS, INITIAL_APPEARANCE.mode)}で表示しています。保存されている値は、配色を選び直して保存するまで消しません。`,
    };
  }
  return null;
}
