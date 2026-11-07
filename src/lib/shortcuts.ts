type Keys = Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "isComposing" | "keyCode">;

/**
 * Whether a key press is Ctrl/Cmd+Enter, the translate shortcut (doc-1 §8.1). Either modifier is
 * taken on every OS, so neither has to be told apart by platform. Not while an IME is composing,
 * whose Enter commits the text (snz-design doc-8 §5.6); WebKit reports that press as keyCode 229
 * without setting isComposing.
 */
export function isTranslateShortcut(event: Keys): boolean {
  if (event.isComposing || event.keyCode === 229) {
    return false;
  }
  return event.key === "Enter" && (event.ctrlKey || event.metaKey);
}
