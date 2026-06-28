export type ProbeStatus = "success" | "queue_miss" | "timeout" | "config_error" | "unknown";

export interface StoredProfile {
  id: string;
  name: string;
  claude_binary_path: string;
  base_url: string;
  token_kind: string;
  model: string;
  effort: string;
  context_size: string;
  prompt: string;
  prompt_pool: string[];
  min_interval_seconds: number;
  max_interval_seconds: number;
  timeout_seconds: number;
  start_time: string;
  end_time: string;
  enabled: boolean;
  prevent_sleep: boolean;
  stdout_summary_limit_bytes: number;
  stderr_summary_limit_bytes: number;
  event_flush_count: number;
  event_flush_interval_seconds: number;
  history_retention_days: number;
  max_events_per_profile: number;
  max_database_size_mb: number;
  has_token: boolean;
}

export interface ProfileInput extends Omit<StoredProfile, "has_token"> {
  token?: string | null;
}

export interface ProbeEvent {
  id: string;
  profile_id: string;
  started_at: string;
  ended_at: string;
  duration_ms: number;
  status: ProbeStatus;
  error_kind?: string | null;
  exit_code?: number | null;
  base_url: string;
  model: string;
  key_summary?: string | null;
  prompt_summary?: string | null;
  prompt_truncated: boolean;
  stdout_summary?: string | null;
  stderr_summary?: string | null;
  stdout_truncated: boolean;
  stderr_truncated: boolean;
}

export interface AppStatus {
  profile_id: string;
  running: boolean;
  current_state:
    | "connected"
    | "racing"
    | "config_error"
    | "paused"
    | "running"
    | "sleeping"
    | "probing"
    | "unknown";
  last_event?: ProbeEvent | null;
  last_success_at?: string | null;
  consecutive_queue_miss: number;
  next_probe_at?: string | null;
  in_window: boolean;
}

export interface ActivityBucket {
  bucket_start: string;
  success_count: number;
  queue_miss_count: number;
  timeout_count: number;
  config_error_count: number;
  unknown_count: number;
  last_status?: ProbeStatus | null;
  last_latency_ms?: number | null;
}

export interface ClaudeInstallation {
  checked_at: string;
  configured_path: string;
  detected_path?: string | null;
  effective_path?: string | null;
  version?: string | null;
  source: "manual" | "path" | string;
  status: "ready" | "not_found" | "invalid" | string;
  error?: string | null;
}

export interface ClaudeRuntimeConfig {
  checked_at: string;
  default_model?: string | null;
  model_source: string;
  default_effort?: string | null;
  effort_source: string;
  error?: string | null;
}

export interface ClaudeDetectionLog extends ClaudeInstallation {
  id: string;
}
