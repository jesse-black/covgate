import { describe, expect, it } from "vitest";
import { summarize } from "../src/math.js";

describe("summarize", () => {
  it("joins three values", () => {
    expect(summarize(1)).toBe("1,2,3");
  });
});
