use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Success,
    QueueMiss,
    Timeout,
    ConfigError,
    Unknown,
}

impl ProbeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProbeStatus::Success => "success",
            ProbeStatus::QueueMiss => "queue_miss",
            ProbeStatus::Timeout => "timeout",
            ProbeStatus::ConfigError => "config_error",
            ProbeStatus::Unknown => "unknown",
        }
    }

    pub fn parse_status(value: &str) -> Self {
        match value {
            "success" => ProbeStatus::Success,
            "queue_miss" => ProbeStatus::QueueMiss,
            "timeout" => ProbeStatus::Timeout,
            "config_error" => ProbeStatus::ConfigError,
            _ => ProbeStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub claude_binary_path: String,
    pub base_url: String,
    pub token: Option<String>,
    pub token_kind: String,
    pub model: String,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default = "default_context_size")]
    pub context_size: String,
    pub prompt: String,
    #[serde(default = "default_prompt_pool")]
    pub prompt_pool: Vec<String>,
    pub min_interval_seconds: u64,
    pub max_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub start_time: String,
    pub end_time: String,
    pub enabled: bool,
    pub stdout_summary_limit_bytes: usize,
    pub stderr_summary_limit_bytes: usize,
    pub event_flush_count: usize,
    pub event_flush_interval_seconds: u64,
    pub history_retention_days: i64,
    pub max_events_per_profile: i64,
    pub max_database_size_mb: i64,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: "AnyRouter".to_string(),
            claude_binary_path: "".to_string(),
            base_url: "".to_string(),
            token: None,
            token_kind: "ANTHROPIC_AUTH_TOKEN".to_string(),
            model: "".to_string(),
            effort: default_effort(),
            context_size: default_context_size(),
            prompt: "只回复 OK".to_string(),
            prompt_pool: default_prompt_pool(),
            min_interval_seconds: 60,
            max_interval_seconds: 120,
            timeout_seconds: 90,
            start_time: "05:00".to_string(),
            end_time: "24:00".to_string(),
            enabled: false,
            stdout_summary_limit_bytes: 2048,
            stderr_summary_limit_bytes: 2048,
            event_flush_count: 5,
            event_flush_interval_seconds: 300,
            history_retention_days: 30,
            max_events_per_profile: 50_000,
            max_database_size_mb: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub claude_binary_path: String,
    pub base_url: String,
    pub token_kind: String,
    pub model: String,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default = "default_context_size")]
    pub context_size: String,
    pub prompt: String,
    #[serde(default = "default_prompt_pool")]
    pub prompt_pool: Vec<String>,
    pub min_interval_seconds: u64,
    pub max_interval_seconds: u64,
    pub timeout_seconds: u64,
    pub start_time: String,
    pub end_time: String,
    pub enabled: bool,
    pub stdout_summary_limit_bytes: usize,
    pub stderr_summary_limit_bytes: usize,
    pub event_flush_count: usize,
    pub event_flush_interval_seconds: u64,
    pub history_retention_days: i64,
    pub max_events_per_profile: i64,
    pub max_database_size_mb: i64,
    pub has_token: bool,
}

impl From<Profile> for StoredProfile {
    fn from(profile: Profile) -> Self {
        Self {
            id: profile.id,
            name: profile.name,
            claude_binary_path: profile.claude_binary_path,
            base_url: profile.base_url,
            token_kind: profile.token_kind,
            model: profile.model,
            effort: profile.effort,
            context_size: profile.context_size,
            prompt: profile.prompt,
            prompt_pool: profile.prompt_pool,
            min_interval_seconds: profile.min_interval_seconds,
            max_interval_seconds: profile.max_interval_seconds,
            timeout_seconds: profile.timeout_seconds,
            start_time: profile.start_time,
            end_time: profile.end_time,
            enabled: profile.enabled,
            stdout_summary_limit_bytes: profile.stdout_summary_limit_bytes,
            stderr_summary_limit_bytes: profile.stderr_summary_limit_bytes,
            event_flush_count: profile.event_flush_count,
            event_flush_interval_seconds: profile.event_flush_interval_seconds,
            history_retention_days: profile.history_retention_days,
            max_events_per_profile: profile.max_events_per_profile,
            max_database_size_mb: profile.max_database_size_mb,
            has_token: profile.token.is_some(),
        }
    }
}

fn default_effort() -> String {
    "low".to_string()
}

fn default_context_size() -> String {
    "1m".to_string()
}

pub fn default_prompt_pool() -> Vec<String> {
    [
        "用一句话讲个笑话",
        "今天天气怎样？一句话答",
        "给我一个英文单词",
        "用十字内问候我",
        "给一句早安文案",
        "给一句晚安文案",
        "说一个冷知识",
        "推荐一个变量名",
        "给一个函数名建议",
        "生成一个短标题",
        "写一句状态提示",
        "给一个测试用例名",
        "用一句话总结日志",
        "给一句鼓励的话",
        "给一个颜色名称",
        "用一句话解释缓存",
        "用一句话解释队列",
        "用一句话解释重试",
        "用一句话解释超时",
        "用一句话解释 429",
        "给一个接口名称",
        "写一句错误提示",
        "给一个英文短句",
        "用一句话解释令牌",
        "用一句话解释代理",
        "给一个分支名",
        "写一句按钮文案",
        "给一个字段名",
        "讲一个双关笑话",
        "给一句产品提示",
        "用一句话解释心跳",
        "给一个任务标题",
        "写一句空状态文案",
        "给一个状态标签",
        "用一句话解释限流",
        "给一个命名建议",
        "写一句加载文案",
        "给一句周五问候",
        "用一句话解释守护",
        "给一个短别名",
        "写一句成功提示",
        "写一句失败提示",
        "用一句话解释路由",
        "给一个轻量回复",
    ]
    .into_iter()
    .map(ToString::to_string)
    .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeEvent {
    pub id: String,
    pub profile_id: String,
    pub started_at: DateTime<Local>,
    pub ended_at: DateTime<Local>,
    pub duration_ms: i64,
    pub status: ProbeStatus,
    pub error_kind: Option<String>,
    pub exit_code: Option<i32>,
    pub base_url: String,
    pub model: String,
    pub prompt_summary: Option<String>,
    pub prompt_truncated: bool,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeEventDto {
    pub id: String,
    pub profile_id: String,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
    pub status: ProbeStatus,
    pub error_kind: Option<String>,
    pub exit_code: Option<i32>,
    pub base_url: String,
    pub model: String,
    pub prompt_summary: Option<String>,
    pub prompt_truncated: bool,
    pub stdout_summary: Option<String>,
    pub stderr_summary: Option<String>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

impl From<ProbeEvent> for ProbeEventDto {
    fn from(event: ProbeEvent) -> Self {
        Self {
            id: event.id,
            profile_id: event.profile_id,
            started_at: event.started_at.to_rfc3339(),
            ended_at: event.ended_at.to_rfc3339(),
            duration_ms: event.duration_ms,
            status: event.status,
            error_kind: event.error_kind,
            exit_code: event.exit_code,
            base_url: event.base_url,
            model: event.model,
            prompt_summary: event.prompt_summary,
            prompt_truncated: event.prompt_truncated,
            stdout_summary: event.stdout_summary,
            stderr_summary: event.stderr_summary,
            stdout_truncated: event.stdout_truncated,
            stderr_truncated: event.stderr_truncated,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub profile_id: String,
    pub running: bool,
    pub current_state: String,
    pub last_event: Option<ProbeEventDto>,
    pub last_success_at: Option<String>,
    pub consecutive_queue_miss: u64,
    pub next_probe_at: Option<String>,
    pub in_window: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityBucket {
    pub bucket_start: String,
    pub success_count: u64,
    pub queue_miss_count: u64,
    pub timeout_count: u64,
    pub config_error_count: u64,
    pub unknown_count: u64,
    pub last_status: Option<ProbeStatus>,
    pub last_latency_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeInstallation {
    pub checked_at: String,
    pub configured_path: String,
    pub detected_path: Option<String>,
    pub effective_path: Option<String>,
    pub version: Option<String>,
    pub source: String,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeRuntimeConfig {
    pub checked_at: String,
    pub default_model: Option<String>,
    pub model_source: String,
    pub default_effort: Option<String>,
    pub effort_source: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaudeDetectionLog {
    pub id: String,
    pub checked_at: String,
    pub configured_path: String,
    pub detected_path: Option<String>,
    pub effective_path: Option<String>,
    pub version: Option<String>,
    pub source: String,
    pub status: String,
    pub error: Option<String>,
}
