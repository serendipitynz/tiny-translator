import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

/**
 * What 起動時のクリップボード翻訳 found (`launch_clipboard` in `src-tauri/src/lib.rs`, doc-1 §6.3):
 * text to translate, nothing new, or a clipboard left unread because macOS would have asked the user.
 */
export type LaunchClipboard = { kind: "text"; text: string } | { kind: "none" } | { kind: "not-allowed" };

type Payload = ["text", string] | ["none" | "not-allowed", null];

export async function launchClipboard(): Promise<LaunchClipboard> {
  const [kind, text] = await invoke<Payload>("launch_clipboard");
  return kind === "text" ? { kind, text } : { kind };
}

/**
 * The clipboard's text for the 貼り付け button, or null when it holds none. Where macOS asks before
 * a read, the promise waits for the user's answer.
 */
export function pasteClipboard(): Promise<string | null> {
  return invoke<string | null>("paste_clipboard");
}

/** Rejects with the reason when the clipboard could not be written. */
export function copyText(text: string): Promise<void> {
  return invoke("copy_text", { text });
}

/** Calls `onLaunch` whenever the app is launched again while running. Returns the function that stops listening. */
export function onLaunchedAgain(onLaunch: () => void): () => void {
  let stopped = false;
  let unlisten: (() => void) | null = null;
  listen("launched-again", () => {
    if (!stopped) {
      onLaunch();
    }
  }).then((fn) => {
    if (stopped) {
      fn();
      return;
    }
    unlisten = fn;
  });
  return () => {
    stopped = true;
    unlisten?.();
  };
}
