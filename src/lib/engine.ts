import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** 翻訳エンジンの状態 (doc-1 §4.5): 停止中・読み込み中・待機中. */
export type EngineState = "stopped" | "loading" | "ready";

export interface EngineStatus {
  state: EngineState;
  /** Why the last start failed or the engine stopped on its own; null once it is ready again. */
  error: string | null;
}

/** What `engine_state` returns and `engine-state` carries (`src-tauri/src/lib.rs`). */
type Payload = [EngineState, string | null];

const toStatus = ([state, error]: Payload): EngineStatus => ({ state, error });

/**
 * Calls `onChange` with the engine's current status and with every change after it. Returns the
 * function that stops watching.
 */
export function watchEngine(onChange: (status: EngineStatus) => void): () => void {
  let stopped = false;
  // Set by the first event. The current status is asked for only once the listener is in place,
  // so no change falls between the two; an event that overtakes the answer is the newer of them.
  let heard = false;
  let unlisten: (() => void) | null = null;

  listen<Payload>("engine-state", (event) => {
    heard = true;
    if (!stopped) {
      onChange(toStatus(event.payload));
    }
  }).then((fn) => {
    if (stopped) {
      fn();
      return;
    }
    unlisten = fn;
    invoke<Payload>("engine_state").then((payload) => {
      if (!stopped && !heard) {
        onChange(toStatus(payload));
      }
    });
  });

  return () => {
    stopped = true;
    unlisten?.();
  };
}
