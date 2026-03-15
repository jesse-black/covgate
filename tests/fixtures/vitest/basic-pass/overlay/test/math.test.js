import { describe, expect, it } from "vitest";
import { adjust } from "../src/math.js";

describe("adjust", () => {
  it("handles non-negative values", () => {
    expect(adjust(1)).toBe(2);
  });

  it("handles negative values", () => {
    expect(adjust(-1)).toBe(-2);
  });
});
