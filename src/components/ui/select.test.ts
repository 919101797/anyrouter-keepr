import { describe, expect, it } from "vitest";
import { SELECT_CONTENT_POSITION, SELECT_CONTENT_POSITIONING_CLASS } from "./select";

describe("select content positioning", () => {
  it("uses popper positioning so portal content does not enter normal document flow", () => {
    expect(SELECT_CONTENT_POSITION).toBe("popper");
    expect(SELECT_CONTENT_POSITIONING_CLASS).toContain("fixed");
    expect(SELECT_CONTENT_POSITIONING_CLASS).toContain("min-w-[var(--radix-select-trigger-width)]");
  });
});
