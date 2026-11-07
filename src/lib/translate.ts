import { Channel, invoke } from "@tauri-apps/api/core";

/** How a translation ended: the engine finished it, or it was stopped first. */
export type Outcome = "done" | "stopped";

export interface TranslateRequest {
  text: string;
  /** The language code the prompt names as the output's `lang`. */
  target: string;
  /** The source language, or null to leave it to the model (doc-1 §6.1's 判定なし). */
  source: string | null;
}

/**
 * Translates through the engine (`translate` in `src-tauri/src/lib.rs`), handing each piece of the
 * translation to `onPiece` as it is generated. Waits for the engine to start when it is stopped.
 * Rejects with the engine's reason when the translation cannot be made.
 */
export function translate(request: TranslateRequest, onPiece: (piece: string) => void): Promise<Outcome> {
  const channel = new Channel<string>();
  channel.onmessage = onPiece;
  return invoke<Outcome>("translate", { ...request, onPiece: channel });
}

/** Stops the running translation, whose `translate` then resolves to "stopped". */
export function stopTranslation(): Promise<void> {
  return invoke("stop_translation");
}
