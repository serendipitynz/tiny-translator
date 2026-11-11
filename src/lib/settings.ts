import { invoke } from "@tauri-apps/api/core";
import type { EngineState } from "./engine";
import { DEFAULT_PRIORITIES, isListedLanguage, type Priorities } from "./languages";
import type { SelectOption } from "./ui/Select.svelte";

/** What `settings_get` and `settings_save` carry (`src-tauri/src/settings.rs`). */
export interface Settings {
  /** 第一優先言語 and 第二優先言語, as the codes the selectors list (doc-1 §6.4). */
  firstLanguage: string;
  secondLanguage: string;
  /** 自動解放までの時間 in minutes; 0 never releases (doc-1 §4.5). */
  idleReleaseMinutes: number;
  /** The GGUF's file name in the models directory, or null for the default choice (doc-1 §10). */
  model: string | null;
  /** Whether 起動時のクリップボード翻訳 runs (doc-1 §6.3). */
  launchClipboard: boolean;
  /** The 翻訳しないキー of JSON, ARB and .strings (doc-1 §7.1). */
  untranslatedKeys: string[];
  /** Whether the app asks for a newer version at launch (doc-1 §14). */
  updateCheck: boolean;
}

/** How `settings.json` stood when it was read at launch, or after the last save. */
export type FileState =
  | { state: "read" }
  | { state: "missing" }
  | { state: "moved"; path: string; reason: string }
  | { state: "unmovable"; reason: string }
  | { state: "newer"; version: number };

export interface LoadedSettings {
  settings: Settings;
  file: FileState;
}

export function loadSettings(): Promise<LoadedSettings> {
  return invoke<LoadedSettings>("settings_get");
}

/**
 * Puts `settings` into effect and saves them. Rejects with the reason when they could not be saved;
 * they are in effect for this launch all the same (snz-design doc-7 §5.3).
 */
export function saveSettings(settings: Settings): Promise<void> {
  return invoke("settings_save", { settings });
}

export interface ModelFile {
  name: string;
  bytes: number;
}

export interface Models {
  /** The models directory (doc-1 §10). */
  dir: string;
  models: ModelFile[];
  /** The GGUF the engine loads on its next start, or null when the directory has none. */
  chosen: string | null;
}

export function listModels(): Promise<Models> {
  return invoke<Models>("models_list");
}

/** The engine's memory use in bytes while it runs, or null (doc-1 §4.5). */
export function engineMemory(): Promise<number | null> {
  return invoke<number | null>("engine_memory");
}

/**
 * The priorities translations use. A stored code the selectors do not list (a file edited by hand)
 * is read as its default, the way the colour axes are (doc-7 §5.1).
 */
export function priorities(settings: Settings): Priorities {
  const read = {
    first: isListedLanguage(settings.firstLanguage) ? settings.firstLanguage : DEFAULT_PRIORITIES.first,
    second: isListedLanguage(settings.secondLanguage) ? settings.secondLanguage : DEFAULT_PRIORITIES.second,
  };
  // One default standing in for the other's unlisted code can make the two the same language,
  // which would translate text in it into itself.
  return read.first === read.second ? DEFAULT_PRIORITIES : read;
}

/** The 翻訳しないキー field's text: one key per line. */
export function keysToText(keys: readonly string[]): string {
  return keys.join("\n");
}

/**
 * The keys a 翻訳しないキー field names: one per line, without the spaces around it, blank lines and
 * repeats dropped. Trimmed because a space typed after a key would otherwise make it match nothing,
 * and a JSON key that starts or ends with a space is rarer than that slip.
 */
export function keysFromText(text: string): string[] {
  const keys = text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
  return [...new Set(keys)];
}

const IDLE_RELEASE_CHOICES = [1, 3, 5, 10, 15, 30, 60, 0];

export function idleReleaseLabel(minutes: number): string {
  return minutes === 0 ? "解放しない" : `${minutes} 分`;
}

/** The choices for 自動解放までの時間, with a stored value outside them kept as one more. */
export function idleReleaseOptions(current: number): SelectOption[] {
  const minutes = IDLE_RELEASE_CHOICES.includes(current) ? IDLE_RELEASE_CHOICES : [...IDLE_RELEASE_CHOICES, current];
  return minutes.map((value) => ({
    value: String(value),
    label: value === 10 ? `${idleReleaseLabel(value)} (既定)` : idleReleaseLabel(value),
  }));
}

/** "約 1.8 GB": a size to read at a glance, which is all a 目安 needs. */
export function sizeLabel(bytes: number): string {
  const gib = bytes / 1024 ** 3;
  if (gib >= 1) {
    return `約 ${gib.toFixed(1)} GB`;
  }
  return `約 ${Math.max(1, Math.round(bytes / 1024 ** 2))} MB`;
}

/**
 * The model selector's choices: each GGUF with its size, which is roughly what the engine takes in
 * memory (doc-1 §4.5). A stored choice that is no longer in the directory stays listed and says so,
 * so the selector does not quietly show another model as the one saved.
 */
export function modelOptions(models: readonly ModelFile[], selected: string | null): SelectOption[] {
  const options = models.map((model) => ({ value: model.name, label: `${model.name} (${sizeLabel(model.bytes)})` }));
  if (selected !== null && !models.some((model) => model.name === selected)) {
    options.push({ value: selected, label: `${selected} (見つかりません)` });
  }
  return options;
}

const ENGINE_STATE_LABELS: Record<EngineState, string> = {
  stopped: "停止中",
  loading: "読み込み中",
  ready: "待機中",
};

export function engineStateLabel(state: EngineState): string {
  return ENGINE_STATE_LABELS[state];
}

export function sameSettings(a: Settings, b: Settings): boolean {
  return (
    a.firstLanguage === b.firstLanguage &&
    a.secondLanguage === b.secondLanguage &&
    a.idleReleaseMinutes === b.idleReleaseMinutes &&
    a.model === b.model &&
    a.launchClipboard === b.launchClipboard &&
    a.updateCheck === b.updateCheck &&
    a.untranslatedKeys.length === b.untranslatedKeys.length &&
    a.untranslatedKeys.every((key, index) => key === b.untranslatedKeys[index])
  );
}

/** Why the settings cannot be saved as they stand, or null when they can. */
export function saveBlocker(settings: Settings): string | null {
  if (settings.firstLanguage === settings.secondLanguage) {
    // 優先順位による翻訳先の決定 (doc-1 §6.2) would translate a text in the first language into itself.
    return "第一優先言語と第二優先言語には別の言語を選んでください";
  }
  return null;
}

/** What the settings screen says about how `settings.json` stood, or null when there is nothing to say. */
export function fileNotice(file: FileState): { level: "info" | "degraded"; words: string } | null {
  switch (file.state) {
    case "moved":
      return {
        level: "info",
        words: `設定ファイルを読めなかったため、既定の設定で起動しました。読めなかったファイルは ${file.path} に移してあります。`,
      };
    case "unmovable":
      return {
        level: "degraded",
        words: `設定ファイルを読めず、別の名前に移すこともできませんでした。ファイルを上書きしないよう、ここで変えた設定は保存しません。この起動の間だけ効きます。(${file.reason})`,
      };
    case "newer":
      return {
        level: "degraded",
        words: "設定ファイルはこのバージョンより新しいバージョンのアプリが保存したものです。そのアプリの設定を消さないよう、ここで変えた設定は保存しません。この起動の間だけ効きます。",
      };
    default:
      return null;
  }
}

/** What the settings screen says about stored languages the selectors do not list, or null. */
export function languageNotice(settings: Settings): string | null {
  const unlisted = [settings.firstLanguage, settings.secondLanguage].filter((code) => !isListedLanguage(code));
  if (unlisted.length === 0) {
    return null;
  }
  return `保存されている優先言語「${unlisted.join("・")}」は選べる言語にありません。既定の言語で翻訳しています。`;
}
