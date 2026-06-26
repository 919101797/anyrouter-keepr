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
} from "./types";
import { DEFAULT_PROMPT_TAGS } from "./promptTags";

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

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
  start_time: "05:00",
  end_time: "24:00",
  enabled: false,
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

const mockClaudeDetectionLogs: ClaudeDetectionLog[] = [
  {
    id: "mock-claude-log-1",
    ...mockClaudeInstallation,
    checked_at: new Date(Date.now() - 45_000).toISOString(),
  },
];

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

  async refreshClaudeInstallation(): Promise<ClaudeInstallation> {
    if (!inTauri) return { ...mockClaudeInstallation, checked_at: new Date().toISOString() };
    return invoke("refresh_claude_installation");
  },

  async listClaudeDetectionLogs(limit = 20): Promise<ClaudeDetectionLog[]> {
    if (!inTauri) return mockClaudeDetectionLogs;
    return invoke("list_claude_detection_logs", { limit });
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
