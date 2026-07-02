import { describe, expect, it } from "vitest";
import {
  fingerprintSourceLabel,
  fingerprintsMatch,
  chooseNextFingerprintTarget,
  fingerprintItemDetail,
  shortDeviceId,
  type ClaudeFingerprint,
  type ClaudeFingerprintHistoryEntry,
  type ProxyStatus,
} from "./fingerprint";

const current: ClaudeFingerprint = {
  checked_at: "2026-07-01T09:00:00+08:00",
  claude_state_path: "/Users/test/.claude.json",
  stainless_os: "MacOS",
  stainless_arch: "arm64",
  device_id: "5a60043ffe0e04ff78da4ed3ebebbb4aa9e263b5417f595270b1c646534bf421",
  device_id_status: "present",
  session_id_status: "runtime_generated_by_claude_code",
  risk_label: "AnyRouter 1M routing key: OS + Arch + device_id",
};

const history: ClaudeFingerprintHistoryEntry = {
  id: "history-1",
  captured_at: "2026-07-01T09:01:00+08:00",
  source: "generated",
  claude_state_path: "/Users/test/.claude.json",
  stainless_os: "MacOS",
  stainless_arch: "arm64",
  device_id: current.device_id,
  device_id_status: "present",
  session_id_status: "runtime_generated_by_claude_code",
  risk_label: "AnyRouter 1M routing key: OS + Arch + device_id",
};

describe("fingerprint helpers", () => {
  it("formats long device ids for dense UI", () => {
    expect(shortDeviceId(current.device_id)).toBe("5a60043f...534bf421");
  });

  it("labels history sources in Chinese", () => {
    expect(fingerprintSourceLabel("captured_before_regenerate")).toBe("切换前保存");
    expect(fingerprintSourceLabel("generated")).toBe("重新生成");
    expect(fingerprintSourceLabel("restored")).toBe("已恢复");
    expect(fingerprintSourceLabel("unknown")).toBe("unknown");
  });

  it("detects whether a history row is the active fingerprint", () => {
    expect(fingerprintsMatch(current, history)).toBe(true);
    expect(fingerprintsMatch(current, { ...history, stainless_arch: "x64" })).toBe(false);
  });

  it("chooses a different target from the current proxy target", () => {
    expect(chooseNextFingerprintTarget("Windows", "x64", () => 0)).toEqual({
      os: "Windows",
      arch: "arm64",
    });
  });

  it("shows target fingerprint values even before the proxy starts", () => {
    const proxyStatus: ProxyStatus = {
      running: false,
      listen_port: 15800,
      target_os: "Windows",
      target_arch: "x64",
      upstream_url: "https://anyrouter.top",
      error: null,
    };

    expect(fingerprintItemDetail("x-stainless-os", current, proxyStatus)).toBe(
      "目标: Windows（未启动，本机: MacOS）",
    );
    expect(fingerprintItemDetail("x-stainless-arch", current, proxyStatus)).toBe(
      "目标: x64（未启动，本机: arm64）",
    );
  });
});
