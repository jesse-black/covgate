import { describe, expect, it } from "vitest";
import { adjust } from "../src/math.js";

describe("adjust", () => {
  it("increments positive numbers", () => {
    expect(adjust(1)).toBe(2);
  });
});
