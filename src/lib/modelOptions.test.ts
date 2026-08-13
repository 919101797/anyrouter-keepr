import { describe, expect, it } from "vitest";

import {
  CLAUDE_MODEL_OPTIONS,
  canonicalModelValue,
  modelDisplayName,
  modelSelectValue,
} from "./modelOptions";

describe("Claude model options", () => {
  it("offers the Claude 5 Sonnet and Opus models", () => {
    expect(CLAUDE_MODEL_OPTIONS).toEqual(
      expect.arrayContaining([
        { value: "claude-sonnet-5", label: "Claude Sonnet 5" },
        { value: "claude-opus-5", label: "Claude Opus 5" },
      ]),
    );
  });

  it.each([
    ["claude-sonnet-5", "Claude Sonnet 5"],
    ["claude-opus-5[1m]", "Claude Opus 5"],
    ["claude-sonnet-5-20260813", "Claude Sonnet 5"],
  ])("recognizes %s as a built-in model", (model, label) => {
    expect(modelSelectValue(model)).not.toBe("__custom_model__");
    expect(modelDisplayName(model)).toBe(label);
  });

  it("canonicalizes dated Claude 5 model identifiers", () => {
    expect(canonicalModelValue("claude-opus-5-20260813")).toBe("claude-opus-5");
  });
});
