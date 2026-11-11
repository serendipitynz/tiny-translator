import { describe, expect, it } from "vitest";
import { formatOptions } from "./formats";
import { AUTO } from "./languages";

describe("formatOptions", () => {
  it("offers 自動 first, then each format the translation reads", () => {
    expect(formatOptions(null)).toEqual([
      { value: AUTO, label: "自動" },
      { value: "text", label: "テキスト" },
      { value: "markdown", label: "Markdown" },
      { value: "json", label: "JSON" },
      { value: "arb", label: "ARB" },
      { value: "strings", label: ".strings" },
      { value: "html", label: "HTML" },
    ]);
  });

  it("names the detected format in 自動", () => {
    expect(formatOptions("markdown")[0]).toEqual({ value: AUTO, label: "自動 (Markdown)" });
    expect(formatOptions("text")[0]).toEqual({ value: AUTO, label: "自動 (テキスト)" });
    expect(formatOptions("strings")[0]).toEqual({ value: AUTO, label: "自動 (.strings)" });
    expect(formatOptions("html").slice(1)).toEqual(formatOptions(null).slice(1));
  });
});
