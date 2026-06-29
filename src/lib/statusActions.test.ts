import { describe, expect, it } from "vitest";
import { getClaudePathActionView, getStatusActionView } from "./statusActions";

describe("status action view", () => {
  it("does not show probe loading while pause is pending", () => {
    const view = getStatusActionView({
      running: true,
      pendingAction: "pause",
    });

    expect(view.guardPending).toBe(true);
    expect(view.guardLabel).toBe("停止中");
    expect(view.probePending).toBe(false);
    expect(view.probeLabel).toBe("探测一次");
  });

  it("shows probe loading only for a probe request", () => {
    const view = getStatusActionView({
      running: true,
      pendingAction: "probe",
    });

    expect(view.guardPending).toBe(false);
    expect(view.probePending).toBe(true);
    expect(view.probeLabel).toBe("探测中");
  });

  it("disables status actions without false loading during Claude path tests", () => {
    const view = getStatusActionView({
      running: true,
      pendingAction: "claude_path_test",
    });

    expect(view.guardDisabled).toBe(true);
    expect(view.probeDisabled).toBe(true);
    expect(view.guardPending).toBe(false);
    expect(view.probePending).toBe(false);
  });

  it("keeps Claude path identify and save actions visible with separate loading states", () => {
    expect(getClaudePathActionView("claude_path_test")).toMatchObject({
      identifyLabel: "识别当前路径",
      saveLabel: "保存路径",
      identifyPending: true,
      savePending: false,
    });

    expect(getClaudePathActionView("profile")).toMatchObject({
      identifyLabel: "识别当前路径",
      saveLabel: "保存中",
      identifyPending: false,
      savePending: true,
    });
  });
});
