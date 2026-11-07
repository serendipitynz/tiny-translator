import { describe, expect, it } from "vitest";
import {
  AUTO,
  CHINESE,
  DEFAULT_PRIORITIES,
  detectionLabel,
  languageName,
  promptLanguages,
  SOURCE_OPTIONS,
  sourceLanguage,
  TARGET_OPTIONS,
  targetLanguage,
} from "./languages";

const enFirst = { first: "en", second: "ja" };

describe("the selectors' languages", () => {
  const languages = SOURCE_OPTIONS.slice(1);

  it("are PLaMo's 53, the same on both sides", () => {
    expect(languages).toHaveLength(53);
    expect(new Set(languages.map((language) => language.value)).size).toBe(53);
    expect(TARGET_OPTIONS.slice(1)).toEqual(languages);
    expect(SOURCE_OPTIONS[0].value).toBe(AUTO);
    expect(TARGET_OPTIONS[0].value).toBe(AUTO);
  });

  it("put the measured five first and the rest in 五十音 order", () => {
    expect(languages.slice(0, 5).map((language) => language.value)).toEqual(["ja", "en", "zh-hans", "zh-hant", "ko"]);
    const rest = languages.slice(5).map((language) => language.label);
    expect(rest).toEqual([...rest].sort((a, b) => a.localeCompare(b, "ja")));
  });
});

describe("targetLanguage", () => {
  it("takes the second priority for a source in the first priority", () => {
    expect(targetLanguage(AUTO, "ja", DEFAULT_PRIORITIES)).toBe("en");
    expect(targetLanguage(AUTO, "en", enFirst)).toBe("ja");
  });

  it("takes the first priority for any other source", () => {
    expect(targetLanguage(AUTO, "en", DEFAULT_PRIORITIES)).toBe("ja");
    expect(targetLanguage(AUTO, "ko", DEFAULT_PRIORITIES)).toBe("ja");
    expect(targetLanguage(AUTO, "ja", enFirst)).toBe("en");
  });

  it("takes the first priority for an undecided source", () => {
    expect(targetLanguage(AUTO, null, DEFAULT_PRIORITIES)).toBe("ja");
    expect(targetLanguage(AUTO, null, enFirst)).toBe("en");
  });

  it("counts detected Chinese as either written Chinese in the priorities", () => {
    expect(targetLanguage(AUTO, CHINESE, { first: "zh-hans", second: "ja" })).toBe("ja");
    expect(targetLanguage(AUTO, CHINESE, { first: "zh-hant", second: "en" })).toBe("en");
    expect(targetLanguage(AUTO, CHINESE, DEFAULT_PRIORITIES)).toBe("ja");
  });

  it("keeps a chosen target whatever the source", () => {
    expect(targetLanguage("ko", "ja", DEFAULT_PRIORITIES)).toBe("ko");
    expect(targetLanguage("ja", "ja", DEFAULT_PRIORITIES)).toBe("ja");
  });
});

describe("sourceLanguage", () => {
  it("takes the detected language while automatic, and the chosen one otherwise", () => {
    expect(sourceLanguage(AUTO, "en")).toBe("en");
    expect(sourceLanguage(AUTO, null)).toBeNull();
    expect(sourceLanguage("fr", "en")).toBe("fr");
  });
});

describe("promptLanguages", () => {
  it("names the detected source and the target the priorities give", () => {
    expect(promptLanguages(AUTO, AUTO, "en", DEFAULT_PRIORITIES)).toEqual({ source: "en", target: "ja" });
    expect(promptLanguages(AUTO, AUTO, "ja", DEFAULT_PRIORITIES)).toEqual({ source: "ja", target: "en" });
  });

  it("leaves an undecided source to the model and translates into the first priority", () => {
    expect(promptLanguages(AUTO, AUTO, null, DEFAULT_PRIORITIES)).toEqual({ source: null, target: "ja" });
  });

  it("leaves detected Chinese out of the prompt", () => {
    expect(promptLanguages(AUTO, AUTO, CHINESE, DEFAULT_PRIORITIES)).toEqual({ source: null, target: "ja" });
  });

  it("follows the chosen source over the detected one", () => {
    expect(promptLanguages("ja", AUTO, "zh", DEFAULT_PRIORITIES)).toEqual({ source: "ja", target: "en" });
    expect(promptLanguages("zh-hant", "en", null, DEFAULT_PRIORITIES)).toEqual({ source: "zh-hant", target: "en" });
  });
});

describe("detectionLabel", () => {
  it("names the detected language, or says it is unknown", () => {
    expect(detectionLabel("en")).toBe("判定: 英語");
    expect(detectionLabel(CHINESE)).toBe("判定: 中国語");
    expect(detectionLabel(null)).toBe("判定: 不明");
  });
});

describe("languageName", () => {
  it("names a selector's language", () => {
    expect(languageName("zh-hant")).toBe("中国語 (繁体字)");
    expect(languageName("ceb")).toBe("セブアノ語");
  });
});
