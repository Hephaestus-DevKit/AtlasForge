import { describe, expect, it } from "vitest";

describe("AtlasForge smoke test", () => {
  it("keeps the test runner wired", () => {
    expect("AtlasForge").toContain("Forge");
  });
});
