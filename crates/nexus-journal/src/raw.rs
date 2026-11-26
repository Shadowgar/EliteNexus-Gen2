//! Cheap pre-scan of journal lines.
//!
//! The game always writes `timestamp` then `event` as the first two keys, so both can be pulled out
//! with two substring searches. That lets us route every line (and skip the ones nobody cares
//! about) without running a JSON parser over it.

use crate::time::Timestamp;
use std::sync::Arc;

/// Returns the raw string value of `"key": "value"`, searching from the start of the line.
fn find_str_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let at = line.find(key)? + key.len();
    let rest = line[at..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Extracts `(timestamp, event name)` from a journal line without parsing the JSON.
pub fn scan_line(line: &str) -> Option<(Timestamp, &str)> {
    let ts = find_str_value(line, "\"timestamp\"")?;
    let ev = find_str_value(line, "\"event\"")?;
    Some((Timestamp::parse(ts)?, ev))
}

/// The game occasionally writes non-finite floats as bare tokens (`"Progress":-nan(ind)`,
/// `inf`, `1.#INF`, `-1.#IND`), which is invalid JSON. Returns a copy with every such token
/// outside of string literals replaced by `0`, or `None` if nothing needed fixing.
pub fn sanitize_non_finite(line: &str) -> Option<String> {
    const TOKENS: [&str; 10] =
        ["-nan(ind)", "nan(ind)", "-1.#QNAN", "1.#QNAN", "-1.#IND", "1.#IND", "-1.#INF", "1.#INF", "-nan", "-inf"];
    const SHORT: [&str; 3] = ["nan", "inf", "infinity"];
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let (mut start, mut quoted, mut escaped) = (0, false, false);
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if quoted {
            if escaped {
                escaped = false;
            } else if c == b'\\' {
                escaped = true;
            } else if c == b'"' {
                quoted = false;
            }
        } else if c == b'"' {
            quoted = true;
        } else if i == 0 || matches!(bytes[i - 1], b':' | b',' | b'[' | b' ' | b'\r' | b'\n' | b'\t') {
            let rest = &bytes[i..];
            let matched = TOKENS.iter().chain(SHORT.iter().rev()).find(|token| {
                let token = token.as_bytes();
                rest.get(..token.len()).is_some_and(|prefix| prefix.eq_ignore_ascii_case(token))
                    && rest.get(token.len()).is_none_or(|next| matches!(next, b',' | b'}' | b']' | b' ' | b'\r' | b'\n' | b'\t'))
            });
            if let Some(token) = matched {
                out.push_str(&line[start..i]);
                out.push('0');
                i += token.len();
                start = i;
                continue;
            }
        }
        i += 1;
    }
    if start == 0 {
        return None;
    }
    out.push_str(&line[start..]);
    Some(out)
}

/// Parses a typed event, retrying once with [`sanitize_non_finite`] if the game wrote NaN/Inf.
pub fn parse_event_lenient(name: &str, line: &str) -> serde_json::Result<crate::events::Event> {
    match crate::events::Event::parse(name, line) {
        Ok(e) => Ok(e),
        Err(err) => match sanitize_non_finite(line) {
            Some(fixed) => crate::events::Event::parse(name, &fixed),
            None => Err(err),
        },
    }
}

/// Where a journal line came from (for the logbook and for re-reading raw JSON on demand).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LineSource {
    /// Index into the journal file list held by the reader.
    pub file: u32,
    /// Byte offset of the line within the file.
    pub offset: u64,
}

/// An owned journal line with its pre-scanned header. Cheap to clone (the text is shared).
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub ts: Timestamp,
    pub name: Arc<str>,
    pub line: Arc<str>,
    pub source: LineSource,
}

impl RawEvent {
    pub fn new(line: &str, source: LineSource) -> Option<Self> {
        let (ts, name) = scan_line(line)?;
        Some(RawEvent { ts, name: Arc::from(name), line: Arc::from(line), source })
    }

    /// Parses the full JSON object (used by uploaders that forward the event with edits).
    pub fn json(&self) -> serde_json::Result<serde_json::Map<String, serde_json::Value>> {
        serde_json::from_str(&self.line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_header() {
        let l = r#"{ "timestamp":"2026-09-05T00:13:29Z", "event":"Commander", "FID":"F1", "Name":"X" }"#;
        let (t, e) = scan_line(l).unwrap();
        assert_eq!(e, "Commander");
        assert_eq!(t.to_iso(), "2026-09-05T00:13:29Z");
        assert!(scan_line("{}").is_none());
    }

    #[test]
    fn sanitizes_nan_outside_strings_only() {
        let l = r#"{ "timestamp":"2026-10-12T06:25:17Z", "event":"FSSDiscoveryScan", "Progress":-nan(ind), "BodyCount":0, "SystemName":"?lan ? inf", "X":inf, "Y":1.#IND }"#;
        let fixed = sanitize_non_finite(l).unwrap();
        assert!(fixed.contains(r#""Progress":0,"#));
        assert!(fixed.contains(r#""SystemName":"?lan ? inf""#));
        assert!(fixed.contains(r#""X":0,"#));
        assert!(fixed.contains(r#""Y":0 }"#));
        assert!(sanitize_non_finite(r#"{"a":"nan","b":1.5}"#).is_none());
        let ev = parse_event_lenient("FSSDiscoveryScan", l).unwrap();
        assert!(matches!(ev, crate::events::Event::FSSDiscoveryScan(_)));
    }
}
