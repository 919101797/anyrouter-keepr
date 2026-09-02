import { describe, expect, it } from "vitest";

import {
  CUSTOM_MODEL_VALUE,
  canonicalModelValue,
  modelDisplayName,
  modelSelectValue,
  runtimeModelSelectValue,
} from "./modelOptions";
import type { UpstreamModel } from "./types";

const upstreamModels: UpstreamModel[] = [
  { id: "claude-future-6", display_name: "Claude Future 6" },
  { id: "gateway-model", display_name: "Gateway Model" },
];

describe("dynamic upstream model options", () => {
  it("recognizes a model returned by the upstream without a release-time catalog", () => {
    expect(modelSelectValue("claude-future-6", upstreamModels)).toBe("claude-future-6");
    expect(modelDisplayName("claude-future-6", upstreamModels)).toBe("Claude Future 6");
  });

  it("matches a selected model after removing its context suffix", () => {
    expect(canonicalModelValue("claude-future-6[1m]")).toBe("claude-future-6");
    expect(modelSelectValue("claude-future-6[1m]", upstreamModels)).toBe("claude-future-6");
  });

  it("keeps models missing from the current upstream response in custom mode", () => {
    expect(modelSelectValue("private-alias", upstreamModels)).toBe(CUSTOM_MODEL_VALUE);
    expect(modelDisplayName("private-alias", upstreamModels)).toBe("private-alias");
  });

  it("uses the detected Claude Code model when no override is configured", () => {
    expect(runtimeModelSelectValue("", "gateway-model", upstreamModels)).toBe("gateway-model");
  });
});
