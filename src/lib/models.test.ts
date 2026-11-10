import { describe, expect, it } from "vitest";
import { type CatalogModel, catalogOptions, firstChoice, LICENCE, progressReadout } from "./models";

const model = (size: string, bytes: number, present = false): CatalogModel => ({
  name: `plamo-3-translate-${size.toLowerCase()}-Q4_K_M.gguf`,
  size,
  repo: `pfnet/plamo-3-translate-${size.toLowerCase()}-GGUF`,
  bytes,
  present,
  partial: 0,
});

const catalog = [model("0.8B", 573_126_848), model("2B", 1_807_203_040), model("8B", 5_280_445_728)];

describe("catalogOptions", () => {
  it("names each model by its size, what it takes and what it is for", () => {
    expect(catalogOptions(catalog).map((option) => option.label)).toEqual([
      "0.8B (約 547 MB)・省メモリ",
      "2B (約 1.7 GB)・既定",
      "8B (約 4.9 GB)・VRAM 8 GB の機械向け",
    ]);
  });

  it("leaves out the models already there", () => {
    const options = catalogOptions([catalog[0], { ...catalog[1], present: true }, catalog[2]]);
    expect(options.map((option) => option.value)).toEqual([catalog[0].name, catalog[2].name]);
  });
});

describe("firstChoice", () => {
  it("is the default 2B", () => {
    expect(firstChoice(catalog)).toBe(catalog[1].name);
  });

  it("is the first other model once 2B is there", () => {
    expect(firstChoice([catalog[0], { ...catalog[1], present: true }, catalog[2]])).toBe(catalog[0].name);
  });

  it("is empty when every model is there", () => {
    expect(firstChoice(catalog.map((entry) => ({ ...entry, present: true })))).toBe("");
  });
});

describe("progressReadout", () => {
  it("gives both amounts and the share done", () => {
    expect(progressReadout(452_984_832, 1_807_203_040)).toBe("0.42 GB / 1.68 GB (25%)");
  });

  it("says 100% only once everything is done", () => {
    expect(progressReadout(1_807_203_039, 1_807_203_040)).toBe("1.68 GB / 1.68 GB (99%)");
    expect(progressReadout(1_807_203_040, 1_807_203_040)).toBe("1.68 GB / 1.68 GB (100%)");
  });
});

describe("LICENCE", () => {
  it("holds both languages of the revision the pinned commits carry", () => {
    expect(LICENCE.texts.ja.startsWith("PLaMoコミュニティライセンス契約")).toBe(true);
    expect(LICENCE.texts.ja.trimEnd().endsWith("2026 年 11 月 9 日改定")).toBe(true);
    expect(LICENCE.texts.en.startsWith("PLaMo Community License Agreement")).toBe(true);
  });
});
