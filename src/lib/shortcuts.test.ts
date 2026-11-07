import { describe, expect, it } from "vitest";
import { isTranslateShortcut } from "./shortcuts";

const press = (keys: Partial<Parameters<typeof isTranslateShortcut>[0]>) =>
  isTranslateShortcut({ key: "Enter", ctrlKey: false, metaKey: false, isComposing: false, keyCode: 13, ...keys });

describe("isTranslateShortcut", () => {
  it("takes Ctrl+Enter and Cmd+Enter", () => {
    expect(press({ ctrlKey: true })).toBe(true);
    expect(press({ metaKey: true })).toBe(true);
  });

  it("leaves a plain Enter and other keys to the field", () => {
    expect(press({})).toBe(false);
    expect(press({ key: "a", ctrlKey: true })).toBe(false);
  });

  it("leaves an Enter that commits IME composition alone", () => {
    expect(press({ metaKey: true, isComposing: true })).toBe(false);
    expect(press({ metaKey: true, keyCode: 229 })).toBe(false);
  });
});
