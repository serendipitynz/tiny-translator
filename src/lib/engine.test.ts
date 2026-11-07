import { beforeEach, describe, expect, it, vi } from "vitest";

type Handler = (event: { payload: unknown }) => void;

const tauri = vi.hoisted(() => ({
  handlers: [] as Handler[],
  unlistened: 0,
  answer: null as ((payload: unknown) => void) | null,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: async (_name: string, handler: Handler) => {
    tauri.handlers.push(handler);
    return () => {
      tauri.unlistened += 1;
    };
  },
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: () =>
    new Promise((resolve) => {
      tauri.answer = resolve;
    }),
}));

const { watchEngine } = await import("./engine");

const emit = (payload: unknown) => {
  for (const handler of tauri.handlers) {
    handler({ payload });
  }
};
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

beforeEach(() => {
  tauri.handlers = [];
  tauri.unlistened = 0;
  tauri.answer = null;
});

describe("watchEngine", () => {
  it("reports the current status, then each change", async () => {
    const seen: unknown[] = [];
    watchEngine((status) => seen.push(status));
    await settle();
    tauri.answer?.(["loading", null]);
    await settle();
    emit(["ready", null]);
    expect(seen).toEqual([
      { state: "loading", error: null },
      { state: "ready", error: null },
    ]);
  });

  it("asks for the current status only once it is listening", async () => {
    watchEngine(() => {});
    expect(tauri.answer).toBeNull();
    await settle();
    expect(tauri.answer).not.toBeNull();
  });

  it("drops a current status that an event overtook", async () => {
    const seen: unknown[] = [];
    watchEngine((status) => seen.push(status));
    await settle();
    emit(["stopped", "no GGUF in /models"]);
    tauri.answer?.(["loading", null]);
    await settle();
    expect(seen).toEqual([{ state: "stopped", error: "no GGUF in /models" }]);
  });

  it("stops reporting and listening once stopped", async () => {
    const seen: unknown[] = [];
    const stop = watchEngine((status) => seen.push(status));
    await settle();
    stop();
    tauri.answer?.(["ready", null]);
    emit(["stopped", null]);
    await settle();
    expect(seen).toEqual([]);
    expect(tauri.unlistened).toBe(1);
  });
});
