import { describe, expect, it } from "vitest";
import {
  fileNotice,
  idleReleaseOptions,
  keysFromText,
  keysToText,
  languageNotice,
  modelOptions,
  priorities,
  sameSettings,
  saveBlocker,
  type Settings,
  sizeLabel,
} from "./settings";

const settings: Settings = {
  firstLanguage: "ja",
  secondLanguage: "en",
  idleReleaseMinutes: 10,
  model: null,
  launchClipboard: true,
  untranslatedKeys: [],
};

describe("priorities", () => {
  it("are the stored languages", () => {
    expect(priorities({ ...settings, firstLanguage: "en", secondLanguage: "zh-hans" })).toEqual({
      first: "en",
      second: "zh-hans",
    });
  });

  it("read a code the selectors do not list as its default", () => {
    expect(priorities({ ...settings, firstLanguage: "xx", secondLanguage: "ko" })).toEqual({ first: "ja", second: "ko" });
  });

  it("fall back to both defaults when a default would make the two the same", () => {
    expect(priorities({ ...settings, firstLanguage: "xx", secondLanguage: "ja" })).toEqual({ first: "ja", second: "en" });
  });
});

describe("the 翻訳しないキー field", () => {
  it("takes one key per line, trimmed, without blanks or repeats", () => {
    expect(keysFromText(" id\n\nurl \n  \nid\nmeta key")).toEqual(["id", "url", "meta key"]);
    expect(keysFromText("")).toEqual([]);
  });

  it("shows the keys one per line, which reads back as the same keys", () => {
    const keys = ["id", "url"];
    expect(keysToText(keys)).toBe("id\nurl");
    expect(keysFromText(keysToText(keys))).toEqual(keys);
  });
});

describe("idleReleaseOptions", () => {
  it("offers the usual minutes, marks the default and ends with never", () => {
    const options = idleReleaseOptions(10);
    expect(options.map((option) => option.value)).toEqual(["1", "3", "5", "10", "15", "30", "60", "0"]);
    expect(options.find((option) => option.value === "10")?.label).toBe("10 分 (既定)");
    expect(options.at(-1)?.label).toBe("解放しない");
  });

  it("keeps a stored value outside them as one more choice", () => {
    expect(idleReleaseOptions(45).at(-1)).toEqual({ value: "45", label: "45 分" });
  });
});

describe("sizeLabel", () => {
  it("gives gigabytes to one place, and megabytes below one", () => {
    expect(sizeLabel(1.81 * 1024 ** 3)).toBe("約 1.8 GB");
    expect(sizeLabel(0.57 * 1024 ** 3)).toBe("約 584 MB");
    expect(sizeLabel(10)).toBe("約 1 MB");
  });
});

describe("modelOptions", () => {
  const models = [
    { name: "plamo-3-translate-0.8b-Q4_K_M.gguf", bytes: 0.57 * 1024 ** 3 },
    { name: "plamo-3-translate-2b-Q4_K_M.gguf", bytes: 1.81 * 1024 ** 3 },
  ];

  it("lists each GGUF with its size", () => {
    expect(modelOptions(models, null)).toEqual([
      { value: "plamo-3-translate-0.8b-Q4_K_M.gguf", label: "plamo-3-translate-0.8b-Q4_K_M.gguf (約 584 MB)" },
      { value: "plamo-3-translate-2b-Q4_K_M.gguf", label: "plamo-3-translate-2b-Q4_K_M.gguf (約 1.8 GB)" },
    ]);
  });

  it("keeps a saved choice that has gone, saying so", () => {
    expect(modelOptions(models, "plamo-3-translate-8b-Q4_K_M.gguf").at(-1)).toEqual({
      value: "plamo-3-translate-8b-Q4_K_M.gguf",
      label: "plamo-3-translate-8b-Q4_K_M.gguf (見つかりません)",
    });
    expect(modelOptions(models, "plamo-3-translate-2b-Q4_K_M.gguf")).toHaveLength(2);
  });
});

describe("sameSettings", () => {
  it("compares every field, the keys in order", () => {
    expect(sameSettings(settings, { ...settings })).toBe(true);
    expect(sameSettings(settings, { ...settings, model: "a.gguf" })).toBe(false);
    expect(sameSettings(settings, { ...settings, launchClipboard: false })).toBe(false);
    expect(sameSettings(settings, { ...settings, idleReleaseMinutes: 0 })).toBe(false);
    const keyed = { ...settings, untranslatedKeys: ["id", "url"] };
    expect(sameSettings(keyed, { ...settings, untranslatedKeys: ["id", "url"] })).toBe(true);
    expect(sameSettings(keyed, { ...settings, untranslatedKeys: ["url", "id"] })).toBe(false);
    expect(sameSettings(keyed, { ...settings, untranslatedKeys: ["id"] })).toBe(false);
  });
});

describe("saveBlocker", () => {
  it("refuses the same language as both priorities", () => {
    expect(saveBlocker({ ...settings, secondLanguage: "ja" })).not.toBeNull();
    expect(saveBlocker(settings)).toBeNull();
  });
});

describe("fileNotice", () => {
  it("says nothing about a file that was read or did not exist", () => {
    expect(fileNotice({ state: "read" })).toBeNull();
    expect(fileNotice({ state: "missing" })).toBeNull();
  });

  it("names where an unreadable file was moved, as 案内", () => {
    const notice = fileNotice({ state: "moved", path: "/data/settings.unreadable-1.json", reason: "EOF" });
    expect(notice?.level).toBe("info");
    expect(notice?.words).toContain("/data/settings.unreadable-1.json");
  });

  it("tells a file that is not written as 進行中の縮退", () => {
    expect(fileNotice({ state: "unmovable", reason: "denied" })?.level).toBe("degraded");
    expect(fileNotice({ state: "newer", version: 2 })?.level).toBe("degraded");
  });
});

describe("languageNotice", () => {
  it("names stored languages the selectors do not list", () => {
    expect(languageNotice(settings)).toBeNull();
    expect(languageNotice({ ...settings, secondLanguage: "xx" })).toContain("「xx」");
  });
});
