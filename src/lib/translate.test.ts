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
  it("passes the request and a channel that delivers each piece", async () => {
    tauri.calls = [];
    const pieces: string[] = [];
    const outcome = await translate({ text: "Hello", target: "ja", source: null }, (piece) => pieces.push(piece));
    expect(outcome).toBe("done");
    const [call] = tauri.calls;
    expect(call.command).toBe("translate");
    const { onPiece, ...request } = call.args as { onPiece: { onmessage: (m: string) => void } };
    expect(request).toEqual({ text: "Hello", target: "ja", source: null });
    onPiece.onmessage("こんにちは");
    expect(pieces).toEqual(["こんにちは"]);
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
