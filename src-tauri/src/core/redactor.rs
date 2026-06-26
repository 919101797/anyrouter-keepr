use once_cell::sync::Lazy;
use regex::Regex;

static PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)(authorization\s*:\s*bearer\s+)[A-Za-z0-9._~+\-/=]+").unwrap(),
        Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+\-/=]+").unwrap(),
        Regex::new(r"(?i)((?:ANTHROPIC_AUTH_TOKEN|ANTHROPIC_API_KEY|CLAUDE_CODE_OAUTH_TOKEN)\s*[=:]\s*)[^\s,;]+").unwrap(),
        Regex::new(r"\bsk-[A-Za-z0-9._~+\-/=]{8,}\b").unwrap(),
        Regex::new(r"\bsk-ant-[A-Za-z0-9._~+\-/=]{8,}\b").unwrap(),
    ]
});

pub fn redact(input: &str) -> String {
    let mut output = input.to_string();
    for pattern in PATTERNS.iter() {
        output = pattern.replace_all(&output, "${1}<redacted>").to_string();
    }
    output
}

pub fn summarize_and_redact(input: &[u8], limit_bytes: usize) -> (Option<String>, bool) {
    if input.is_empty() {
        return (None, false);
    }

    let text = String::from_utf8_lossy(input);
    let redacted = redact(&text);
    let bytes = redacted.as_bytes();
    if bytes.len() <= limit_bytes {
        return (Some(redacted), false);
    }

    let mut end = limit_bytes.min(bytes.len());
    while end > 0 && !redacted.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = redacted[..end].to_string();
    truncated.push_str("\n[truncated]");
    (Some(truncated), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_tokens_and_headers() {
        let text = "Authorization: Bearer sk-ant-abc123456789 ANTHROPIC_AUTH_TOKEN=sk-secret999";
        let redacted = redact(text);
        assert!(!redacted.contains("abc123456789"));
        assert!(!redacted.contains("secret999"));
        assert!(redacted.contains("<redacted>"));
    }

    #[test]
    fn truncates_on_char_boundary() {
        let (summary, truncated) = summarize_and_redact("你好世界".as_bytes(), 5);
        assert!(truncated);
        assert!(summary.unwrap().contains("[truncated]"));
    }
}
