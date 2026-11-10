import { Channel, invoke } from "@tauri-apps/api/core";
import licenceEn from "../licenses/plamo-community-license.en.txt?raw";
import licenceJa from "../licenses/plamo-community-license.ja.txt?raw";
import { sizeLabel } from "./settings";
import type { SelectOption } from "./ui/Select.svelte";

/** A model the app can fetch (doc-1 §10.4), as `models_catalog` reports it. */
export interface CatalogModel {
  /** The GGUF's file name, in the repository and in the models directory. */
  name: string;
  /** "0.8B", "2B" or "8B" (doc-1 §4.4). */
  size: string;
  /** The Hugging Face repository it is fetched from. */
  repo: string;
  bytes: number;
  /** Whether the models directory already has a file of this name. */
  present: boolean;
  /** Bytes an earlier fetch left, which the next one resumes from. */
  partial: number;
}

export interface Catalog {
  /** The models directory (doc-1 §10). */
  dir: string;
  models: CatalogModel[];
}

export function loadCatalog(): Promise<Catalog> {
  return invoke<Catalog>("models_catalog");
}

export interface FetchProgress {
  phase: "fetching" | "verifying";
  bytes: number;
}

/**
 * Fetches the catalog's model `name`. Resolves "cancelled" when `cancelFetch` stopped it, and
 * rejects with the reason when it failed.
 */
export function fetchModel(name: string, onProgress: (progress: FetchProgress) => void): Promise<"done" | "cancelled"> {
  const channel = new Channel<FetchProgress>();
  channel.onmessage = onProgress;
  return invoke<"done" | "cancelled">("model_download", { name, progress: channel });
}

export function cancelFetch(): Promise<void> {
  return invoke("model_download_cancel");
}

/** What each size is for (doc-1 §4.4). */
const SIZE_NOTES: Record<string, string> = {
  "0.8B": "省メモリ",
  "2B": "既定",
  "8B": "VRAM 8 GB の機械向け",
};

/** The models that can still be fetched: those the models directory does not have yet. */
export function catalogOptions(models: readonly CatalogModel[]): SelectOption[] {
  return models
    .filter((model) => !model.present)
    .map((model) => {
      const note = SIZE_NOTES[model.size];
      const size = `${model.size} (${sizeLabel(model.bytes)})`;
      return { value: model.name, label: note === undefined ? size : `${size}・${note}` };
    });
}

/** The model offered first: the default 2B (doc-1 §4.4) unless it is already there. */
export function firstChoice(models: readonly CatalogModel[]): string {
  const fetchable = models.filter((model) => !model.present);
  return (fetchable.find((model) => model.size === "2B") ?? fetchable[0])?.name ?? "";
}

function gigabytes(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

/** "0.42 GB / 1.68 GB (25%)": exact enough that the numbers are seen to move. */
export function progressReadout(done: number, total: number): string {
  const percent = total === 0 ? 0 : Math.floor((Math.min(done, total) / total) * 100);
  return `${gigabytes(done)} / ${gigabytes(total)} (${percent}%)`;
}

export type LicenceLanguage = "ja" | "en";

/**
 * PLaMo Community License as the pinned commits carry it (doc-1 §10.3's D3): the repository's
 * `LICENSE/ja` and `LICENSE/en`, copied byte for byte, not rewritten.
 */
export const LICENCE = {
  revised: "2026 年 11 月 9 日改定",
  texts: { ja: licenceJa, en: licenceEn } satisfies Record<LicenceLanguage, string>,
};

export const LICENCE_LANGUAGE_OPTIONS: SelectOption[] = [
  { value: "ja", label: "日本語" },
  { value: "en", label: "English" },
];
