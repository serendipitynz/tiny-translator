// Why each main-screen action cannot run right now, or null when it can. A disabled control must
// carry its reason in words (snz-design doc-8 §5.4), so the reason is what disables it.

export function translateBlocker(source: string): string | null {
  return source.trim() === "" ? "原文が空なので翻訳できません" : null;
}

export function copyBlocker(text: string, what: "原文" | "訳文"): string | null {
  return text === "" ? `${what}が空なのでコピーできません` : null;
}

export function clearBlocker(source: string): string | null {
  return source === "" ? "原文はすでに空です" : null;
}

export function stopBlocker(translating: boolean): string | null {
  return translating ? null : "翻訳していないので停止できません";
}

export function swapBlocker(translating: boolean): string | null {
  return translating ? "翻訳中は入れ替えられません" : null;
}
