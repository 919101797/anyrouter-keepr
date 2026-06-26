use crate::core::types::ProbeStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub status: ProbeStatus,
    pub error_kind: Option<String>,
}

pub fn classify(
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: &str,
    stderr: &str,
) -> Classification {
    if timed_out {
        return Classification {
            status: ProbeStatus::Timeout,
            error_kind: Some("timeout".to_string()),
        };
    }

    let combined = format!("{stdout}\n{stderr}").to_lowercase();

    if combined.contains("command not found")
        || combined.contains("no such file or directory")
        || combined.contains("could not find")
        || combined.contains("enoent")
    {
        return config("claude_not_found");
    }

    for code in ["401", "403"] {
        if contains_http_code(&combined, code) {
            return config(code);
        }
    }

    if combined.contains("invalid api key")
        || combined.contains("invalid token")
        || combined.contains("authentication")
        || combined.contains("unauthorized")
        || combined.contains("permission denied")
        || combined.contains("model not found")
        || combined.contains("settings")
    {
        return config("auth_or_config");
    }

    for code in ["429", "503", "524"] {
        if contains_http_code(&combined, code) {
            return queue(code);
        }
    }

    for marker in [
        "econnreset",
        "etimedout",
        "overloaded",
        "server busy",
        "network reset",
        "temporarily unavailable",
        "rate limit",
        "too many requests",
        "gateway timeout",
        "timed out",
    ] {
        if combined.contains(marker) {
            return queue(marker);
        }
    }

    if exit_code.unwrap_or(1) == 0 {
        return Classification {
            status: ProbeStatus::Success,
            error_kind: None,
        };
    }

    Classification {
        status: ProbeStatus::Unknown,
        error_kind: Some("unknown".to_string()),
    }
}

fn contains_http_code(text: &str, code: &str) -> bool {
    text.contains(&format!("http {code}"))
        || text.contains(&format!("status code {code}"))
        || text.contains(&format!("status: {code}"))
        || text.contains(&format!("\"status\":{code}"))
        || text.contains(&format!(" {code} "))
}

fn queue(kind: &str) -> Classification {
    Classification {
        status: ProbeStatus::QueueMiss,
        error_kind: Some(kind.to_string()),
    }
}

fn config(kind: &str) -> Classification {
    Classification {
        status: ProbeStatus::ConfigError,
        error_kind: Some(kind.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn treats_429_as_queue_miss() {
        let result = classify(Some(1), false, "", "API Error: 429 status code");
        assert_eq!(result.status, ProbeStatus::QueueMiss);
    }

    #[test]
    fn treats_524_as_queue_miss() {
        let result = classify(Some(1), false, "", "API Error: 524 status code");
        assert_eq!(result.status, ProbeStatus::QueueMiss);
    }

    #[test]
    fn treats_503_as_queue_miss() {
        let result = classify(Some(1), false, "", "HTTP 503 temporarily unavailable");
        assert_eq!(result.status, ProbeStatus::QueueMiss);
    }

    #[test]
    fn treats_network_reset_as_queue_miss() {
        let result = classify(Some(1), false, "", "request failed: ECONNRESET");
        assert_eq!(result.status, ProbeStatus::QueueMiss);
    }

    #[test]
    fn treats_overloaded_as_queue_miss() {
        let result = classify(Some(1), false, "", "model overloaded, server busy");
        assert_eq!(result.status, ProbeStatus::QueueMiss);
    }

    #[test]
    fn treats_timeout_as_timeout() {
        let result = classify(None, true, "", "");
        assert_eq!(result.status, ProbeStatus::Timeout);
    }

    #[test]
    fn treats_auth_as_config_error() {
        let result = classify(Some(1), false, "", "HTTP 401 unauthorized");
        assert_eq!(result.status, ProbeStatus::ConfigError);
    }

    #[test]
    fn success_when_exit_zero() {
        let result = classify(Some(0), false, "{\"result\":\"OK\"}", "");
        assert_eq!(result.status, ProbeStatus::Success);
    }
}
