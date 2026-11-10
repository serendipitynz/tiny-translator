type Target = Partial<Pick<HTMLInputElement, "tagName" | "type">>;

/**
 * Whether a right click on this element may open the webview's own context menu. Only the fields
 * that hold text keep it, for cut, copy and paste (TASK-27); everywhere else the menu offers Reload,
 * Back and Print, and reloading discards the source text just as F5 would. A text field has no
 * element children, so the click's target is the field itself.
 */
export function allowsContextMenu(target: Target | null): boolean {
  if (target === null) {
    return false;
  }
  if (target.tagName === "TEXTAREA") {
    return true;
  }
  return target.tagName === "INPUT" && target.type === "text";
}
