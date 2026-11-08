import { Channel, invoke } from "@tauri-apps/api/core";

/** How a translation ended: the engine finished it, or it was stopped first. */
export type Outcome = "done" | "stopped";

/** How the original is read (doc-1 §7.1): its structure is kept in the translation. */
export type Format = "text" | "markdown";

export interface TranslateRequest {
  text: string;
  format: Format;
  /** The language code the prompt names as the output's `lang`. */
  target: string;
  /** The source language, or null to leave it to the model (doc-1 §6.1's 判定なし). */
  source: string | null;
}

/** A change to the translation: text to add at the end, or the whole translation so far. */
type Update = ["append" | "replace", string];

/**
 * Translates through the engine (`translate` in `src-tauri/src/lib.rs`) 翻訳単位 by 翻訳単位,
 * handing the whole translation so far to `onText` each time it grows — or shrinks, when the
 * translation of a 翻訳単位 that started repeating itself is taken back (doc-1 §7.3). Waits for the
 * engine to start when it is stopped. Rejects with the engine's reason when the translation cannot
 * be made.
 */
export function translate(request: TranslateRequest, onText: (text: string) => void): Promise<Outcome> {
  let text = "";
  const channel = new Channel<Update>();
  channel.onmessage = ([kind, part]) => {
    text = kind === "append" ? text + part : part;
    onText(text);
  };
  return invoke<Outcome>("translate", { ...request, onUpdate: channel });
}

/**
 * The source language of `text` as PLaMo's language code, `"zh"` for Chinese, or null when it is
 * left undecided (`detect_language` in `src-tauri/src/lib.rs`, doc-1 §6.1).
 */
export function detectLanguage(text: string): Promise<string | null> {
  return invoke<string | null>("detect_language", { text });
}

/** Stops the running translation, whose `translate` then resolves to "stopped". */
export function stopTranslation(): Promise<void> {
  return invoke("stop_translation");
}
