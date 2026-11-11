import { describe, expect, it } from "vitest";
import { checkWords, updateWords } from "./update";

describe("updateWords", () => {
  it("say nothing when no newer version is known", () => {
    expect(updateWords({ phase: "none" })).toBeNull();
  });

  it("name the version in every phase", () => {
    expect(updateWords({ phase: "available", version: "0.2.0" })).toContain("0.2.0");
    expect(updateWords({ phase: "downloading", version: "0.2.0", received: 1, total: null })).toContain("0.2.0");
    expect(updateWords({ phase: "installing", version: "0.2.0" })).toContain("0.2.0");
  });

  it("say why an update failed and that the app still works", () => {
    const words = updateWords({ phase: "failed", version: "0.2.0", reason: "signature mismatch" });
    expect(words).toContain("signature mismatch");
    expect(words).toContain("今のバージョンのまま使えます");
  });
});

describe("checkWords", () => {
  it("report an up-to-date app beside the button", () => {
    expect(checkWords("none")).toBe("最新のバージョンです");
  });

  it("leave a found update to the update notice", () => {
    expect(checkWords("available")).toBeNull();
    expect(checkWords("busy")).toBeNull();
  });
});
