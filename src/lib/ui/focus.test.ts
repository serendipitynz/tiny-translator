import { describe, expect, it } from "vitest";
import { focusWasLost } from "./focus";

describe("focusWasLost", () => {
  const body = { tagName: "BODY" } as Element;

  it("is lost when the focused control went with its row", () => {
    expect(focusWasLost(body, body)).toBe(true);
    expect(focusWasLost(null, body)).toBe(true);
  });

  it("is not lost when a newer layer holds it", () => {
    const cancelInConfirmation = { tagName: "BUTTON" } as Element;
    expect(focusWasLost(cancelInConfirmation, body)).toBe(false);
  });
});
