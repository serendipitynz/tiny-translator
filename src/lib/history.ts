import { invoke } from "@tauri-apps/api/core";
import { AUTO, languageName, SOURCE_OPTIONS, TARGET_OPTIONS } from "./languages";
import type { SelectOption } from "./ui/Select.svelte";

/** One translation in 翻訳履歴, as `history_search` in `src-tauri/src/lib.rs` answers it. */
export interface HistoryEntry {
  id: number;
  sourceText: string;
  translation: string;
  /** The source language the translation used, or null for doc-1 §6.1's 判定なし. */
  sourceLang: string | null;
  targetLang: string;
  format: string;
  model: string;
  /** Milliseconds since the Unix epoch. */
  createdAt: number;
}

/**
 * The translations containing every whitespace-separated term of `query` in their source or
 * translation, newest first; all of them for a blank `query` (doc-1 §9).
 */
export function searchHistory(query: string): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("history_search", { query });
}

export function countHistory(): Promise<number> {
  return invoke<number>("history_count");
}

/** Resolves to false when the translation was already gone. */
export function deleteHistoryEntry(id: number): Promise<boolean> {
  return invoke<boolean>("history_delete", { id });
}

export function clearHistory(): Promise<void> {
  return invoke("history_clear");
}

/** What picking a translation from the history puts back on the screen (doc-1 §9). */
export interface Restored {
  source: string;
  target: string;
  sourceLanguage: string;
  targetLanguage: string;
}

function selectable(options: readonly SelectOption[], code: string | null): string {
  return code !== null && options.some((option) => option.value === code) ? code : AUTO;
}

/**
 * The selectors are set to the languages the translation used rather than to 自動, which the history
 * does not record, so translating again goes the same way (2026-11-09 owner decision). 判定なし, and
 * a code the selectors no longer list, fall back to 自動 rather than to a value no option shows.
 */
export function restore(entry: HistoryEntry): Restored {
  return {
    source: entry.sourceText,
    target: entry.translation,
    sourceLanguage: selectable(SOURCE_OPTIONS, entry.sourceLang),
    targetLanguage: selectable(TARGET_OPTIONS, entry.targetLang),
  };
}

/** The direction of a saved translation, as 「英語 → 日本語」, or 「判定なし → 日本語」. */
export function directionLabel(entry: Pick<HistoryEntry, "sourceLang" | "targetLang">): string {
  const source = entry.sourceLang === null ? "判定なし" : languageName(entry.sourceLang);
  return `${source} → ${languageName(entry.targetLang)}`;
}

const pad = (value: number) => String(value).padStart(2, "0");

/** When a translation was saved, in local time, as 「2026/11/09 14:03」. */
export function savedAtLabel(createdAt: number): string {
  const at = new Date(createdAt);
  const date = `${at.getFullYear()}/${pad(at.getMonth() + 1)}/${pad(at.getDate())}`;
  return `${date} ${pad(at.getHours())}:${pad(at.getMinutes())}`;
}

/** The terms of a search, as the history reads them: split on whitespace. */
export function searchTerms(query: string): string[] {
  return query.split(/\s+/).filter((term) => term !== "");
}

/**
 * The words an empty list says (snz-design doc-9 §5.4): what would fill it when nothing narrows it,
 * and which condition left nothing when a search does.
 */
export function emptyListWords(query: string): string {
  const terms = searchTerms(query);
  if (terms.length === 0) {
    return "翻訳履歴はまだありません。翻訳が終わるたびにここへ保存されます。";
  }
  return `${terms.map((term) => `「${term}」`).join("と")}を含む翻訳はありません。`;
}

// Long enough to tell translations apart by their first words, short enough to read aloud in a row.
const NAME_EXCERPT_CHARS = 40;

/**
 * What a screen reader calls a row of the list: when, which way, and how the source begins. The
 * whole source and translation would make every row a paragraph long.
 */
export function entryName(entry: HistoryEntry): string {
  const source = [...entry.sourceText.replace(/\s+/g, " ").trim()];
  const excerpt =
    source.length > NAME_EXCERPT_CHARS ? `${source.slice(0, NAME_EXCERPT_CHARS).join("")}…` : source.join("");
  return `${savedAtLabel(entry.createdAt)} ${directionLabel(entry)}: ${excerpt}`;
}
