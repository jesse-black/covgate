import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { ProfileCard } from "../src/profileCard";

describe("ProfileCard", () => {
  it("renders the basic summary", () => {
    const markup = renderToStaticMarkup(
      <ProfileCard
        profile={{
          name: "Ada",
          email: "ada@example.com",
          tags: ["admin", "editor"]
        }}
        showEmail
      />
    );

    expect(markup).toContain("Ada");
    expect(markup).toContain("admin / editor");
  });
});
