import { describe, expect, it } from "vitest";
import { appearanceNotice, appearanceToStore, INITIAL_APPEARANCE, readAppearance, writeAppearance } from "./appearance";

const storage = (stored: Record<string, string>) => ({ getItem: (key: string) => stored[key] ?? null });

describe("readAppearance", () => {
  it("reads both axes, a missing one as its initial value", () => {
    expect(readAppearance(storage({ colorFamily: "solarized", colorMode: "dark" }))).toMatchObject({
      appearance: { family: "solarized", mode: "dark" },
      state: "read",
    });
    expect(readAppearance(storage({ colorFamily: "solarized" })).appearance).toEqual({
      family: "solarized",
      mode: "auto",
    });
    expect(readAppearance(storage({})).appearance).toEqual(INITIAL_APPEARANCE);
  });

  it("draws the initial choice for a value this version lacks, as index.html does", () => {
    const read = readAppearance(storage({ colorFamily: "nord", colorMode: "dark" }));
    expect(read.state).toBe("unrecorded");
    expect(read.appearance).toEqual(INITIAL_APPEARANCE);
  });

  it("draws the initial choice when the storage throws", () => {
    const read = readAppearance({
      getItem: () => {
        throw new Error("SecurityError");
      },
    });
    expect(read.state).toBe("unreadable");
    expect(read.appearance).toEqual(INITIAL_APPEARANCE);
  });
});

describe("writeAppearance", () => {
  it("stores both axes", () => {
    const written: Record<string, string> = {};
    const ok = writeAppearance({ setItem: (key, value) => (written[key] = value) }, { family: "solarized", mode: "light" });
    expect(ok).toBe(true);
    expect(written).toEqual({ colorFamily: "solarized", colorMode: "light" });
  });

  it("answers false when the storage throws", () => {
    const ok = writeAppearance(
      {
        setItem: () => {
          throw new Error("QuotaExceededError");
        },
      },
      INITIAL_APPEARANCE,
    );
    expect(ok).toBe(false);
  });
});

describe("appearanceNotice", () => {
  it("says nothing about a choice that was read", () => {
    expect(appearanceNotice(readAppearance(storage({})))).toBeNull();
  });

  it("names the stored value this version lacks, as 案内", () => {
    const notice = appearanceNotice(readAppearance(storage({ colorFamily: "nord", colorMode: "dark" })));
    expect(notice?.level).toBe("info");
    expect(notice?.words).toContain("「nord」");
    expect(notice?.words).not.toContain("dark");
  });

  it("tells unreadable storage as 進行中の縮退", () => {
    const notice = appearanceNotice(
      readAppearance({
        getItem: () => {
          throw new Error("SecurityError");
        },
      }),
    );
    expect(notice?.level).toBe("degraded");
  });
});

describe("appearanceToStore", () => {
  const solarized = { family: "solarized", mode: "dark" };

  it("writes a choice that differs from the one drawn", () => {
    expect(appearanceToStore(solarized, INITIAL_APPEARANCE, false)).toBe(true);
    expect(appearanceToStore({ ...INITIAL_APPEARANCE, mode: "light" }, INITIAL_APPEARANCE, false)).toBe(true);
  });

  it("leaves an unchanged choice alone, so a value this version lacks stays stored", () => {
    expect(appearanceToStore(INITIAL_APPEARANCE, INITIAL_APPEARANCE, false)).toBe(false);
  });

  it("writes again a choice an earlier save drew but could not store", () => {
    expect(appearanceToStore(solarized, solarized, true)).toBe(true);
  });
});
