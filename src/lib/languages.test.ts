import { describe, expect, it } from "vitest";
import { AUTO, promptLanguages } from "./languages";

describe("promptLanguages", () => {
  it("leaves an automatic source to the model and translates into Japanese", () => {
    expect(promptLanguages(AUTO, AUTO)).toEqual({ source: null, target: "ja" });
  });

  it("names the languages the selectors chose", () => {
    expect(promptLanguages("ja", "en")).toEqual({ source: "ja", target: "en" });
  });
});
