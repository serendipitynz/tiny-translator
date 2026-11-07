import { describe, expect, it } from "vitest";
import { clearBlocker, copyBlocker, translateBlocker } from "./blockers";

describe("translateBlocker", () => {
  it("blocks an empty source and one of whitespace only", () => {
    expect(translateBlocker("")).not.toBeNull();
    expect(translateBlocker(" \n\t")).not.toBeNull();
  });

  it("lets a source with text through", () => {
    expect(translateBlocker(" Hello ")).toBeNull();
  });
});

describe("copyBlocker", () => {
  it("names the pane that has nothing to copy", () => {
    expect(copyBlocker("", "訳文")).toContain("訳文");
    expect(copyBlocker("", "原文")).toContain("原文");
  });

  it("lets whitespace be copied, since it is what the pane holds", () => {
    expect(copyBlocker(" ", "原文")).toBeNull();
  });
});

describe("clearBlocker", () => {
  it("blocks only a source that is already empty", () => {
    expect(clearBlocker("")).not.toBeNull();
    expect(clearBlocker(" ")).toBeNull();
  });
});
