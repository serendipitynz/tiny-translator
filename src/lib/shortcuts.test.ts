import { describe, expect, it } from "vitest";
import { isCopyTranslationShortcut, isTranslateShortcut } from "./shortcuts";

type Keys = Parameters<typeof isTranslateShortcut>[0];

const keys = (key: string, rest: Partial<Keys>): Keys => ({
  key,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  isComposing: false,
  keyCode: 0,
  ...rest,
});

describe("isTranslateShortcut", () => {
  const press = (rest: Partial<Keys>) => isTranslateShortcut(keys("Enter", { keyCode: 13, ...rest }));

  it("takes Ctrl+Enter and Cmd+Enter", () => {
    expect(press({ ctrlKey: true })).toBe(true);
    expect(press({ metaKey: true })).toBe(true);
  });

  it("leaves a plain Enter and other keys to the field", () => {
    expect(press({})).toBe(false);
    expect(isTranslateShortcut(keys("a", { ctrlKey: true }))).toBe(false);
  });

  it("leaves an Enter that commits IME composition alone", () => {
    expect(press({ metaKey: true, isComposing: true })).toBe(false);
    expect(press({ metaKey: true, keyCode: 229 })).toBe(false);
  });
});

describe("isCopyTranslationShortcut", () => {
  it("takes Ctrl+Shift+C and Cmd+Shift+C, whichever case the key arrives in", () => {
    expect(isCopyTranslationShortcut(keys("C", { ctrlKey: true, shiftKey: true }))).toBe(true);
    expect(isCopyTranslationShortcut(keys("c", { metaKey: true, shiftKey: true }))).toBe(true);
  });

  it("leaves the plain copy and Shift+C to the field", () => {
    expect(isCopyTranslationShortcut(keys("c", { metaKey: true }))).toBe(false);
    expect(isCopyTranslationShortcut(keys("C", { shiftKey: true }))).toBe(false);
  });

  it("does nothing while an IME is composing", () => {
    expect(isCopyTranslationShortcut(keys("C", { metaKey: true, shiftKey: true, isComposing: true }))).toBe(false);
  });
});
