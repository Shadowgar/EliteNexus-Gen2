//! Live tailing of the journal and companion files.
//!
//! Plain polling on a dedicated thread: the game keeps the journal open and appends to it, and
//! directory-change notifications on Windows are unreliable for that pattern. A stat call every
//! 100 ms on a dozen files is negligible CPU and gives ~100 ms worst-case latency, which beats
//! every tool that waits for change notifications or polls once a second.

use crate::companion::{Companion, CompanionKind};
use crate::events::Event;
use crate::paths::{list_journal_files, JournalFile};
use crate::raw::{LineSource, RawEvent};
use crossbeam_channel::Sender;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

/// A journal event observed live (written after the watcher started).
#[derive(Debug, Clone)]
pub struct LiveEvent {
    pub raw: RawEvent,
    pub event: Event,
    pub live: bool,
}

#[derive(Debug, Clone)]
pub enum WatchMsg {
    Event(LiveEvent),
    Companion(Companion, bool),
    CompanionUnavailable(CompanionKind, CompanionUnavailable),
    /// The watcher started tailing a (new) journal file.
    FileOpened(JournalFile, FileIdentity),
    /// A consumed file was replaced, shortened, removed or arrived out of order. Replay the source quietly.
    Rebuild,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct WatcherConfig {
    pub dir: PathBuf,
    pub poll: Duration,
    /// Complete-line cursors from the initial historical read, ordered oldest first.
    pub resume: Vec<Checkpoint>,
    /// Files present at the initial discovery, including any that could not be read.
    pub initial: Vec<JournalFile>,
    pub live_from: crate::Timestamp,
}

impl WatcherConfig {
    pub fn new(dir: PathBuf) -> Self {
        WatcherConfig { dir, poll: Duration::from_millis(100), resume: Vec::new(), initial: Vec::new(), live_from: crate::Timestamp::now() }
    }
}

pub struct Watcher {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Watcher {
    pub fn spawn(cfg: WatcherConfig, tx: Sender<WatchMsg>) -> std::io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let handle = std::thread::Builder::new()
            .name("journal-watcher".into())
            .spawn(move || run(cfg, tx, stop2))?;
        Ok(Watcher { stop, handle: Some(handle) })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Stable identity of an opened file, independent of its path and length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileIdentity(u64, u64);

#[cfg(unix)]
pub fn file_identity(file: &File) -> std::io::Result<FileIdentity> {
    use std::os::unix::fs::MetadataExt;
    let m = file.metadata()?;
    Ok(FileIdentity(m.dev(), m.ino()))
}

#[cfg(windows)]
pub fn file_identity(file: &File) -> std::io::Result<FileIdentity> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle};
    let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
    // SAFETY: the file handle stays open and the OS initializes the output on success.
    let info = unsafe {
        if GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        info.assume_init()
    };
    Ok(FileIdentity(u64::from(info.dwVolumeSerialNumber), (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow)))
}

#[derive(Debug, Clone)]
pub struct Checkpoint {
    pub path: PathBuf,
    pub identity: FileIdentity,
    pub consumed: u64,
    head: Vec<u8>,
    boundary: Vec<u8>,
}

const SAMPLE: usize = 256;

impl Checkpoint {
    pub(crate) fn from_bytes(path: PathBuf, identity: FileIdentity, bytes: &[u8], consumed: u64) -> Self {
        let n = consumed as usize;
        Self { path, identity, consumed, head: bytes[..n.min(SAMPLE)].to_vec(), boundary: bytes[n.saturating_sub(SAMPLE)..n].to_vec() }
    }

    fn check(&self, file: &mut File) -> std::io::Result<bool> {
        if file_identity(file)? != self.identity || file.metadata()?.len() < self.consumed {
            return Ok(false);
        }
        Ok(matches_at(file, 0, &self.head)? && matches_at(file, self.consumed - self.boundary.len() as u64, &self.boundary)?)
    }
}

fn matches_at(file: &mut File, offset: u64, sample: &[u8]) -> std::io::Result<bool> {
    let mut actual = [0; SAMPLE];
    file.seek(SeekFrom::Start(offset))?;
    match file.read_exact(&mut actual[..sample.len()]) {
        Ok(()) => Ok(actual[..sample.len()] == *sample),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e),
    }
}

struct Tail {
    file: File,
    checkpoint: Checkpoint,
    pending: Vec<u8>,
    offset: u64,
    historical_end: u64,
    live_from: crate::Timestamp,
}

impl Tail {
    fn read_lines(&mut self) -> std::io::Result<Vec<(u64, String)>> {
        self.file.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::new();
        (&mut self.file).take(1024 * 1024).read_to_end(&mut buf)?;
        let start_of_pending = self.offset - self.pending.len() as u64;
        self.offset += buf.len() as u64;
        self.pending.extend_from_slice(&buf);
        let mut out = Vec::new();
        let mut start = 0;
        while let Some(nl) = self.pending[start..].iter().position(|&b| b == b'\n') {
            let end = start + nl;
            if let Ok(line) = std::str::from_utf8(&self.pending[start..end]) {
                let line = line.trim_end_matches('\r');
                if !line.trim().is_empty() {
                    out.push((start_of_pending + start as u64, line.to_owned()));
                }
            }
            start = end + 1;
        }
        if start > 0 {
            let keep = start.min(SAMPLE);
            let mut boundary = std::mem::take(&mut self.checkpoint.boundary);
            let previous = boundary.len().min(SAMPLE - keep);
            boundary.drain(..boundary.len() - previous);
            boundary.extend_from_slice(&self.pending[start - keep..start]);
            self.checkpoint.boundary = boundary;
            self.checkpoint.consumed = start_of_pending + start as u64;
            let take = start.min(SAMPLE - self.checkpoint.head.len());
            self.checkpoint.head.extend_from_slice(&self.pending[..take]);
            self.pending.drain(..start);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionUnavailable {
    Missing,
    Incomplete,
}

#[derive(Default)]
struct CompanionTracker {
    seen: HashMap<CompanionKind, (SystemTime, u64, FileIdentity)>,
    unavailable: HashMap<CompanionKind, CompanionUnavailable>,
}

impl CompanionTracker {
    fn read(&mut self, dir: &Path, kind: CompanionKind, force: bool) -> std::io::Result<Option<Companion>> {
        let mut file = File::open(dir.join(kind.file_name()))?;
        let meta = file.metadata()?;
        let stamp = (meta.modified()?, meta.len(), file_identity(&file)?);
        if !force && self.seen.get(&kind) == Some(&stamp) {
            return Ok(None);
        }
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        let parsed = Companion::parse(kind, &text).ok_or(std::io::ErrorKind::InvalidData)?;
        self.seen.insert(kind, stamp);
        Ok(Some(parsed))
    }

    fn observe(&mut self, dir: &Path, kind: CompanionKind, force: bool, live: bool) -> Option<WatchMsg> {
        match self.read(dir, kind, force) {
            Ok(Some(value)) => {
                self.unavailable.remove(&kind);
                Some(WatchMsg::Companion(value, live))
            }
            Ok(None) => None,
            Err(e) => {
                let reason = if e.kind() == std::io::ErrorKind::NotFound {
                    CompanionUnavailable::Missing
                } else {
                    CompanionUnavailable::Incomplete
                };
                self.seen.remove(&kind);
                (self.unavailable.insert(kind, reason) != Some(reason)).then_some(WatchMsg::CompanionUnavailable(kind, reason))
            }
        }
    }
}

fn run(cfg: WatcherConfig, tx: Sender<WatchMsg>, stop: Arc<AtomicBool>) {
    let initial: HashMap<_, _> = cfg.initial.iter().map(|f| (f.path.clone(), f.size)).collect();
    let last_replayed = cfg.resume.last().map(|c| c.path.clone());
    let mut files_seen: HashMap<_, _> = cfg.resume.into_iter().map(|c| (c.path.clone(), c)).collect();
    let mut shapes = HashMap::new();
    let mut companions = CompanionTracker::default();
    let mut initial_pass = true;
    let mut newest: Option<(crate::Timestamp, u32)> = None;
    let mut announced = last_replayed.clone();
    let mut last_full_check = std::time::Instant::now() - Duration::from_secs(1);
    let mut files = Vec::new();
    while !stop.load(Ordering::Acquire) {
        let full_check = last_full_check.elapsed() >= Duration::from_secs(1);
        if full_check {
            last_full_check = std::time::Instant::now();
            match list_journal_files(&cfg.dir) {
                Ok(list) => files = list,
                Err(e) => {
                    let _ = tx.send(WatchMsg::Error(format!("cannot list journal folder: {e}")));
                    std::thread::sleep(cfg.poll);
                    continue;
                }
            }
            if files_seen.len() > files.iter().filter(|f| files_seen.contains_key(&f.path)).count() {
                let _ = tx.send(WatchMsg::Rebuild);
                return;
            }
        }
        let last = files.last().map(|f| &f.path);
        for info in &files {
            if !full_check && Some(&info.path) != last && files_seen.contains_key(&info.path) {
                continue;
            }
            let stamp = std::fs::metadata(&info.path).and_then(|m| Ok((m.len(), m.modified()?))).ok();
            let opened = File::open(&info.path).and_then(|file| file_identity(&file).map(|identity| (file, identity)));
            let (mut file, identity) = match opened {
                Ok(opened) => opened,
                Err(e) => {
                    if full_check {
                        let _ = tx.send(WatchMsg::Error(format!("journal open failed: {e}")));
                    }
                    // A file already skipped by history can be retried without blocking the active tail.
                    if initial.contains_key(&info.path) && !files_seen.contains_key(&info.path) && Some(&info.path) != last {
                        continue;
                    }
                    break;
                }
            };
            let order = (info.started, info.part);
            let old = files_seen.remove(&info.path);
            let edited_in_place = old.as_ref().is_some_and(|c| c.consumed > 0)
                && shapes.get(&info.path).is_some_and(|previous: &(u64, SystemTime)| stamp.is_some_and(|current| current.0 == previous.0 && current.1 != previous.1));
            if edited_in_place || old.as_ref().is_some_and(|c| !matches!(c.check(&mut file), Ok(true)))
                || old.is_none() && newest.is_some_and(|n| order < n)
                || old.as_ref().is_some_and(|c| newest.is_some_and(|n| order < n) && shapes.get(&info.path).is_some_and(|previous: &(u64, SystemTime)| stamp.is_some_and(|s| s.0 > previous.0)) && stamp.is_some_and(|s| s.0 > c.consumed))
            {
                let _ = tx.send(WatchMsg::Rebuild);
                return;
            }
            let new_file = old.is_none();
            let checkpoint = old.unwrap_or_else(|| Checkpoint::from_bytes(info.path.clone(), identity, &[], 0));
            let historical_end = initial.get(&info.path).copied().unwrap_or(0);
            let before_handoff = last_replayed.as_ref().is_some_and(|p| &info.path != p) && initial.contains_key(&info.path);
            let mut tail = Tail { offset: checkpoint.consumed, checkpoint, file, pending: Vec::new(), historical_end, live_from: cfg.live_from };
            let needs_tail = stamp.is_none_or(|s| s.0 > tail.offset);
            if before_handoff && (new_file || stamp.is_some_and(|s| s.0 > historical_end)) {
                let _ = tx.send(WatchMsg::Rebuild);
                return;
            }
            if !before_handoff && (needs_tail || new_file) {
                if announced.as_ref() != Some(&info.path) {
                    if tx.send(WatchMsg::FileOpened(info.clone(), identity)).is_err() {
                        return;
                    }
                    announced = Some(info.path.clone());
                }
                loop {
                    let offset = tail.offset;
                    if !emit_lines(&mut tail, &tx, &cfg.dir, &mut companions) {
                        return;
                    }
                    if tail.offset == offset || stop.load(Ordering::Acquire) {
                        break;
                    }
                }
            }
            files_seen.insert(info.path.clone(), tail.checkpoint);
            if let Some(stamp) = stamp {
                shapes.insert(info.path.clone(), stamp);
            }
            newest = Some(newest.map_or(order, |n| n.max(order)));
        }
        for kind in CompanionKind::ALL {
            if let Some(message) = companions.observe(&cfg.dir, kind, false, !initial_pass)
                && tx.send(message).is_err() {
                return;
            }
        }
        initial_pass = false;
        std::thread::sleep(cfg.poll);
    }
}

fn emit_lines(t: &mut Tail, tx: &Sender<WatchMsg>, dir: &Path, companions: &mut CompanionTracker) -> bool {
    let lines = match t.read_lines() {
        Ok(lines) => lines,
        Err(e) => {
            let _ = tx.send(WatchMsg::Error(format!("journal read failed: {e}")));
            return true;
        }
    };
    for (offset, line) in lines {
        let Some(raw) = RawEvent::new(&line, LineSource { file: 0, offset }) else { continue };
        let event = crate::raw::parse_event_lenient(&raw.name, &raw.line).unwrap_or(Event::Other);
        let kind = CompanionKind::from_event_name(&raw.name);
        let live = offset >= t.historical_end && raw.ts >= t.live_from && raw.ts <= crate::Timestamp(crate::Timestamp::now().0 + 300);
        if tx.send(WatchMsg::Event(LiveEvent { raw, event, live })).is_err() {
            return false;
        }
        if let Some(kind) = kind
            && let Some(message) = companions.observe(dir, kind, true, live)
            && tx.send(message).is_err()
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            static ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!("nexus-tail-{}-{}", std::process::id(), ID.fetch_add(1, Ordering::Relaxed)));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn file(&self, part: u32) -> PathBuf {
            self.0.join(format!("Journal.2026-09-05T001329.{part:02}.log"))
        }

        fn watcher(&self, initial: Vec<JournalFile>, resume: Vec<Checkpoint>) -> (Watcher, crossbeam_channel::Receiver<WatchMsg>) {
            let (tx, rx) = crossbeam_channel::unbounded();
            let cfg = WatcherConfig { dir: self.0.clone(), poll: Duration::from_millis(20), resume, initial, live_from: crate::Timestamp(0) };
            (Watcher::spawn(cfg, tx).unwrap(), rx)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const ONE: &str = "{\"timestamp\":\"2026-09-05T00:13:29Z\",\"event\":\"Commander\",\"FID\":\"F1\",\"Name\":\"One\"}";
    const TWO: &str = "{\"timestamp\":\"2026-09-05T00:13:29Z\",\"event\":\"Commander\",\"FID\":\"F2\",\"Name\":\"Two\"}";

    fn next_event(rx: &crossbeam_channel::Receiver<WatchMsg>) -> LiveEvent {
        loop {
            match rx.recv_timeout(Duration::from_secs(3)).unwrap() {
                WatchMsg::Event(event) => return event,
                WatchMsg::Rebuild => panic!("unexpected replay"),
                _ => {}
            }
        }
    }

    #[test]
    fn empty_start_then_multiple_parts_are_drained_in_order() {
        let dir = Fixture::new();
        let (_watcher, rx) = dir.watcher(Vec::new(), Vec::new());
        std::fs::write(dir.file(1), format!("{ONE}\n")).unwrap();
        std::fs::write(dir.file(2), format!("{TWO}\n")).unwrap();
        std::fs::write(dir.file(3), format!("{ONE}\n")).unwrap();
        for expected in [ONE, TWO, ONE] {
            let event = next_event(&rx);
            assert_eq!(&*event.raw.line, expected);
            assert_eq!(event.raw.source.offset, 0);
            assert!(event.live);
        }
    }

    #[test]
    fn append_during_bulk_initialization_is_consumed_once_after_the_fence() {
        let dir = Fixture::new();
        std::fs::write(dir.file(1), format!("{ONE}\n")).unwrap();
        std::fs::write(dir.file(2), format!("{TWO}\n")).unwrap();
        let initial = list_journal_files(&dir.0).unwrap();
        let mut resume = Vec::new();
        crate::reader::load_history(&initial, 1, |file, history| {
            if file.part == 1 {
                writeln!(std::fs::OpenOptions::new().append(true).open(dir.file(2)).unwrap(), "{ONE}").unwrap();
            }
            assert_eq!(history.events.len(), 1);
            resume.push(history.checkpoint.unwrap());
        });
        let (_watcher, rx) = dir.watcher(initial, resume);
        let event = next_event(&rx);
        assert_eq!(&*event.raw.line, ONE);
        assert_eq!(event.raw.source.offset, (TWO.len() + 1) as u64);
        assert!(event.live);
        while let Ok(message) = rx.recv_timeout(Duration::from_millis(100)) {
            assert!(!matches!(message, WatchMsg::Event(_)));
        }
    }

    #[test]
    fn companion_availability_reports_partial_then_safe_recovery() {
        let dir = Fixture::new();
        let mut tracker = CompanionTracker::default();
        let kind = CompanionKind::Cargo;
        assert!(matches!(tracker.observe(&dir.0, kind, false, false), Some(WatchMsg::CompanionUnavailable(_, CompanionUnavailable::Missing))));
        assert!(tracker.observe(&dir.0, kind, false, false).is_none());
        let file = dir.0.join(kind.file_name());
        std::fs::write(&file, r#"{"timestamp":"#).unwrap();
        assert!(matches!(tracker.observe(&dir.0, kind, false, false), Some(WatchMsg::CompanionUnavailable(_, CompanionUnavailable::Incomplete))));
        std::fs::write(&file, r#"{"timestamp":"2026-09-05T00:13:31Z","event":"Cargo","Vessel":"Ship","Count":0,"Inventory":[]}"#).unwrap();
        assert!(matches!(tracker.observe(&dir.0, kind, false, false), Some(WatchMsg::Companion(Companion::Cargo(..), false))));
        std::fs::remove_file(file).unwrap();
        assert!(matches!(tracker.observe(&dir.0, kind, false, false), Some(WatchMsg::CompanionUnavailable(_, CompanionUnavailable::Missing))));
    }

    #[test]
    fn historical_handoff_keeps_partial_bytes_and_one_cursor() {
        let dir = Fixture::new();
        let file = dir.file(1);
        let split = TWO.len() / 2;
        std::fs::write(&file, format!("{ONE}\n{}", &TWO[..split])).unwrap();
        let initial = list_journal_files(&dir.0).unwrap();
        let history = crate::reader::read_journal_file(&file, 0).unwrap();
        assert_eq!(history.events.len(), 1);
        writeln!(std::fs::OpenOptions::new().append(true).open(file).unwrap(), "{}", &TWO[split..]).unwrap();
        let (_watcher, rx) = dir.watcher(initial, vec![history.checkpoint.unwrap()]);
        let event = next_event(&rx);
        assert_eq!(&*event.raw.line, TWO);
        assert_eq!(event.raw.source.offset, (ONE.len() + 1) as u64);
        assert!(!event.live);
        while let Ok(message) = rx.recv_timeout(Duration::from_millis(100)) {
            assert!(!matches!(message, WatchMsg::Event(_)));
        }
    }

    #[test]
    fn restart_reconciles_replacement_and_truncation() {
        for replacement in [format!("{TWO}\n"), format!("{TWO}\n{TWO}\n"), String::new()] {
            let dir = Fixture::new();
            let file = dir.file(1);
            std::fs::write(&file, format!("{ONE}\n")).unwrap();
            let initial = list_journal_files(&dir.0).unwrap();
            let checkpoint = crate::reader::read_journal_file(&file, 0).unwrap().checkpoint.unwrap();
            if replacement.is_empty() {
                std::fs::write(&file, "").unwrap();
            } else {
                std::fs::rename(&file, dir.0.join("previous.log")).unwrap();
                std::fs::write(&file, replacement).unwrap();
            }
            let (_watcher, rx) = dir.watcher(initial, vec![checkpoint]);
            assert!(matches!(rx.recv_timeout(Duration::from_secs(3)).unwrap(), WatchMsg::Rebuild));
        }
    }
}
