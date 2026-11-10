import { describe, expect, it } from "vitest";
import { allowsContextMenu } from "./contextmenu";

describe("allowsContextMenu", () => {
  it("keeps the menu on the source and translation fields and on single-line text fields", () => {
    expect(allowsContextMenu({ tagName: "TEXTAREA" })).toBe(true);
    expect(allowsContextMenu({ tagName: "INPUT", type: "text" })).toBe(true);
  });

  it("drops it on inputs that hold no text, where the page menu with Reload would open", () => {
    expect(allowsContextMenu({ tagName: "INPUT", type: "checkbox" })).toBe(false);
  });

  it("drops it on the rest of the screen and on targets that are not elements", () => {
    expect(allowsContextMenu({ tagName: "DIV" })).toBe(false);
    expect(allowsContextMenu({ tagName: "SELECT" })).toBe(false);
    expect(allowsContextMenu({})).toBe(false);
    expect(allowsContextMenu(null)).toBe(false);
  });
});
