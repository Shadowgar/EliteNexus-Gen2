//! Bulk (historical) journal loading.
//!
//! Files are read and parsed in parallel with rayon, then handed to the caller strictly in
//! chronological order, a batch at a time, so memory stays bounded no matter how many years of
//! journals a commander has. Each [`FileEvents`] keeps the file text so callers can slice raw
//! lines and event names out of it without extra allocations.

use crate::events::Event;
use crate::paths::JournalFile;
use crate::raw::{parse_event_lenient, scan_line};
use crate::time::Timestamp;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// One complete journal line from a bulk load.
#[derive(Debug, Clone)]
pub struct LoadedEvent {
    pub ts: Timestamp,
    /// Byte offset of the line within the file.
    pub offset: u64,
    /// Byte length of the line (without the newline).
    pub len: usize,
    name_off: usize,
    name_len: usize,
    pub event: Event,
}

/// The parsed contents of one journal file.
#[derive(Debug)]
pub struct FileEvents {
    /// Index of the file in the list passed to [`load_history`].
    pub file: u32,
    bytes: Vec<u8>,
    pub checkpoint: Option<crate::watcher::Checkpoint>,
    pub events: Vec<LoadedEvent>,
    /// Bytes consumed: everything up to and including the last newline. A partially written
    /// final line (the game is still writing it) is left for the live tailer.
    pub consumed: u64,
    pub parse_errors: u32,
}

impl FileEvents {
    #[inline]
    pub fn line(&self, e: &LoadedEvent) -> &str {
        std::str::from_utf8(&self.bytes[e.offset as usize..e.offset as usize + e.len]).expect("validated journal line")
    }
    #[inline]
    pub fn name(&self, e: &LoadedEvent) -> &str {
        std::str::from_utf8(&self.bytes[e.name_off..e.name_off + e.name_len]).expect("validated journal event name")
    }
}

/// Parses the complete lines of one journal file.
pub fn read_journal_file(path: &Path, file: u32) -> std::io::Result<FileEvents> {
    read_snapshot(path, file, u64::MAX)
}

fn read_snapshot(path: &Path, file: u32, limit: u64) -> io::Result<FileEvents> {
    let handle = File::open(path)?;
    let identity = crate::watcher::file_identity(&handle)?;
    let mut bytes = Vec::new();
    handle.take(limit).read_to_end(&mut bytes)?;
    let mut events = parse_journal_bytes(bytes, file);
    events.checkpoint = Some(crate::watcher::Checkpoint::from_bytes(path.to_owned(), identity, &events.bytes, events.consumed));
    Ok(events)
}

/// Parses journal text (exposed for tests and replays).
pub fn parse_journal_text(text: String, file: u32) -> FileEvents {
    parse_journal_bytes(text.into_bytes(), file)
}

/// Parse complete UTF-8 records without changing the offsets of the surrounding bytes.
pub fn parse_journal_bytes(bytes: Vec<u8>, file: u32) -> FileEvents {
    let consumed = bytes.iter().rposition(|&b| b == b'\n').map_or(0, |i| i + 1);
    let mut events = Vec::with_capacity(consumed / 300 + 1);
    let mut parse_errors = 0u32;
    let mut pos = 0usize;
    let base = bytes.as_ptr() as usize;
    while pos < consumed {
        let end = bytes[pos..consumed].iter().position(|&b| b == b'\n').map_or(consumed, |i| pos + i);
        let Ok(line) = std::str::from_utf8(&bytes[pos..end]) else {
            parse_errors += 1;
            pos = end + 1;
            continue;
        };
        let line = line.trim_end_matches('\r');
        if !line.trim().is_empty() {
            if let Some((ts, name)) = scan_line(line) {
                match parse_event_lenient(name, line) {
                    Ok(event) => events.push(LoadedEvent {
                        ts,
                        offset: pos as u64,
                        len: line.len(),
                        name_off: name.as_ptr() as usize - base,
                        name_len: name.len(),
                        event,
                    }),
                    Err(err) => {
                        parse_errors += 1;
                        tracing::debug!(%err, event = name, "journal line failed typed parse");
                        events.push(LoadedEvent {
                            ts,
                            offset: pos as u64,
                            len: line.len(),
                            name_off: name.as_ptr() as usize - base,
                            name_len: name.len(),
                            event: Event::Other,
                        });
                    }
                }
            } else {
                parse_errors += 1;
            }
        }
        pos = end + 1;
    }
    FileEvents { file, bytes, checkpoint: None, events, consumed: consumed as u64, parse_errors }
}

/// Loads every file in `files` (oldest first), parsing `batch` files concurrently and calling
/// `sink` once per file in chronological order.
pub fn load_history(files: &[JournalFile], batch: usize, mut sink: impl FnMut(&JournalFile, FileEvents)) {
    let batch = batch.max(1);
    for (chunk_idx, chunk) in files.chunks(batch).enumerate() {
        let parsed: Vec<std::io::Result<FileEvents>> = chunk
            .par_iter()
            .enumerate()
            .map(|(i, f)| read_snapshot(&f.path, (chunk_idx * batch + i) as u32, f.size))
            .collect();
        for (f, res) in chunk.iter().zip(parsed) {
            match res {
                Ok(fe) => sink(f, fe),
                Err(err) => tracing::warn!(file = %f.path.display(), %err, "failed to read journal file"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_utf8_never_moves_a_following_event_offset() {
        let mut bytes = vec![0xff, b'\n'];
        let first = bytes.len();
        let line = "{\"timestamp\":\"2026-09-05T00:13:29Z\",\"event\":\"Commander\",\"FID\":\"F1\",\"Name\":\"Test-?\"}";
        bytes.extend(line.as_bytes());
        bytes.extend_from_slice(b"\r\npartial");
        let fe = parse_journal_bytes(bytes, 0);
        assert_eq!(fe.parse_errors, 1);
        assert_eq!(fe.events.len(), 1);
        assert_eq!(fe.events[0].offset, first as u64);
        assert_eq!(fe.line(&fe.events[0]), line);
        assert_eq!(fe.consumed, (first + line.len() + 2) as u64);
    }

    #[test]
    fn leaves_partial_last_line() {
        let text = concat!(
            r#"{ "timestamp":"2026-09-05T00:13:29Z", "event":"Commander", "FID":"F1", "Name":"Jameson" }"#,
            "\n",
            r#"{ "timestamp":"2026-09-05T00:13:30Z", "event":"Music", "MusicTrack":"NoTrack" }"#,
            "\r\n",
            r#"{ "timestamp":"2026-09-05T00:13:31Z", "event":"Fileh"#
        )
        .to_string();
        let fe = parse_journal_text(text, 0);
        assert_eq!(fe.events.len(), 2);
        assert_eq!(fe.name(&fe.events[0]), "Commander");
        assert_eq!(fe.name(&fe.events[1]), "Music");
        assert!(fe.line(&fe.events[1]).ends_with('}'));
        assert!(matches!(&fe.events[1].event, Event::Music(m) if m.music_track == "NoTrack"));
        let Event::Commander(c) = &fe.events[0].event else { panic!() };
        assert_eq!(c.name, "Jameson");
        assert!(fe.consumed < fe.bytes.len() as u64);
    }
}
