import { describe, expect, it } from "vitest";
import indexHtml from "../index.html?raw";

interface ColorScheme {
  choose: (family: string, mode: string) => void;
}

const script = indexHtml.match(/<script id="color-scheme">([\s\S]*?)<\/script>/)?.[1];

interface Run {
  attrs: Map<string, string>;
  choose: (family: string, mode: string) => void;
  osReads: () => number;
  writes: string[];
  setOsDark: (dark: boolean) => void;
}

function boot(stored: Record<string, string> | "unreadable", osDark: boolean): Run {
  if (script === undefined) {
    throw new Error('index.html has no <script id="color-scheme">');
  }
  const attrs = new Map<string, string>();
  const writes: string[] = [];
  const listeners: (() => void)[] = [];
  let dark = osDark;
  let osReads = 0;
  const query = {
    get matches() {
      osReads += 1;
      return dark;
    },
    addEventListener: (type: string, listener: () => void) => {
      if (type === "change") {
        listeners.push(listener);
      }
    },
  };
  const localStorage = {
    getItem: (key: string) => {
      if (stored === "unreadable") {
        throw new Error("SecurityError");
      }
      return stored[key] ?? null;
    },
    setItem: (key: string) => {
      writes.push(key);
    },
    removeItem: (key: string) => {
      writes.push(key);
    },
  };
  const window: { localStorage: typeof localStorage; matchMedia: () => typeof query; tinyColorScheme?: ColorScheme } = {
    localStorage,
    matchMedia: () => query,
  };
  const document = { documentElement: { setAttribute: (name: string, value: string) => attrs.set(name, value) } };
  new Function("window", "document", script)(window, document);
  const chosen = window.tinyColorScheme;
  if (chosen === undefined) {
    throw new Error("the script did not hand the settings screen its way to choose");
  }
  return {
    attrs,
    choose: (family, mode) => chosen.choose(family, mode),
    osReads: () => osReads,
    writes,
    setOsDark: (next) => {
      dark = next;
      for (const listener of listeners) {
        listener();
      }
    },
  };
}

const drawn = (run: Run) => [run.attrs.get("data-color-family"), run.attrs.get("data-color-mode")];

describe("the color-scheme script in index.html", () => {
  it("draws standard following the OS when nothing is stored", () => {
    expect(drawn(boot({}, false))).toEqual(["standard", "light"]);
    expect(drawn(boot({}, true))).toEqual(["standard", "dark"]);
  });

  it("draws every stored pair of the two axes", () => {
    for (const family of ["standard", "solarized"]) {
      expect(drawn(boot({ colorFamily: family, colorMode: "light" }, true))).toEqual([family, "light"]);
      expect(drawn(boot({ colorFamily: family, colorMode: "dark" }, false))).toEqual([family, "dark"]);
      expect(drawn(boot({ colorFamily: family, colorMode: "auto" }, true))).toEqual([family, "dark"]);
      expect(drawn(boot({ colorFamily: family, colorMode: "auto" }, false))).toEqual([family, "light"]);
    }
  });

  it("reads a missing key as that axis's initial value", () => {
    expect(drawn(boot({ colorFamily: "solarized" }, true))).toEqual(["solarized", "dark"]);
    expect(drawn(boot({ colorMode: "light" }, true))).toEqual(["standard", "light"]);
  });

  it("draws the initial choice when either axis holds a value this app does not have", () => {
    expect(drawn(boot({ colorFamily: "nord", colorMode: "light" }, true))).toEqual(["standard", "dark"]);
    expect(drawn(boot({ colorFamily: "solarized", colorMode: "dim" }, false))).toEqual(["standard", "light"]);
  });

  it("draws the initial choice when storage cannot be read", () => {
    expect(drawn(boot("unreadable", true))).toEqual(["standard", "dark"]);
  });

  it("never writes to storage", () => {
    expect(boot({ colorFamily: "nord", colorMode: "dim" }, true).writes).toEqual([]);
    expect(boot({}, false).writes).toEqual([]);
  });

  it("follows OS changes while the choice is auto", () => {
    const run = boot({ colorFamily: "solarized", colorMode: "auto" }, false);
    run.setOsDark(true);
    expect(drawn(run)).toEqual(["solarized", "dark"]);
    run.setOsDark(false);
    expect(drawn(run)).toEqual(["solarized", "light"]);
  });

  it("does not consult the OS when light or dark is chosen", () => {
    const run = boot({ colorMode: "light" }, true);
    run.setOsDark(false);
    run.setOsDark(true);
    expect(drawn(run)).toEqual(["standard", "light"]);
    expect(run.osReads()).toBe(0);
  });

  it("puts a choice made after launch into effect, and follows the OS by it", () => {
    const run = boot({ colorMode: "auto" }, true);
    run.choose("solarized", "light");
    expect(drawn(run)).toEqual(["solarized", "light"]);
    run.setOsDark(false);
    run.setOsDark(true);
    expect(drawn(run)).toEqual(["solarized", "light"]);
    run.choose("standard", "auto");
    expect(drawn(run)).toEqual(["standard", "dark"]);
    run.setOsDark(false);
    expect(drawn(run)).toEqual(["standard", "light"]);
    expect(run.writes).toEqual([]);
  });
});
