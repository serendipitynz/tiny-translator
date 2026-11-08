import { describe, expect, it, vi } from "vitest";

const tauri = vi.hoisted(() => ({
  calls: [] as { command: string; args: unknown }[],
}));

vi.mock("@tauri-apps/api/core", () => ({
  Channel: class {
    onmessage: (message: unknown) => void = () => {};
  },
  invoke: async (command: string, args: unknown) => {
    tauri.calls.push({ command, args });
    if (command === "translate") {
      return "done";
    }
    return command === "detect_language" ? "en" : undefined;
  },
}));

const { detectLanguage, translate, stopTranslation } = await import("./translate");

describe("translate", () => {
  it("passes the request and a channel whose updates build up the translation", async () => {
    tauri.calls = [];
    const texts: string[] = [];
    const outcome = await translate(
      { text: "Hello", format: "json", untranslatedKeys: ["id"], target: "ja", source: null },
      (text) => texts.push(text),
    );
    expect(outcome).toBe("done");
    const [call] = tauri.calls;
    expect(call.command).toBe("translate");
    const { onUpdate, ...request } = call.args as { onUpdate: { onmessage: (m: [string, string]) => void } };
    expect(request).toEqual({ text: "Hello", format: "json", untranslatedKeys: ["id"], target: "ja", source: null });
    onUpdate.onmessage(["append", "こんにちは"]);
    onUpdate.onmessage(["append", "。\n\n"]);
    onUpdate.onmessage(["append", "繰り返し繰り返し"]);
    // The repetition is taken back to where its 翻訳単位 began, and the translation goes on.
    onUpdate.onmessage(["replace", "こんにちは。\n\n"]);
    onUpdate.onmessage(["append", "世界。"]);
    expect(texts).toEqual([
      "こんにちは",
      "こんにちは。\n\n",
      "こんにちは。\n\n繰り返し繰り返し",
      "こんにちは。\n\n",
      "こんにちは。\n\n世界。",
    ]);
  });

  it("stops through its own command", async () => {
    tauri.calls = [];
    await stopTranslation();
    expect(tauri.calls).toEqual([{ command: "stop_translation", args: undefined }]);
  });

  it("detects the source language through its own command", async () => {
    tauri.calls = [];
    expect(await detectLanguage("Hello, world")).toBe("en");
    expect(tauri.calls).toEqual([{ command: "detect_language", args: { text: "Hello, world" } }]);
  });
});
