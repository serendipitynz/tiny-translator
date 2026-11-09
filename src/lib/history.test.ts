import { describe, expect, it } from "vitest";
import { directionLabel, emptyListWords, entryName, type HistoryEntry, restore, savedAtLabel, searchTerms } from "./history";
import { AUTO } from "./languages";

const entry: HistoryEntry = {
  id: 7,
  sourceText: "The weather is nice today.",
  translation: "今日は天気が良い。",
  sourceLang: "en",
  targetLang: "ja",
  format: "text",
  model: "plamo-3-translate-2b-Q4_K_M.gguf",
  createdAt: new Date(2026, 10, 9, 14, 3).getTime(),
};

describe("restore", () => {
  it("puts the source, the translation and the languages it used back", () => {
    expect(restore(entry)).toEqual({
      source: "The weather is nice today.",
      target: "今日は天気が良い。",
      sourceLanguage: "en",
      targetLanguage: "ja",
    });
  });

  it("leaves the source to detection when the translation had none", () => {
    expect(restore({ ...entry, sourceLang: null }).sourceLanguage).toBe(AUTO);
  });

  it("falls back to 自動 for a language the selectors do not list", () => {
    const restored = restore({ ...entry, sourceLang: "xx", targetLang: "yy" });
    expect(restored.sourceLanguage).toBe(AUTO);
    expect(restored.targetLanguage).toBe(AUTO);
  });
});

describe("directionLabel", () => {
  it("names both languages", () => {
    expect(directionLabel(entry)).toBe("英語 → 日本語");
  });

  it("says 判定なし for an undecided source", () => {
    expect(directionLabel({ sourceLang: null, targetLang: "en" })).toBe("判定なし → 英語");
  });
});

describe("savedAtLabel", () => {
  it("gives the local date and time to the minute", () => {
    expect(savedAtLabel(entry.createdAt)).toBe("2026/11/09 14:03");
  });
});

describe("searchTerms", () => {
  it("splits on any whitespace, the full-width space included", () => {
    expect(searchTerms("  天気　weather\n")).toEqual(["天気", "weather"]);
    expect(searchTerms(" \t")).toEqual([]);
  });
});

describe("emptyListWords", () => {
  it("says what fills an empty history", () => {
    expect(emptyListWords("")).toBe("翻訳履歴はまだありません。翻訳が終わるたびにここへ保存されます。");
    expect(emptyListWords("   ")).toBe(emptyListWords(""));
  });

  it("names the terms a search found nothing for", () => {
    expect(emptyListWords("天気")).toBe("「天気」を含む翻訳はありません。");
    expect(emptyListWords("天気  weather")).toBe("「天気」と「weather」を含む翻訳はありません。");
  });
});

describe("entryName", () => {
  it("says when, which way and how the source begins", () => {
    expect(entryName(entry)).toBe("2026/11/09 14:03 英語 → 日本語: The weather is nice today.");
  });

  it("cuts a long source short and folds its line breaks", () => {
    const sourceText = `# 見出し\n\n${"あ".repeat(50)}`;
    expect(entryName({ ...entry, sourceText })).toBe(`2026/11/09 14:03 英語 → 日本語: # 見出し ${"あ".repeat(34)}…`);
  });
});
