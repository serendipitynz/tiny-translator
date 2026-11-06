import type { SelectOption } from "./ui/Select.svelte";

/** The value of either direction selector that leaves the choice to the app (doc-1 §6.1, §6.2). */
export const AUTO = "auto";

// The languages doc-1 §6.1 measured whatlang on. Which languages are offered, and the codes the
// prompt is given, are TASK-10's to settle when the direction is wired.
const LANGUAGES: readonly SelectOption[] = [
  { value: "ja", label: "日本語" },
  { value: "en", label: "英語" },
  { value: "zh-Hans", label: "中国語 (簡体字)" },
  { value: "zh-Hant", label: "中国語 (繁体字)" },
  { value: "ko", label: "韓国語" },
];

export const SOURCE_OPTIONS: readonly SelectOption[] = [{ value: AUTO, label: "自動判定" }, ...LANGUAGES];

export const TARGET_OPTIONS: readonly SelectOption[] = [{ value: AUTO, label: "自動 (優先順位)" }, ...LANGUAGES];
