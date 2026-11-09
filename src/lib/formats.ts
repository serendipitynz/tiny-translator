import { AUTO } from "./languages";
import type { Format } from "./translate";
import type { SelectOption } from "./ui/Select.svelte";

const FORMATS: readonly { value: Format; label: string }[] = [
  { value: "text", label: "テキスト" },
  { value: "markdown", label: "Markdown" },
  { value: "json", label: "JSON" },
  { value: "html", label: "HTML" },
];

/**
 * The format selector's choices (doc-1 §7.2). 自動 names the format detected from the source, as
 * 「自動 (Markdown)」 (§8.1), once there is one.
 */
export function formatOptions(detected: Format | null): SelectOption[] {
  const name = FORMATS.find((format) => format.value === detected)?.label;
  return [{ value: AUTO, label: name === undefined ? "自動" : `自動 (${name})` }, ...FORMATS];
}
