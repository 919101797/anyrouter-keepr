use chrono::{DateTime, Duration, Local, NaiveTime};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeWindow {
    start: NaiveTime,
    end: NaiveTime,
    end_is_midnight: bool,
    all_day: bool,
}

impl TimeWindow {
    pub fn parse(start: &str, end: &str) -> Result<Self, String> {
        let start_value = start.trim();
        let end_value = end.trim();
        let start = parse_time(start_value)?;
        let end_is_midnight = end_value == "24:00";
        let end = if end_is_midnight {
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        } else {
            parse_time(end_value)?
        };
        let all_day = start == NaiveTime::from_hms_opt(0, 0, 0).unwrap() && end_is_midnight;

        Ok(Self {
            start,
            end,
            end_is_midnight,
            all_day,
        })
    }

    pub fn contains(&self, now: DateTime<Local>) -> bool {
        if self.all_day {
            return true;
        }

        let current = now.time();
        if self.end_is_midnight {
            return current >= self.start;
        }
        if self.start <= self.end {
            current >= self.start && current < self.end
        } else {
            current >= self.start || current < self.end
        }
    }

    pub fn next_start_after(&self, now: DateTime<Local>) -> DateTime<Local> {
        if self.contains(now) {
            return now;
        }

        let today_start = now
            .date_naive()
            .and_time(self.start)
            .and_local_timezone(Local)
            .single()
            .unwrap_or(now);
        if today_start > now {
            today_start
        } else {
            today_start + Duration::days(1)
        }
    }
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    let value = value.trim();
    let parts: Vec<_> = value.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid time: {value}"));
    }
    let hour: u32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid hour: {value}"))?;
    let minute: u32 = parts[1]
        .parse()
        .map_err(|_| format!("invalid minute: {value}"))?;
    if hour > 23 || minute > 59 {
        return Err(format!("invalid time: {value}"));
    }
    NaiveTime::from_hms_opt(hour, minute, 0).ok_or_else(|| format!("invalid time: {value}"))
}

pub fn seconds_until(target: DateTime<Local>) -> u64 {
    let now = Local::now();
    if target <= now {
        0
    } else {
        (target - now).num_seconds().max(0) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    #[test]
    fn handles_all_day_window() {
        let window = TimeWindow::parse("00:00", "24:00").unwrap();
        assert!(window.contains(Local.with_ymd_and_hms(2026, 6, 26, 0, 0, 0).unwrap()));
        assert!(window.contains(Local.with_ymd_and_hms(2026, 6, 26, 12, 0, 0).unwrap()));
        assert!(window.contains(Local.with_ymd_and_hms(2026, 6, 26, 23, 59, 0).unwrap()));
    }

    #[test]
    fn handles_5_to_midnight() {
        let window = TimeWindow::parse("05:00", "24:00").unwrap();
        let inside = Local.with_ymd_and_hms(2026, 6, 26, 23, 59, 0).unwrap();
        let outside = Local.with_ymd_and_hms(2026, 6, 26, 2, 0, 0).unwrap();
        assert!(window.contains(inside));
        assert!(!window.contains(outside));
    }

    #[test]
    fn handles_overnight_window() {
        let window = TimeWindow::parse("22:00", "06:00").unwrap();
        assert!(window.contains(Local.with_ymd_and_hms(2026, 6, 26, 23, 0, 0).unwrap()));
        assert!(window.contains(Local.with_ymd_and_hms(2026, 6, 26, 3, 0, 0).unwrap()));
        assert!(!window.contains(Local.with_ymd_and_hms(2026, 6, 26, 12, 0, 0).unwrap()));
    }

    #[test]
    fn trims_time_values() {
        let window = TimeWindow::parse(" 00:00 ", " 24:00 ").unwrap();
        assert!(window.contains(Local.with_ymd_and_hms(2026, 6, 26, 8, 0, 0).unwrap()));
    }
}
