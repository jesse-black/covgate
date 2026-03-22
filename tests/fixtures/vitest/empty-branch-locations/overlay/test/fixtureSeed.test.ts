import { describe, expect, it } from "vitest";
import { applyFixtureSeed } from "../src/fixtures/fixtureSeed";

describe("applyFixtureSeed", () => {
  it("returns fixture defaults and persists the provided token", () => {
    expect(applyFixtureSeed(" seeded-token ")).toEqual({
      authority: "https://login.example.test/common",
      clientId: "fixture-client-id",
      token: "seeded-token",
    });
  });
});
