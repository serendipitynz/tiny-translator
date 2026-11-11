import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/** What `update_state` returns and `update-state` carries (`src-tauri/src/update.rs`). */
export type UpdateState =
  | { phase: "none" }
  | { phase: "available"; version: string }
  | { phase: "downloading"; version: string; received: number; total: number | null }
  | { phase: "installing"; version: string }
  | { phase: "failed"; version: string; reason: string };

/**
 * What `update_check` answers: `skipped` for a check at launch the settings turned off or one made
 * less than a day ago, `busy` while an update is being installed.
 */
export type CheckResult = "available" | "none" | "skipped" | "busy";

/**
 * Asks whether a newer version is published (doc-1 §14). `manual` is 今すぐ確認; a check at launch
 * is left to the app's own once-a-day rule. Rejects with the reason when the question could not be
 * asked.
 */
export function checkForUpdate(manual: boolean): Promise<CheckResult> {
  return invoke<CheckResult>("update_check", { manual });
}

/** Downloads, verifies and installs the offered update, then the app restarts. Progress and failure arrive as state. */
export function installUpdate(): Promise<void> {
  return invoke("update_install");
}

export function dismissUpdate(): Promise<void> {
  return invoke("update_dismiss");
}

/**
 * Calls `onChange` with the update's current state and with every change after it, as `watchEngine`
 * does for the engine. Returns the function that stops watching.
 */
export function watchUpdate(onChange: (state: UpdateState) => void): () => void {
  let stopped = false;
  let heard = false;
  let unlisten: (() => void) | null = null;

  listen<UpdateState>("update-state", (event) => {
    heard = true;
    if (!stopped) {
      onChange(event.payload);
    }
  }).then((fn) => {
    if (stopped) {
      fn();
      return;
    }
    unlisten = fn;
    invoke<UpdateState>("update_state").then((state) => {
      if (!stopped && !heard) {
        onChange(state);
      }
    });
  });

  return () => {
    stopped = true;
    unlisten?.();
  };
}

/** What the screen says about the update, or null when there is nothing to say. */
export function updateWords(state: UpdateState): string | null {
  switch (state.phase) {
    case "available":
      return `新しいバージョン ${state.version} があります。更新するとアプリを再起動します。`;
    case "downloading":
      return `新しいバージョン ${state.version} をダウンロードしています`;
    case "installing":
      return `新しいバージョン ${state.version} に入れ替えています。終わるとアプリを再起動します。`;
    case "failed":
      return `新しいバージョン ${state.version} に更新できませんでした。今のバージョンのまま使えます。(${state.reason})`;
    default:
      return null;
  }
}

/** What 今すぐ確認 reports beside its button, or null when the update notice says it instead. */
export function checkWords(result: CheckResult): string | null {
  return result === "none" ? "最新のバージョンです" : null;
}
