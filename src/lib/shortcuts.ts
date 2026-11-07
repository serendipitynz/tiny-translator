type Keys = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "isComposing" | "keyCode">;

// Either modifier is taken on every OS, so neither has to be told apart by platform. Not while an IME
// is composing, whose Enter commits the text (snz-design doc-8 §5.6); WebKit reports that press as
// keyCode 229 without setting isComposing.
function isCommand(event: Keys): boolean {
  if (event.isComposing || event.keyCode === 229) {
    return false;
  }
  return event.ctrlKey || event.metaKey;
}

/** Whether a key press is Ctrl/Cmd+Enter, the translate shortcut (doc-1 §8.1). */
export function isTranslateShortcut(event: Keys): boolean {
  return isCommand(event) && event.key === "Enter";
}

/**
 * Whether a key press is Ctrl/Cmd+Shift+C, the shortcut that copies the translation (doc-1 §8.1).
 * Shift makes some engines report the key as "C", others as "c".
 */
export function isCopyTranslationShortcut(event: Keys): boolean {
  return isCommand(event) && event.shiftKey && event.key.toLowerCase() === "c";
}
