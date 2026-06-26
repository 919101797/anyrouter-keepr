import { describe, expect, it } from "vitest";
import { DEFAULT_PROMPT_TAGS, MAX_PROMPT_TAGS, normalizePromptTags, promptTagsForUi } from "./promptTags";

describe("prompt tag helpers", () => {
  it("normalizes prompt tags for storage", () => {
    const tags = normalizePromptTags([" ok ", "", "ping", "ok", "hi"]);

    expect(tags).toEqual(["ok", "ping", "hi"]);
  });

  it("upgrades the legacy default prompt pool", () => {
    expect(promptTagsForUi(["只回复 OK", "请只回复 OK", "hi", "ping", "请回复 ready"])).toEqual(
      DEFAULT_PROMPT_TAGS,
    );
  });

  it("upgrades the short default prompt pool", () => {
    expect(
      promptTagsForUi([
        "ok",
        "hi",
        "ping",
        "pong",
        "ack",
        "yes",
        "go",
        "up",
        "on",
        "run",
        "rdy",
        "chk",
        "stat",
        "live",
        "beat",
        "tick",
        "tap",
        "echo",
        "noop",
        "test",
        "mark",
        "trace",
        "node",
        "edge",
        "route",
        "gw",
        "api",
        "cc",
        "ar",
        "keep",
        "pulse",
        "warm",
        "wake",
        "link",
        "path",
        "hold",
        "sync",
        "green",
        "ready",
        "ok?",
        "ping?",
        "1",
        "2",
        "3",
      ]),
    ).toEqual(DEFAULT_PROMPT_TAGS);
  });

  it("keeps the prompt tag pool bounded", () => {
    const tags = normalizePromptTags(Array.from({ length: MAX_PROMPT_TAGS + 3 }, (_, index) => `t${index}`));

    expect(tags).toHaveLength(MAX_PROMPT_TAGS);
  });
});
