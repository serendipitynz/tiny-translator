import { describe, expect, it } from "vitest";
import { toastDuration } from "./toast";

describe("toastDuration", () => {
  it("grows with the words", () => {
    expect(toastDuration("あ".repeat(30), 1800, 6000)).toBe(2700);
  });

  it("stays up at least the minimum and at most the maximum", () => {
    expect(toastDuration("訳文をコピーしました", 1800, 6000)).toBe(1800);
    expect(toastDuration("あ".repeat(200), 1800, 6000)).toBe(6000);
  });

  it("counts characters, not UTF-16 units", () => {
    expect(toastDuration("😀".repeat(30), 0, 10000)).toBe(2700);
  });
});
