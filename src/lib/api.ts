import { invoke } from "@tauri-apps/api/core";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import type {
  ActivityBucket,
  AppStatus,
  ClaudeDetectionLog,
  ClaudeInstallation,
  ClaudeRuntimeConfig,
  ProbeEvent,
  ProfileInput,
  StoredProfile,
  UpstreamModelCatalog,
} from "./types";
import type { ClaudeFingerprintSnapshot, ProxyStatus } from "./fingerprint";
import { DEFAULT_PROMPT_TAGS } from "./promptTags";
import { DEFAULT_END_TIME, DEFAULT_START_TIME } from "./timeWindow";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export interface SwitchAllFingerprintsResult {
  fingerprint: ClaudeFingerprintSnapshot;
  proxy: ProxyStatus;
}

const mockEvents: ProbeEvent[] = [
  {
    id: "mock-1",
    profile_id: "default",
    started_at: new Date(Date.now() - 90_000).toISOString(),
    ended_at: new Date(Date.now() - 88_900).toISOString(),
    duration_ms: 1100,
    status: "queue_miss",
    error_kind: "429",
    exit_code: 1,
    base_url: "https://anyrouter.top",
    model: "sonnet[1m]",
    key_summary: "cc_switch:any-github · ANTHROPIC_AUTH_TOKEN · sk-ant-...abc123",
    prompt_summary: "今天天气怎样？一句话答",
    prompt_truncated: false,
    stdout_summary: null,
    stderr_summary: "API Error: 429 status code",
    stdout_truncated: false,
    stderr_truncated: false,
  },
  {
    id: "mock-2",
    profile_id: "default",
    started_at: new Date(Date.now() - 12 * 60_000).toISOString(),
    ended_at: new Date(Date.now() - 12 * 60_000 + 1300).toISOString(),
    duration_ms: 1300,
    status: "success",
    error_kind: null,
    exit_code: 0,
    base_url: "https://anyrouter.top",
    model: "sonnet[1m]",
    key_summary: "cc_switch:any-github · ANTHROPIC_AUTH_TOKEN · sk-ant-...abc123",
    prompt_summary: "用一句话讲个笑话",
    prompt_truncated: false,
    stdout_summary: '{"result":"OK"}',
    stderr_summary: null,
    stdout_truncated: false,
    stderr_truncated: false,
  },
];

const mockProfile: StoredProfile = {
  id: "default",
  name: "AnyRouter",
  claude_binary_path: "",
  base_url: "",
  token_kind: "ANTHROPIC_AUTH_TOKEN",
  model: "",
  effort: "low",
  context_size: "1m",
  prompt: "只回复 OK",
  prompt_pool: DEFAULT_PROMPT_TAGS,
  min_interval_seconds: 60,
  max_interval_seconds: 120,
  timeout_seconds: 90,
  start_time: DEFAULT_START_TIME,
  end_time: DEFAULT_END_TIME,
  enabled: false,
  prevent_sleep: true,
  stdout_summary_limit_bytes: 2048,
  stderr_summary_limit_bytes: 2048,
  event_flush_count: 5,
  event_flush_interval_seconds: 300,
  history_retention_days: 30,
  max_events_per_profile: 50_000,
  max_database_size_mb: 50,
  has_token: false,
};

const mockClaudeInstallation: ClaudeInstallation = {
  checked_at: new Date().toISOString(),
  configured_path: "",
  detected_path: "/usr/local/bin/claude",
  effective_path: "/usr/local/bin/claude",
  version: "2.1.170 (Claude Code)",
  source: "path",
  status: "ready",
  error: null,
};

const mockClaudeRuntimeConfig: ClaudeRuntimeConfig = {
  checked_at: new Date().toISOString(),
  default_model: "claude-opus-4-8",
  model_source: "cc_switch:recent_request_model",
  default_effort: "high",
  effort_source: "cc_switch:effort:anyrouter-git",
  error: null,
};

const mockUpstreamModelCatalog: UpstreamModelCatalog = {
  checked_at: new Date().toISOString(),
  upstream_url: "https://anyrouter.top",
  upstream_source: "cc_switch:mock",
  models: [
    { id: "claude-opus-4-8", display_name: "Claude Opus 4.8" },
    { id: "claude-sonnet-4-6", display_name: "Claude Sonnet 4.6" },
  ],
  error: null,
};

const mockClaudeDetectionLogs: ClaudeDetectionLog[] = [
  {
    id: "mock-claude-log-1",
    ...mockClaudeInstallation,
    checked_at: new Date(Date.now() - 45_000).toISOString(),
  },
];

const mockClaudeFingerprintSnapshot: ClaudeFingerprintSnapshot = {
  current: {
    checked_at: new Date().toISOString(),
    claude_state_path: "/Users/mock/.claude.json",
    stainless_os: "MacOS",
    stainless_arch: "arm64",
    device_id: "5a60043ffe0e04ff78da4ed3ebebbb4aa9e263b5417f595270b1c646534bf421",
    device_id_status: "present",
    session_id_status: "runtime_generated_by_claude_code",
    risk_label: "AnyRouter 1M routing key: OS + Arch + device_id",
  },
  history: [
    {
      id: "mock-fingerprint-1",
      captured_at: new Date(Date.now() - 4 * 60_000).toISOString(),
      source: "generated",
      claude_state_path: "/Users/mock/.claude.json",
      stainless_os: "MacOS",
      stainless_arch: "arm64",
      device_id: "5a60043ffe0e04ff78da4ed3ebebbb4aa9e263b5417f595270b1c646534bf421",
      device_id_status: "present",
      session_id_status: "runtime_generated_by_claude_code",
      risk_label: "AnyRouter 1M routing key: OS + Arch + device_id",
    },
    {
      id: "mock-fingerprint-2",
      captured_at: new Date(Date.now() - 18 * 60_000).toISOString(),
      source: "captured_before_regenerate",
      claude_state_path: "/Users/mock/.claude.json",
      stainless_os: "Windows",
      stainless_arch: "x64",
      device_id: "1b750947616d19fc1b8bba20b8024351b1a61012ea72c38b99d33071f4e3a74b",
      device_id_status: "present",
      session_id_status: "runtime_generated_by_claude_code",
      risk_label: "AnyRouter 1M routing key: OS + Arch + device_id",
    },
  ],
  error: null,
};

export const api = {
  async getProfile(): Promise<StoredProfile> {
    if (!inTauri) return mockProfile;
    return invoke("get_profile");
  },

  async saveProfile(profile: ProfileInput): Promise<StoredProfile> {
    if (!inTauri) return { ...profile, has_token: Boolean(profile.token) };
    return invoke("save_profile", { profile });
  },

  async getClaudeInstallation(): Promise<ClaudeInstallation> {
    if (!inTauri) return mockClaudeInstallation;
    return invoke("get_claude_installation");
  },

  async getClaudeRuntimeConfig(): Promise<ClaudeRuntimeConfig> {
    if (!inTauri) return mockClaudeRuntimeConfig;
    return invoke("get_claude_runtime_config");
  },

  async getUpstreamModels(): Promise<UpstreamModelCatalog> {
    if (!inTauri) return { ...mockUpstreamModelCatalog, checked_at: new Date().toISOString() };
    return invoke("get_upstream_models");
  },

  async getClaudeKeyValue(keySummary?: string | null): Promise<string | null> {
    if (!inTauri) return "sk-ant-mock-copied-key";
    return invoke("get_claude_key_value", { keySummary: keySummary || null });
  },

  async refreshClaudeInstallation(): Promise<ClaudeInstallation> {
    if (!inTauri) return { ...mockClaudeInstallation, checked_at: new Date().toISOString() };
    return invoke("refresh_claude_installation");
  },

  async testClaudeInstallation(configuredPath: string): Promise<ClaudeInstallation> {
    if (!inTauri) {
      const trimmed = configuredPath.trim();
      const isDirectory = trimmed.endsWith("/bin") || trimmed.endsWith("\\bin");
      return {
        ...mockClaudeInstallation,
        checked_at: new Date().toISOString(),
        configured_path: trimmed,
        detected_path: null,
        effective_path: isDirectory ? `${trimmed}/claude` : trimmed || mockClaudeInstallation.effective_path,
        source: trimmed ? "manual" : "path",
      };
    }
    return invoke("test_claude_installation", { configuredPath });
  },

  async listClaudeDetectionLogs(limit = 20): Promise<ClaudeDetectionLog[]> {
    if (!inTauri) return mockClaudeDetectionLogs;
    return invoke("list_claude_detection_logs", { limit });
  },

  async getClaudeFingerprintSnapshot(): Promise<ClaudeFingerprintSnapshot> {
    if (!inTauri) return mockClaudeFingerprintSnapshot;
    return invoke("get_claude_fingerprint_snapshot");
  },

  async regenerateClaudeFingerprint(): Promise<ClaudeFingerprintSnapshot> {
    if (!inTauri) {
      return {
        ...mockClaudeFingerprintSnapshot,
        current: {
          ...mockClaudeFingerprintSnapshot.current!,
          checked_at: new Date().toISOString(),
          device_id: "cff2afa3147b17a437d6a02b8d1e83d33bcc619e52d43172fdf3e15b03b6d2e8",
        },
      };
    }
    return invoke("regenerate_claude_fingerprint");
  },

  async restoreClaudeFingerprint(id: string): Promise<ClaudeFingerprintSnapshot> {
    if (!inTauri) {
      const selected = mockClaudeFingerprintSnapshot.history.find((entry) => entry.id === id);
      return {
        ...mockClaudeFingerprintSnapshot,
        current: selected
          ? {
              checked_at: new Date().toISOString(),
              claude_state_path: selected.claude_state_path,
              stainless_os: selected.stainless_os,
              stainless_arch: selected.stainless_arch,
              device_id: selected.device_id,
              device_id_status: selected.device_id_status,
              session_id_status: selected.session_id_status,
              risk_label: selected.risk_label,
            }
          : mockClaudeFingerprintSnapshot.current,
      };
    }
    return invoke("restore_claude_fingerprint", { id });
  },

  async deleteClaudeFingerprintHistory(id: string): Promise<ClaudeFingerprintSnapshot> {
    if (!inTauri) {
      return {
        ...mockClaudeFingerprintSnapshot,
        history: mockClaudeFingerprintSnapshot.history.filter((entry) => entry.id !== id),
      };
    }
    return invoke("delete_claude_fingerprint_history", { id });
  },

  async getProxyStatus(): Promise<ProxyStatus> {
    if (!inTauri)
      return {
        running: false,
        listen_port: 15800,
        target_os: "Windows",
        target_arch: "x64",
        upstream_url: "https://anyrouter.top",
        dynamic_upstream: true,
        error: null,
      };
    return invoke("get_proxy_status");
  },

  async startProxy(): Promise<ProxyStatus> {
    if (!inTauri)
      return {
        running: true,
        listen_port: 15800,
        target_os: "Windows",
        target_arch: "x64",
        upstream_url: "https://anyrouter.top",
        dynamic_upstream: true,
        error: null,
      };
    return invoke("start_proxy");
  },

  async stopProxy(): Promise<ProxyStatus> {
    if (!inTauri)
      return {
        running: false,
        listen_port: 15800,
        target_os: "Windows",
        target_arch: "x64",
        upstream_url: "https://anyrouter.top",
        dynamic_upstream: true,
        error: null,
      };
    return invoke("stop_proxy");
  },

  async setProxyTarget(targetOs: string, targetArch: string): Promise<ProxyStatus> {
    if (!inTauri)
      return {
        running: false,
        listen_port: 15800,
        target_os: targetOs,
        target_arch: targetArch,
        upstream_url: "https://anyrouter.top",
        dynamic_upstream: true,
        error: null,
      };
    return invoke("set_proxy_target", { targetOs, targetArch });
  },

  async switchAllFingerprints(): Promise<SwitchAllFingerprintsResult> {
    if (!inTauri) {
      const deviceId = crypto.randomUUID().replaceAll("-", "") + crypto.randomUUID().replaceAll("-", "");
      return {
        fingerprint: {
          ...mockClaudeFingerprintSnapshot,
          current: {
            ...mockClaudeFingerprintSnapshot.current!,
            checked_at: new Date().toISOString(),
            device_id: deviceId.slice(0, 64),
          },
          history: [
            {
              id: `mock-fingerprint-${Date.now()}`,
              captured_at: new Date().toISOString(),
              source: "generated",
              claude_state_path: mockClaudeFingerprintSnapshot.current!.claude_state_path,
              stainless_os: mockClaudeFingerprintSnapshot.current!.stainless_os,
              stainless_arch: mockClaudeFingerprintSnapshot.current!.stainless_arch,
              device_id: deviceId.slice(0, 64),
              device_id_status: "present",
              session_id_status: "runtime_generated_by_claude_code",
              risk_label: mockClaudeFingerprintSnapshot.current!.risk_label,
            },
            ...mockClaudeFingerprintSnapshot.history,
          ],
        },
        proxy: {
          running: true,
          listen_port: 15800,
          target_os: "Windows",
          target_arch: "x64",
          upstream_url: "https://anyrouter.top",
          dynamic_upstream: true,
          error: null,
        },
      };
    }
    return invoke("switch_all_fingerprints");
  },

  async runProbeNow(): Promise<ProbeEvent> {
    if (!inTauri) return mockEvents[0];
    return invoke("run_probe_now");
  },

  async startScheduler(): Promise<void> {
    if (!inTauri) return;
    return invoke("start_scheduler");
  },

  async pauseScheduler(): Promise<void> {
    if (!inTauri) return;
    return invoke("pause_scheduler");
  },

  async getCurrentStatus(): Promise<AppStatus> {
    if (!inTauri) {
      return {
        profile_id: "default",
        running: false,
        current_state: "racing",
        last_event: mockEvents[0],
        last_success_at: mockEvents[1].started_at,
        consecutive_queue_miss: 3,
        next_probe_at: new Date(Date.now() + 64_000).toISOString(),
        in_window: true,
      };
    }
    return invoke("get_current_status");
  },

  async listProbeEvents(limit = 200, status?: string): Promise<ProbeEvent[]> {
    if (!inTauri) return mockEvents;
    return invoke("list_probe_events", { limit, status: status || null });
  },

  async getActivitySummary(hours = 24): Promise<ActivityBucket[]> {
    if (!inTauri) {
      const buckets: ActivityBucket[] = [];
      for (let index = 0; index < 96; index += 1) {
        const time = new Date(Date.now() - (96 - index) * 15 * 60_000);
        const mod = index % 9;
        buckets.push({
          bucket_start: time.toISOString(),
          success_count: mod === 0 ? 1 : 0,
          queue_miss_count: mod > 0 && mod < 6 ? 1 : 0,
          timeout_count: mod === 6 ? 1 : 0,
          config_error_count: 0,
          unknown_count: mod === 7 ? 1 : 0,
          last_status: mod === 0 ? "success" : mod === 6 ? "timeout" : "queue_miss",
          last_latency_ms: 800 + index * 8,
        });
      }
      return buckets;
    }
    return invoke("get_activity_summary", { hours });
  },

  async isAutostartEnabled(): Promise<boolean> {
    if (!inTauri) return false;
    return isEnabled();
  },

  async setAutostart(enabled: boolean): Promise<void> {
    if (!inTauri) return;
    if (enabled) {
      await enable();
    } else {
      await disable();
    }
  },
};
