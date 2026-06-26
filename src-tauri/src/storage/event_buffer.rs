use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::core::types::ProbeEvent;

#[derive(Debug)]
pub struct EventBuffer {
    pending: Vec<ProbeEvent>,
    repeated_errors: HashMap<String, u64>,
    last_flush: Instant,
    flush_count: usize,
    flush_interval: Duration,
}

impl EventBuffer {
    pub fn new(flush_count: usize, flush_interval_seconds: u64) -> Self {
        Self {
            pending: Vec::new(),
            repeated_errors: HashMap::new(),
            last_flush: Instant::now(),
            flush_count: flush_count.max(1),
            flush_interval: Duration::from_secs(flush_interval_seconds.max(1)),
        }
    }

    pub fn push(&mut self, mut event: ProbeEvent) -> bool {
        self.compact_repeated_error_text(&mut event);
        self.pending.push(event);
        self.should_flush()
    }

    pub fn drain(&mut self) -> Vec<ProbeEvent> {
        self.last_flush = Instant::now();
        self.pending.drain(..).collect()
    }

    pub fn should_flush(&self) -> bool {
        self.pending.len() >= self.flush_count || self.last_flush.elapsed() >= self.flush_interval
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn compact_repeated_error_text(&mut self, event: &mut ProbeEvent) {
        let Some(kind) = event.error_kind.clone() else {
            return;
        };
        let key = format!("{}:{kind}", event.status.as_str());
        let count = self.repeated_errors.entry(key).or_insert(0);
        *count += 1;
        if *count > 1 {
            if event
                .stderr_summary
                .as_ref()
                .is_some_and(|value| value.len() > 256)
            {
                event.stderr_summary = Some(format!("same error repeated {} times: {kind}", count));
                event.stderr_truncated = true;
            }
            if event
                .stdout_summary
                .as_ref()
                .is_some_and(|value| value.len() > 256)
            {
                event.stdout_summary = Some(format!("same error repeated {} times: {kind}", count));
                event.stdout_truncated = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Local;
    use uuid::Uuid;

    use super::*;
    use crate::core::types::ProbeStatus;

    fn event(stderr: &str) -> ProbeEvent {
        let now = Local::now();
        ProbeEvent {
            id: Uuid::new_v4().to_string(),
            profile_id: "default".to_string(),
            started_at: now,
            ended_at: now,
            duration_ms: 1,
            status: ProbeStatus::QueueMiss,
            error_kind: Some("429".to_string()),
            exit_code: Some(1),
            base_url: "https://anyrouter.top".to_string(),
            model: "sonnet".to_string(),
            prompt_summary: Some("用一句话讲个笑话".to_string()),
            prompt_truncated: false,
            stdout_summary: None,
            stderr_summary: Some(stderr.to_string()),
            stdout_truncated: false,
            stderr_truncated: false,
        }
    }

    #[test]
    fn flushes_after_count() {
        let mut buffer = EventBuffer::new(2, 300);
        assert!(!buffer.push(event("short")));
        assert!(buffer.push(event("short")));
    }

    #[test]
    fn compacts_repeated_long_error() {
        let long = "x".repeat(1024);
        let mut buffer = EventBuffer::new(10, 300);
        buffer.push(event(&long));
        buffer.push(event(&long));
        let items = buffer.drain();
        assert!(items[1]
            .stderr_summary
            .as_ref()
            .unwrap()
            .contains("same error repeated"));
    }
}
