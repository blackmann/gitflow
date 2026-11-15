//! Small formatting helpers shared by the views.

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

/// RFC 3339 → Unix seconds; unparsable input reads as "unknown" (0).
pub fn parse_time(s: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

/// "just now", "5m ago", "3h ago", "4d ago", then a calendar date.
pub fn relative(ts: i64, now: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let d = (now - ts).max(0);
    match d {
        0..60 => "just now".into(),
        60..3600 => format!("{}m ago", d / 60),
        3600..86_400 => format!("{}h ago", d / 3600),
        86_400..2_592_000 => format!("{}d ago", d / 86_400),
        _ => date(ts, now),
    }
}

/// "Mar 4" this year, "Mar 4, 2024" otherwise, in local time.
pub fn date(ts: i64, now: i64) -> String {
    use chrono::{Datelike, Local, TimeZone};
    let (Some(t), Some(n)) = (
        Local.timestamp_opt(ts, 0).single(),
        Local.timestamp_opt(now, 0).single(),
    ) else {
        return String::new();
    };
    if t.year() == n.year() {
        t.format("%b %-d").to_string()
    } else {
        t.format("%b %-d, %Y").to_string()
    }
}

pub fn when(ts: i64, relative_dates: bool) -> String {
    if relative_dates {
        relative(ts, now())
    } else {
        date(ts, now())
    }
}

/// Elapsed time as "45s", "3m 12s", "1h 4m".
pub fn duration(secs: i64) -> String {
    let s = secs.max(0);
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m {}s", s / 60, s % 60),
        _ => format!("{}h {}m", s / 3600, (s % 3600) / 60),
    }
}

/// Duration between two optional RFC 3339 times; an open end counts up to now.
pub fn span(start: Option<&str>, end: Option<&str>) -> Option<String> {
    let start = parse_time(start?);
    if start <= 0 {
        return None;
    }
    let end = end.map(parse_time).filter(|t| *t > 0).unwrap_or_else(now);
    Some(duration(end - start))
}

pub fn thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// 47024 → "47k", 1530 → "1.5k".
pub fn compact(n: u64) -> String {
    let v = n as f64;
    if v >= 1_000_000. {
        format!("{:.1}M", v / 1_000_000.).replace(".0M", "M")
    } else if v >= 10_000. {
        format!("{}k", (v / 1_000.).round())
    } else if v >= 1_000. {
        format!("{:.1}k", v / 1_000.).replace(".0k", "k")
    } else {
        n.to_string()
    }
}

pub fn bytes(n: u64) -> String {
    let v = n as f64;
    if v >= 1_048_576. {
        format!("{:.1} MB", v / 1_048_576.)
    } else if v >= 1024. {
        format!("{:.1} KB", v / 1024.)
    } else {
        format!("{n} B")
    }
}

/// Up to two letters for an avatar placeholder: "Ada Lovelace" → "AL".
pub fn initials(name: &str) -> String {
    let words = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_' || c == '.')
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    let mut out: String = match words.as_slice() {
        [] => "?".into(),
        [one] => one.chars().take(2).collect(),
        [first, .., last] => first.chars().take(1).chain(last.chars().take(1)).collect(),
    };
    out = out.to_uppercase();
    out
}

/// A stable small number for a name, used to pick a placeholder color.
pub fn hash(name: &str) -> usize {
    name.bytes()
        .fold(5381usize, |h, b| h.wrapping_mul(33) ^ b as usize)
}

/// Remove terminal escape sequences from log output.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

pub fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(7)]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats_relative_times() {
        let now = 1_700_000_000;
        assert_eq!(relative(now - 5, now), "just now");
        assert_eq!(relative(now - 300, now), "5m ago");
        assert_eq!(relative(now - 7200, now), "2h ago");
        assert_eq!(relative(now - 86_400 * 3, now), "3d ago");
        assert_eq!(relative(0, now), "");
    }
    #[test]
    fn formats_numbers() {
        assert_eq!(thousands(9_812_004), "9,812,004");
        assert_eq!(compact(47_024), "47k");
        assert_eq!(compact(1_530), "1.5k");
        assert_eq!(compact(999), "999");
        assert_eq!(duration(192), "3m 12s");
        assert_eq!(bytes(2048), "2.0 KB");
    }
    #[test]
    fn makes_initials_and_strips_ansi() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("octocat"), "OC");
        assert_eq!(initials(""), "?");
        assert_eq!(strip_ansi("\u{1b}[31mred\u{1b}[0m text"), "red text");
    }
}
