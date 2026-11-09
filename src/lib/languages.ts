import type { SelectOption } from "./ui/Select.svelte";

/** The value of either direction selector that leaves the choice to the app (doc-1 §6.1, §6.2). */
export const AUTO = "auto";

/**
 * What `detect_language` answers for Chinese (`src-tauri/src/language.rs`): whatlang does not tell
 * simplified from traditional, so it names neither of PLaMo's `zh-hans` and `zh-hant`.
 */
export const CHINESE = "zh";

// The 53 languages PLaMo 3 Translate translates, by the codes its model card gives the prompt's
// `lang` (doc-1 §6.4). The ones §6.1 measured detection on come first; the rest follow in 五十音 order.
const LANGUAGES: readonly SelectOption[] = [
  { value: "ja", label: "日本語" },
  { value: "en", label: "英語" },
  { value: "zh-hans", label: "中国語 (簡体字)" },
  { value: "zh-hant", label: "中国語 (繁体字)" },
  { value: "ko", label: "韓国語" },
  { value: "az", label: "アゼルバイジャン語" },
  { value: "ar", label: "アラビア語" },
  { value: "hy", label: "アルメニア語" },
  { value: "it", label: "イタリア語" },
  { value: "id", label: "インドネシア語" },
  { value: "uz", label: "ウズベク語" },
  { value: "ur", label: "ウルドゥー語" },
  { value: "et", label: "エストニア語" },
  { value: "nl", label: "オランダ語" },
  { value: "kk", label: "カザフ語" },
  { value: "ca", label: "カタルーニャ語" },
  { value: "gu", label: "グジャラート語" },
  { value: "km", label: "クメール語" },
  { value: "hr", label: "クロアチア語" },
  { value: "jv", label: "ジャワ語" },
  { value: "ka", label: "ジョージア語" },
  { value: "si", label: "シンハラ語" },
  { value: "es", label: "スペイン語" },
  { value: "sk", label: "スロバキア語" },
  { value: "sl", label: "スロベニア語" },
  { value: "sw", label: "スワヒリ語" },
  { value: "ceb", label: "セブアノ語" },
  { value: "sr", label: "セルビア語" },
  { value: "th", label: "タイ語" },
  { value: "tl", label: "タガログ語" },
  { value: "ta", label: "タミル語" },
  { value: "te", label: "テルグ語" },
  { value: "de", label: "ドイツ語" },
  { value: "ne", label: "ネパール語" },
  { value: "eu", label: "バスク語" },
  { value: "hu", label: "ハンガリー語" },
  { value: "pa", label: "パンジャブ語" },
  { value: "hi", label: "ヒンディー語" },
  { value: "fr", label: "フランス語" },
  { value: "bg", label: "ブルガリア語" },
  { value: "vi", label: "ベトナム語" },
  { value: "he", label: "ヘブライ語" },
  { value: "be", label: "ベラルーシ語" },
  { value: "fa", label: "ペルシア語" },
  { value: "bn", label: "ベンガル語" },
  { value: "pl", label: "ポーランド語" },
  { value: "mr", label: "マラーティー語" },
  { value: "ml", label: "マラヤーラム語" },
  { value: "ms", label: "マレー語" },
  { value: "my", label: "ミャンマー語" },
  { value: "lv", label: "ラトビア語" },
  { value: "lt", label: "リトアニア語" },
  { value: "ru", label: "ロシア語" },
];

/** The 53 languages alone, for a selector that offers no 自動 (the settings screen's priorities). */
export const LANGUAGE_OPTIONS: readonly SelectOption[] = LANGUAGES;

/** Whether `code` is one of the 53 the selectors list. */
export function isListedLanguage(code: string): boolean {
  return LANGUAGES.some((language) => language.value === code);
}

export const SOURCE_OPTIONS: readonly SelectOption[] = [{ value: AUTO, label: "自動判定" }, ...LANGUAGES];

export const TARGET_OPTIONS: readonly SelectOption[] = [{ value: AUTO, label: "自動 (優先順位)" }, ...LANGUAGES];

/** 第一優先言語と第二優先言語 (doc-1 §6.2). */
export interface Priorities {
  first: string;
  second: string;
}

/** doc-1 §6.2's defaults, which the settings screen (TASK-21) starts from. */
export const DEFAULT_PRIORITIES: Priorities = { first: "ja", second: "en" };

/** The language's name on the screen, for a code the selectors list or [`CHINESE`]. */
export function languageName(code: string): string {
  if (code === CHINESE) {
    return "中国語";
  }
  return LANGUAGES.find((language) => language.value === code)?.label ?? code;
}

/** Whether a source language is `language`; detected Chinese is both kinds of written Chinese. */
function isLanguage(source: string, language: string): boolean {
  return source === language || (source === CHINESE && language.startsWith("zh-"));
}

/**
 * The source language of the translation: the source selector's choice, or, when it is automatic,
 * the detected one — null when detection left it undecided (doc-1 §6.1's 判定なし).
 */
export function sourceLanguage(selected: string, detected: string | null): string | null {
  return selected === AUTO ? detected : selected;
}

/**
 * The target language: the target selector's choice, or, when it is automatic, the second
 * priority for a source in the first priority and the first priority otherwise — an undecided
 * source included (doc-1 §6.2).
 */
export function targetLanguage(selected: string, source: string | null, priorities: Priorities): string {
  if (selected !== AUTO) {
    return selected;
  }
  return source !== null && isLanguage(source, priorities.first) ? priorities.second : priorities.first;
}

/**
 * The languages the prompt names. Detected Chinese is left out, as an undecided source is: PLaMo
 * needs to be told which written Chinese it is, and the model detects that itself.
 */
export function promptLanguages(
  sourceSelected: string,
  targetSelected: string,
  detected: string | null,
  priorities: Priorities,
): { source: string | null; target: string } {
  const source = sourceLanguage(sourceSelected, detected);
  return {
    source: source === CHINESE ? null : source,
    target: targetLanguage(targetSelected, source, priorities),
  };
}

/** What the screen says about the detected source language: 「判定: 英語」, or 「判定: 不明」 for 判定なし. */
export function detectionLabel(detected: string | null): string {
  return `判定: ${detected === null ? "不明" : languageName(detected)}`;
}
