//! The core service: loads journal history, then tails the journal live, folding everything into
//! the shared [`Model`]. Runs on its own thread; the UI only ever takes short read locks.

use crossbeam_channel::{Receiver, Sender};
use nexus_journal::index::IndexedObservation;
use nexus_journal::watcher::CompanionUnavailable;
use nexus_journal::{JournalFile, JournalIndex, LineSource, Timestamp};
use nexus_journal::{
    list_journal_files, load_history, Companion, CompanionKind, EventCtx, LiveEvent, WatchMsg, Watcher, WatcherConfig,
};
use nexus_model::{Alert, Alerts, Model};
use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    Starting,
    Loading { done: usize, total: usize },
    Live,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotStatus {
    Missing,
    Incomplete,
    Unknown,
    Stale,
    Empty,
    Current,
}

impl SnapshotStatus {
    fn accepted(self) -> bool {
        matches!(self, Self::Empty | Self::Current)
    }
}

#[derive(Debug, Clone)]
pub struct CoreStatus {
    pub journal_dir: Option<PathBuf>,
    pub phase: Phase,
    pub files: usize,
    pub bytes: u64,
    pub load_ms: f64,
    pub current_file: Option<String>,
    pub live_events: u64,
    pub last_live: Option<Instant>,
    pub errors: Vec<String>,
    pub snapshots: HashMap<CompanionKind, SnapshotStatus>,
}

impl Default for CoreStatus {
    fn default() -> Self {
        CoreStatus {
            journal_dir: None,
            phase: Phase::Starting,
            files: 0,
            bytes: 0,
            load_ms: 0.0,
            current_file: None,
            live_events: 0,
            last_live: None,
            errors: Vec::new(),
            snapshots: HashMap::new(),
        }
    }
}

/// Commander/location context captured right after a live event was applied (under the model
/// write lock, so uploaders never see a newer state than the event they are sending).
#[derive(Debug, Clone, Default)]
pub struct EventContext {
    pub source_generation: u64,
    pub commander_generation: u64,
    pub cmdr: String,
    pub fid: String,
    pub game_version: String,
    pub game_build: String,
    pub horizons: Option<bool>,
    pub odyssey: Option<bool>,
    pub system_name: String,
    pub system_address: u64,
    pub star_pos: Option<[f64; 3]>,
    pub market_id: Option<u64>,
    pub station_name: Option<String>,
    pub ship_id: Option<u64>,
    pub observed_at: Option<Instant>,
}

impl EventContext {
    pub fn capture(m: &Model) -> EventContext {
        let Some(gs) = m.active() else { return EventContext::default() };
        let c = &gs.cmdr;
        let st = gs.location.station.as_ref().filter(|_| gs.location.docked);
        EventContext {
            source_generation: m.source_generation(),
            commander_generation: m.active_generation(),
            cmdr: c.name.clone(),
            fid: c.fid.clone(),
            game_version: c.game_version.clone(),
            game_build: c.game_build.clone(),
            // EDDN: these flags may only come from LoadGame (absent key = omitted).
            horizons: c.load_game_flags.0,
            odyssey: c.load_game_flags.1,
            system_name: gs.location.system.clone(),
            system_address: gs.location.system_address,
            star_pos: gs.location.star_pos,
            market_id: st.map(|s| s.market_id),
            // The journal's own spelling: uploads send what the game wrote.
            station_name: st.map(|s| s.journal_name.clone()),
            ship_id: gs.fleet.current_ship_id,
            observed_at: Some(Instant::now()),
        }
    }
}

/// Something that happened live, forwarded to other services (uploaders, market, …).
#[derive(Debug, Clone)]
pub enum LiveMsg {
    Event(LiveEvent, EventContext),
    Companion(Companion, EventContext),
}

/// Receives (event name, raw line) for every historical journal event during the initial load,
/// then ("Market", Market.json) for the current market file (seeds the galaxy database with the
/// commander's own stations, systems and hotspots).
pub type HistorySink = Box<dyn FnMut(&str, &str) + Send>;

/// Rhino launches with the mining location recorded for them, by the launch's journal time.
pub type Drops = std::collections::BTreeMap<nexus_journal::Timestamp, nexus_model::mining::SiteKey>;

/// How far each commander cleared their unsold exobiology list ([`nexus_model::Model::exobio_cleared`]).
pub type ExobioCleared = std::collections::BTreeMap<String, nexus_journal::Timestamp>;

type ScopeListener = RwLock<Option<Box<dyn Fn((u64, u64)) + Send + Sync>>>;

pub struct Core {
    pub model: Arc<RwLock<Model>>,
    /// All discovered journal events. Raw reads check the indexed file and line identity.
    pub index: Arc<RwLock<JournalIndex>>,
    pub status: Arc<RwLock<CoreStatus>>,
    pub alerts: Receiver<Alert>,
    subscribers: Arc<RwLock<Vec<Sender<LiveMsg>>>>,
    scope_listener: Arc<ScopeListener>,
    stop: Arc<AtomicBool>,
}

impl Core {
    /// `drops`: Rhino launches recorded on earlier runs ([`nexus_model::mining::Mining::drops`]), and `exobio_cleared`:
    /// how far each commander cleared their unsold exobiology list, both given to the model before it reads the
    /// journals.
    pub fn start(journal_dir: Option<PathBuf>, repaint: impl Fn() + Send + 'static, history_sink: Option<HistorySink>, drops: Drops, exobio_cleared: ExobioCleared) -> Core {
        let model = Arc::new(RwLock::new(Model::default()));
        let index = Arc::new(RwLock::new(JournalIndex::default()));
        let status = Arc::new(RwLock::new(CoreStatus::default()));
        let subscribers: Arc<RwLock<Vec<Sender<LiveMsg>>>> = Arc::new(RwLock::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let scope_listener = Arc::new(ScopeListener::default());
        let (alert_tx, alert_rx) = crossbeam_channel::unbounded();
        {
            let (model, index, status, subscribers, stop) = (model.clone(), index.clone(), status.clone(), subscribers.clone(), stop.clone());
            let scope_listener = scope_listener.clone();
            std::thread::Builder::new()
                .name("nexus-core".into())
                .spawn(move || run(journal_dir, model, index, status, subscribers, scope_listener, alert_tx, stop, repaint, history_sink, drops, exobio_cleared))
                .expect("spawn core thread");
        }
        Core { model, index, status, alerts: alert_rx, subscribers, scope_listener, stop }
    }

    /// Receive every live journal event / companion update (for uploaders etc.).
    pub fn subscribe(&self) -> Receiver<LiveMsg> {
        let (tx, rx) = crossbeam_channel::unbounded();
        self.subscribers.write().push(tx);
        rx
    }

    /// Runs before forwarding observations from a different verified source or commander scope.
    pub fn on_scope_changed(&self, listener: impl Fn((u64, u64)) + Send + Sync + 'static) {
        *self.scope_listener.write() = Some(Box::new(listener));
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Runs one event through the model. A panic there (a bug, or journal data the model doesn't expect) skips that
/// event instead of ending the thread that follows the journal; the panic hook has written it to crash.log.
fn guarded(apply: impl FnOnce()) -> bool {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(apply)).is_ok()
}

#[allow(clippy::too_many_arguments)]
fn run(
    journal_dir: Option<PathBuf>,
    model: Arc<RwLock<Model>>,
    index: Arc<RwLock<JournalIndex>>,
    status: Arc<RwLock<CoreStatus>>,
    subscribers: Arc<RwLock<Vec<Sender<LiveMsg>>>>,
    scope_listener: Arc<ScopeListener>,
    alert_tx: Sender<Alert>,
    stop: Arc<AtomicBool>,
    repaint: impl Fn(),
    mut history_sink: Option<HistorySink>,
    drops: Drops,
    exobio_cleared: ExobioCleared,
) {
    let local_profile = journal_dir.is_none();
    let Some(dir) = journal_dir.or_else(nexus_journal::default_journal_dir) else {
        tracing::warn!("no journal folder found");
        status.write().phase = Phase::Error("Praetorium couldn't find Elite Dangerous's journal folder on this computer.".into());
        repaint();
        return;
    };
    let dir = dir.canonicalize().unwrap_or(dir);
    tracing::info!(dir = %dir.display(), "journal folder");
    // Bindings and HUD colours read the game's Options folder, which on Linux and macOS sits in the same Wine prefix.
    #[cfg(not(test))]
    nexus_journal::set_journal_dir_in_use(Some(&dir));
    status.write().journal_dir = Some(dir.clone());
    while !stop.load(Ordering::Acquire) {
        let generation = next_source_generation();
        let live_from = Timestamp::now();
        let found = replay_and_follow(
            &dir, &model, &index, &status, &subscribers, &scope_listener, &alert_tx, &stop, &repaint,
            history_sink.take(), &drops, &exobio_cleared, generation, live_from, local_profile,
        );
        if !found {
            break;
        }
        {
            let mut status = status.write();
            status.phase = Phase::Starting;
            status.snapshots.clear();
        }
        repaint();
    }
}

fn next_source_generation() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

#[allow(clippy::too_many_arguments)]
fn replay_and_follow(
    dir: &std::path::Path,
    model: &RwLock<Model>,
    index: &RwLock<JournalIndex>,
    status: &RwLock<CoreStatus>,
    subscribers: &RwLock<Vec<Sender<LiveMsg>>>,
    scope_listener: &ScopeListener,
    alert_tx: &Sender<Alert>,
    stop: &AtomicBool,
    repaint: &impl Fn(),
    mut history_sink: Option<HistorySink>,
    drops: &Drops,
    exobio_cleared: &ExobioCleared,
    generation: u64,
    live_from: Timestamp,
    local_profile: bool,
) -> bool {
    let files = match list_journal_files(dir) {
        Ok(f) => f,
        Err(e) => {
            status.write().phase = Phase::Error(if e.kind() == std::io::ErrorKind::NotFound {
                format!("Praetorium looked for the journals in {}, but that folder doesn't exist on this computer.", dir.display())
            } else {
                format!("Praetorium can't read the journal folder {} ({e}).", dir.display())
            });
            repaint();
            return false;
        }
    };

    // ---- 1. history ---------------------------------------------------------------------
    let started = Instant::now();
    let total = files.len();
    {
        let mut st = status.write();
        st.files = total;
        st.bytes = files.iter().map(|f| f.size).sum();
        st.phase = Phase::Loading { done: 0, total };
    }
    repaint();
    let mut local = Model::with_mining_drops(drops.clone()).with_exobio_cleared(exobio_cleared.clone());
    local.set_source_generation(generation);
    let mut idx = JournalIndex::default();
    idx.source_generation = generation;
    let mut quiet = Alerts::disabled();
    let mut snapshots = SnapshotScope { local_profile, ..Default::default() };
    let mut current: Option<JournalFile> = None;
    let mut identities = HashSet::new();
    let mut resume = Vec::new();
    let mut done = 0usize;
    let mut skipped = 0usize;
    let mut last_progress = Instant::now();
    load_history(&files, 32, |f, fe| {
        if stop.load(Ordering::Acquire) {
            return;
        }
        let Some(checkpoint) = fe.checkpoint.as_ref() else { return };
        let file_id = idx.bind_file(f.path.clone(), checkpoint.identity);
        resume.push(checkpoint.clone());
        if !identities.insert(checkpoint.identity) {
            return;
        }
        begin_file(&mut local, &mut current, f);
        for e in &fe.events {
            let ctx = EventCtx { ts: e.ts, name: fe.name(e), line: fe.line(e), live: false };
            if !guarded(|| local.apply(&ctx, &e.event, &mut quiet)) {
                skipped += 1;
                continue;
            }
            snapshots.announce(&local, ctx.name, ctx.ts);
            index_observation(&mut idx, &local, file_id, e.offset, &ctx);
            if let Some(sink) = history_sink.as_mut() {
                sink(ctx.name, ctx.line);
            }
        }
        done += 1;
        if last_progress.elapsed() > Duration::from_millis(100) {
            status.write().phase = Phase::Loading { done, total };
            repaint();
            last_progress = Instant::now();
        }
    });
    if stop.load(Ordering::Acquire) {
        return false;
    }
    let mut live_file_id = current.as_ref().map(|f| idx.add_file(f.path.clone())).unwrap_or(0);
    // The watcher handles startup snapshots through the same freshness rules as subsequent changes.
    *index.write() = idx;
    let update_scope = |m: &Model| {
        if let Some(listener) = scope_listener.read().as_ref() {
            listener((m.source_generation(), m.active_generation()));
        }
    };
    {
        let mut current = model.write();
        *current = local;
        update_scope(&current);
    }
    {
        let mut st = status.write();
        st.load_ms = started.elapsed().as_secs_f64() * 1e3;
        st.phase = Phase::Live;
        if skipped > 0 {
            st.errors.push(format!("{skipped} journal entries couldn't be handled and were skipped (crash.log in the data folder has the details)."));
        }
        st.current_file = current.as_ref().map(|f| f.file_name.clone());
    }
    tracing::info!(files = total, ms = status.read().load_ms, "journal history loaded");
    repaint();

    // ---- 2. live ------------------------------------------------------------------------
    let (tx, rx) = crossbeam_channel::unbounded();
    let cfg = WatcherConfig { dir: dir.to_owned(), poll: Duration::from_millis(100), resume, initial: files, live_from };
    let _watcher = match Watcher::spawn(cfg, tx) {
        Ok(w) => w,
        Err(e) => {
            status.write().phase = Phase::Error(format!("Cannot start journal watcher: {e}"));
            repaint();
            return false;
        }
    };

    let mut duplicate_file = false;
    while !stop.load(Ordering::Acquire) {
        let first = match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(m) => m,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        // Batch everything that is already queued under one write lock.
        let mut batch = vec![first];
        while let Ok(m) = rx.try_recv() {
            batch.push(m);
            if batch.len() >= 512 {
                break;
            }
        }
        let mut alerts = Alerts::live();
        let mut forward = Vec::new();
        let mut failed: Vec<String> = Vec::new();
        {
            let mut m = model.write();
            for msg in batch {
                match msg {
                    WatchMsg::Event(le) => {
                        let live = le.live && !duplicate_file;
                        if duplicate_file {
                            continue;
                        }
                        let ctx = EventCtx { ts: le.raw.ts, name: &le.raw.name, line: &le.raw.line, live };
                        let commander_generation = m.active_generation();
                        let target = if live { &mut alerts } else { &mut quiet };
                        if !guarded(|| m.apply(&ctx, &le.event, target)) {
                            failed.push(format!("A {} event couldn't be handled and was skipped.", le.raw.name));
                            continue;
                        }
                        if m.active_generation() != commander_generation {
                            update_scope(&m);
                            status.write().snapshots.clear();
                        }
                        snapshots.announce(&m, ctx.name, ctx.ts);
                        index_observation(&mut index.write(), &m, live_file_id, le.raw.source.offset, &ctx);
                        if live && m.verified_fid().is_some() {
                            let ectx = EventContext::capture(&m);
                            forward.push(LiveMsg::Event(le, ectx));
                        }
                    }
                    WatchMsg::Companion(c, observed_live) => {
                        let decision = snapshots.admit(&m, &c);
                        status.write().snapshots.insert(c.kind(), decision);
                        if !decision.accepted() {
                            continue;
                        }
                        let live = observed_live && c.timestamp() >= live_from;
                        let target = if live { &mut alerts } else { &mut quiet };
                        if !guarded(|| m.apply_companion(&c, live, target)) {
                            failed.push(format!("{} couldn't be handled and was skipped.", c.kind().file_name()));
                            continue;
                        }
                        if let (Companion::Market(_, raw), Some(sink)) = (&c, history_sink.as_mut()) {
                            sink("Market", raw);
                        }
                        if live {
                            forward.push(LiveMsg::Companion(c, EventContext::capture(&m)));
                        }
                    }
                    WatchMsg::CompanionUnavailable(kind, reason) => {
                        let observation = match reason {
                            CompanionUnavailable::Missing => SnapshotStatus::Missing,
                            CompanionUnavailable::Incomplete => SnapshotStatus::Incomplete,
                        };
                        status.write().snapshots.insert(kind, observation);
                    }
                    WatchMsg::FileOpened(f, identity) => {
                        duplicate_file = !identities.insert(identity);
                        if !duplicate_file {
                            let previous = m.active_generation();
                            begin_file(&mut m, &mut current, &f);
                            if m.active_generation() != previous {
                                update_scope(&m);
                                status.write().snapshots.clear();
                            }
                            live_file_id = index.write().bind_file(f.path.clone(), identity);
                            status.write().current_file = Some(f.file_name);
                        }
                    }
                    WatchMsg::Rebuild => {
                        let mut retired = Model::default();
                        retired.set_source_generation(next_source_generation());
                        *m = retired;
                        update_scope(&m);
                        *index.write() = JournalIndex::default();
                        return true;
                    }

                    WatchMsg::Error(e) => {
                        tracing::warn!("{e}");
                        let mut st = status.write();
                        st.errors.push(e);
                        if st.errors.len() > 50 {
                            st.errors.remove(0);
                        }
                    }
                }
            }
        }
        {
            let mut st = status.write();
            st.live_events += forward.len() as u64;
            if !forward.is_empty() {
                st.last_live = Some(Instant::now());
            }
            st.errors.extend(failed);
            let excess = st.errors.len().saturating_sub(50);
            st.errors.drain(..excess);
        }
        if stop.load(Ordering::Acquire) {
            return false;
        }
        for a in alerts.items {
            let _ = alert_tx.send(a);
        }
        if !forward.is_empty() {
            let mut subs = subscribers.write();
            subs.retain(|s| forward.iter().all(|msg| s.send(msg.clone()).is_ok()));
        }
        repaint();
    }
    false
}

fn begin_file(model: &mut Model, current: &mut Option<JournalFile>, file: &JournalFile) {
    if current.as_ref().is_none_or(|previous| {
        previous.started != file.started || previous.part.checked_add(1) != Some(file.part)
    }) {
        model.begin_journal_session(file.started);
    }
    *current = Some(file.clone());
}

fn index_observation(index: &mut JournalIndex, model: &Model, file: u32, offset: u64, ctx: &EventCtx<'_>) {
    index.push_observation(IndexedObservation {
        timestamp: ctx.ts,
        source: LineSource { file, offset },
        scope: model.active_generation(),
        fid: model.verified_fid(),
        name: ctx.name,
        line: ctx.line,
    });
}

#[derive(Default)]
struct SnapshotScope {
    local_profile: bool,
    active: u64,
    announced: HashMap<CompanionKind, (Timestamp, bool)>,
    accepted: HashMap<CompanionKind, Timestamp>,
}

impl SnapshotScope {
    fn refresh(&mut self, model: &Model) {
        if self.active != model.active_generation() {
            self.active = model.active_generation();
            self.announced.clear();
            self.accepted.clear();
        }
    }

    fn announce(&mut self, model: &Model, name: &str, timestamp: Timestamp) {
        self.refresh(model);
        if model.verified_fid().is_some()
            && let Some(kind) = CompanionKind::from_event_name(name)
        {
            self.announced.insert(kind, (timestamp, name == "NavRouteClear"));
        }
    }

    /// An unowned or older snapshot may be retried after a qualifying journal line.
    fn admit(&mut self, model: &Model, companion: &Companion) -> SnapshotStatus {
        self.refresh(model);
        if model.verified_fid().is_none() { return SnapshotStatus::Unknown; }
        let ts = companion.timestamp();
        let kind = companion.kind();
        if ts.is_zero() || ts > Timestamp(Timestamp::now().0 + 5 * 60) {
            return SnapshotStatus::Unknown;
        }
        if ts < model.journal_session_start() || self.accepted.get(&kind).is_some_and(|previous| ts < *previous) {
            return SnapshotStatus::Stale;
        }
        if kind == CompanionKind::Status {
            // Status has no announcement or FID. Only use it with current, dated session evidence.
            if !self.local_profile || model.active().is_none_or(|gs| gs.game_stopped()) {
                return SnapshotStatus::Unknown;
            }
            if ts < model.last_event {
                return SnapshotStatus::Stale;
            }
        } else {
            let Some((announced, cleared)) = self.announced.get(&kind).copied() else { return SnapshotStatus::Unknown };
            if cleared || ts < announced {
                return SnapshotStatus::Stale;
            }
            if ts != announced {
                return SnapshotStatus::Unknown;
            }
            if let Companion::Market(market, _) = companion {
                let Some(gs) = model.active() else { return SnapshotStatus::Unknown };
                let loc = &gs.location;
                if !loc.docked || !market.star_system.eq_ignore_ascii_case(&loc.system)
                    || loc.station.as_ref().is_none_or(|station| station.market_id != market.market_id)
                {
                    return SnapshotStatus::Unknown;
                }
            }
            // A newer typed event may already have mutated the same projection.
            if matches!(kind, CompanionKind::Cargo | CompanionKind::ShipLocker | CompanionKind::Backpack | CompanionKind::NavRoute)
                && ts < model.last_event
            {
                return SnapshotStatus::Stale;
            }
        }
        self.accepted.insert(kind, ts);
        let empty = match companion {
            Companion::NavRoute(route, _) => route.route.is_empty(),
            Companion::Cargo(cargo, _) => cargo.cargo.inventory.as_ref().is_some_and(Vec::is_empty),
            Companion::Market(market, _) => market.items.is_empty(),
            _ => false,
        };
        if empty { SnapshotStatus::Empty } else { SnapshotStatus::Current }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(model: &mut Model, snapshots: &mut SnapshotScope, line: &str) {
        let (ts, name) = nexus_journal::raw::scan_line(line).unwrap();
        let event = nexus_journal::raw::parse_event_lenient(name, line).unwrap();
        model.apply(&EventCtx { ts, name, line, live: false }, &event, &mut Alerts::disabled());
        snapshots.announce(model, name, ts);
    }

    fn start() -> Timestamp { Timestamp::parse("2026-09-05T00:13:29Z").unwrap() }
    const OWNER_A: &str = r#"{"timestamp":"2026-09-05T00:13:29Z","event":"Commander","FID":"F1","Name":"One"}"#;
    const OWNER_B: &str = r#"{"timestamp":"2026-09-05T00:13:29Z","event":"Commander","FID":"F2","Name":"Two"}"#;

    #[test]
    fn replay_and_replacement_never_publish_historical_events() {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!("nexus-core-fixture-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Journal.2026-09-05T001329.01.log");
        std::fs::write(&path, format!("{OWNER_A}\n")).unwrap();
        let core = Core::start(Some(dir.clone()), || {}, None, Default::default(), Default::default());
        let events = core.subscribe();
        let wait = |fid: &str| {
            let deadline = Instant::now() + Duration::from_secs(6);
            loop {
                let model = core.model.read();
                if model.verified_fid() == Some(fid) {
                    return model.source_generation();
                }
                drop(model);
                assert!(Instant::now() < deadline, "journal projection did not recover");
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        let first = wait("F1");
        std::fs::rename(&path, dir.join("old.log")).unwrap();
        std::fs::write(&path, format!("{OWNER_B}\n")).unwrap();
        let second = wait("F2");
        assert_ne!(first, second);
        assert!(events.try_recv().is_err());
        assert!(core.alerts.try_recv().is_err());
        let index = core.index.read();
        assert_eq!(index.entries.len(), 1);
        assert_eq!(index.owner(&index.entries[0]), Some("F2"));
        drop(index);
        drop(core);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn multipart_scope_and_same_time_events_have_distinct_owners() {
        let mut model = Model::default();
        let mut snapshots = SnapshotScope::default();
        let mut current = None;
        let mut index = JournalIndex::default();
        let file = |part: u32, started| JournalFile { path: format!("fixture-{part}").into(), file_name: part.to_string(), started, part, size: 0 };
        begin_file(&mut model, &mut current, &file(1, start()));
        apply(&mut model, &mut snapshots, OWNER_A);
        begin_file(&mut model, &mut current, &file(2, start()));
        assert_eq!(model.verified_fid(), Some("F1"));
        let observe = |index: &mut JournalIndex, model: &Model| index.push_observation(IndexedObservation {
            timestamp: start(), source: LineSource::default(), scope: model.active_generation(),
            fid: model.verified_fid(), name: "Commander", line: "synthetic fixture",
        });
        let scope_a = model.active_generation();
        observe(&mut index, &model);
        apply(&mut model, &mut snapshots, OWNER_B);
        observe(&mut index, &model);
        assert_eq!(index.entries.len(), 2);
        assert!(index.belongs_to(&index.entries[0], "F1"));
        assert!(!index.belongs_to(&index.entries[0], "F2"));
        assert!(index.belongs_to(&index.entries[1], "F2"));
        assert!(index.in_scope(&index.entries[0], scope_a));
        assert!(!index.in_scope(&index.entries[1], scope_a));
        begin_file(&mut model, &mut current, &file(1, Timestamp(start().0 + 1)));
        assert!(model.active().is_none());
        assert_eq!(index.entries.len(), 2, "the aggregate index retains both owners");
    }

    #[test]
    fn same_session_name_can_gain_fid_without_rekeying_another_session() {
        let mut model = Model::default();
        let mut snapshots = SnapshotScope::default();
        model.begin_journal_session(start());
        apply(&mut model, &mut snapshots, r#"{"timestamp":"2026-09-05T00:13:29Z","event":"Commander","Name":"One"}"#);
        assert!(model.verified_fid().is_none());
        model.active_mut().unwrap().cmdr.credits = 123;
        apply(&mut model, &mut snapshots, OWNER_A);
        assert_eq!(model.verified_fid(), Some("F1"));
        assert_eq!(model.active().unwrap().cmdr.credits, 123);
        assert_eq!(model.commanders.len(), 1);
        let no_fid = r#"{"timestamp":"2026-09-05T00:13:29Z","event":"Commander","Name":"One"}"#;
        apply(&mut model, &mut snapshots, no_fid);
        assert!(model.verified_fid().is_none());
        assert_eq!(model.active().unwrap().cmdr.credits, 0);
        model.active_mut().unwrap().cmdr.credits = 456;
        model.begin_journal_session(start());
        apply(&mut model, &mut snapshots, no_fid);
        assert!(model.verified_fid().is_none());
        assert_eq!(model.active().unwrap().cmdr.credits, 0);
    }

    #[test]
    fn stale_or_previous_owner_snapshots_do_not_replace_current_state() {
        let mut model = Model::default();
        let mut snapshots = SnapshotScope::default();
        model.begin_journal_session(start());
        apply(&mut model, &mut snapshots, OWNER_A);
        apply(&mut model, &mut snapshots, r#"{"timestamp":"2026-09-05T00:13:31Z","event":"Cargo","Vessel":"Ship","Count":0}"#);
        let cargo = Companion::parse(CompanionKind::Cargo, r#"{"timestamp":"2026-09-05T00:13:31Z","event":"Cargo","Vessel":"Ship","Count":0,"Inventory":[]}"#).unwrap();
        assert_eq!(snapshots.admit(&model, &cargo), SnapshotStatus::Empty);
        let initial = Companion::parse(CompanionKind::Status, r#"{"timestamp":"2026-09-05T00:13:29Z","event":"Status","Flags":1}"#).unwrap();
        assert_eq!(snapshots.admit(&model, &initial), SnapshotStatus::Unknown);
        apply(&mut model, &mut snapshots, r#"{"timestamp":"2026-09-05T00:13:32Z","event":"NavRouteClear"}"#);
        assert!(!snapshots.admit(&model, &cargo).accepted());
        apply(&mut model, &mut snapshots, OWNER_B);
        assert!(!snapshots.admit(&model, &cargo).accepted());
    }
}
