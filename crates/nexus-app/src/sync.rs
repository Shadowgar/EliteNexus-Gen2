//! Uploads to EDDN / EDSM / Inara / EDAstro: forwards the core's live events to `nexus-uplink`.
//!
//! Only live events ever reach this (the history load never does), and every service stays off
//! until the user enables it in Accounts & sync.

use crate::core::{Core, EventContext, LiveMsg, Phase};
use crate::secrets;
use crate::settings::Accounts;
use crossbeam_channel::Receiver;
use nexus_uplink::{EdsmCredentials, InaraCredentials, Uplink, UplinkConfig, UplinkStatus, UploadContext};
use nexus_journal::{LineSource, RawEvent};
use std::path::Path;
use std::sync::Arc;

/// Gen2 needs separate application authorization before a real outbound transport starts.
pub const OUTBOUND_AUTHORIZED: bool = false;

pub struct SyncService {
    /// None for test runs: the uplink is never started, so its persisted queues are not touched.
    uplink: Option<Arc<Uplink>>,
    applied: UplinkConfig,
    /// Send the current session's profile to Inara once the journal history is loaded.
    inara_due: Option<u64>,
    attachment: Arc<parking_lot::Mutex<Attachment>>,
}

#[derive(Default)]
struct Attachment {
    id: u64,
    scope: u64,
    observed: Option<(u64, u64)>,
}

impl SyncService {
    pub fn start(core: &Core, acc: &Accounts, data_dir: &Path, rt: &tokio::runtime::Handle) -> SyncService {
        if !OUTBOUND_AUTHORIZED || crate::launch::isolated() { return Self::disabled(); }
        let cfg = config(acc);
        let uplink = Arc::new(Uplink::new(cfg.clone(), data_dir, rt));
        let inara_due = inara_snapshot_due(&UplinkConfig::default(), &cfg);
        let mut sync = SyncService { uplink: Some(uplink), applied: cfg, inara_due: None, attachment: Default::default() };
        sync.attach(core);
        sync.inara_due = inara_due.then(|| sync.attachment.lock().scope);
        sync
    }

    /// No uploads at all (screenshot / replay runs).
    pub fn disabled() -> SyncService {
        SyncService { uplink: None, applied: UplinkConfig::default(), inara_due: None, attachment: Default::default() }
    }

    /// Call every frame: brings the Inara profile up to date when it is due (at start-up and
    /// when sync is switched on or the key changes) and the journal history has been read.
    pub fn tick(&mut self, core: &Core) {
        if self.inara_due.is_none() || !matches!(core.status.read().phase, Phase::Live) {
            return;
        }
        let due = self.inara_due.take();
        let Some(u) = &self.uplink else { return };
        let model = core.model.read();
        let attachment = self.attachment.lock();
        if due != Some(attachment.scope) || attachment.observed != Some((model.source_generation(), model.active_generation())) { return; }
        let events = current_session(core, &model);
        if !events.is_empty() {
            let ctx = upload_context(&EventContext::capture(&model));
            u.sync_inara_profile(events, &ctx);
        }
    }

    /// Asks Inara again who the key belongs to.
    pub fn check_inara(&self) {
        if let Some(u) = &self.uplink {
            u.check_inara();
        }
    }

    /// Follows a (re)started core. The previous forwarder ends when its core stops.
    pub fn attach(&self, core: &Core) {
        if let Some(u) = &self.uplink {
            let model = core.model.read();
            let owner = {
                let mut active = self.attachment.lock();
                active.id = active.id.wrapping_add(1);
                active.scope = active.scope.wrapping_add(1);
                active.observed = (model.source_generation() != 0).then(|| (model.source_generation(), model.active_generation()));
                u.source_changed();
                active.id
            };
            let active = self.attachment.clone();
            let uplink = u.clone();
            core.on_scope_changed(move |current| {
                let mut a = active.lock();
                if a.id == owner && a.observed != Some(current) && a.observed.replace(current).is_some() {
                    a.scope = a.scope.wrapping_add(1);
                    uplink.source_changed();
                }
            });
            forward(core.subscribe(), u.clone(), self.attachment.clone(), owner);
        }
    }

    /// Call after the account settings changed (decrypts the keys, so not every frame).
    pub fn apply(&mut self, acc: &Accounts) {
        let Some(u) = &self.uplink else { return };
        let cfg = config(acc);
        if cfg != self.applied {
            let active = self.attachment.lock();
            let previous_key = snapshot_key(&self.applied);
            let retained = self.inara_due == Some(active.scope) && previous_key.is_some() && previous_key == snapshot_key(&cfg);
            self.inara_due = (retained || inara_snapshot_due(&self.applied, &cfg)).then_some(active.scope);
            u.set_config(cfg.clone());
            self.applied = cfg;
        }
    }

    pub fn status(&self) -> UplinkStatus {
        self.uplink.as_ref().map(|u| u.status()).unwrap_or_default()
    }

    pub fn any_enabled(&self) -> bool {
        self.applied.any_enabled()
    }

    /// Sends what is batched now (rate limits still apply; the rest stays queued on disk).
    pub fn flush(&self) {
        if let Some(u) = &self.uplink {
            u.flush();
        }
    }
}

/// Read the current commander's session from verified, generation-bound raw observations.
fn current_session(core: &Core, model: &nexus_model::Model) -> Vec<RawEvent> {
    let Some(fid) = model.verified_fid() else { return Vec::new() };
    let scope = model.active_generation();
    let idx = core.index.read();
    if idx.source_generation != model.source_generation() {
        return Vec::new();
    }
    let mut events: Vec<_> = idx.entries.iter().rev().take_while(|e| idx.in_scope(e, scope))
        .filter(|e| idx.belongs_to(e, fid))
        .filter_map(|e| {
            let line = idx.read_line(e)?;
            RawEvent::new(&line, LineSource::default())
        })
        .collect();
    events.reverse();
    events
}

fn forward(rx: Receiver<LiveMsg>, uplink: Arc<Uplink>, attachment: Arc<parking_lot::Mutex<Attachment>>, owner: u64) {
    let spawned = std::thread::Builder::new().name("nexus-sync".into()).spawn(move || {
        // Watcher order is preserved (EDSM swaps Cargo/ShipLocker events for their companion files).
        for msg in rx {
            let active = attachment.lock();
            if active.id != owner { break; }
            let context = match &msg {
                LiveMsg::Event(_, c) | LiveMsg::Companion(_, c) => c,
            };
            if active.observed != Some((context.source_generation, context.commander_generation)) { continue; }
            match msg {
                LiveMsg::Event(e, c) => {
                    if let Some(at) = c.observed_at { uplink.on_event_at(&e, &upload_context(&c), at); }
                }
                LiveMsg::Companion(cmp, c) => {
                    if let Some(at) = c.observed_at { uplink.on_companion_at(&cmp, &upload_context(&c), at); }
                }
            }
        }
    });
    if let Err(e) = spawned {
        tracing::error!("sync: cannot start the forwarding thread: {e}");
    }
}

fn upload_context(c: &EventContext) -> UploadContext {
    UploadContext {
        cmdr: c.cmdr.clone(),
        fid: c.fid.clone(),
        game_version: c.game_version.clone(),
        game_build: c.game_build.clone(),
        horizons: c.horizons,
        odyssey: c.odyssey,
        system_name: c.system_name.clone(),
        system_address: c.system_address,
        star_pos: c.star_pos,
        market_id: c.market_id,
        station_name: c.station_name.clone(),
        ship_id: c.ship_id,
        is_beta: nexus_uplink::json::is_beta_version(&c.game_version),
    }
}

fn snapshot_key(cfg: &UplinkConfig) -> Option<&str> {
    cfg.inara_sending().filter(|_| cfg.inara_profile_consent).map(|c| c.api_key.as_str())
}

fn inara_snapshot_due(previous: &UplinkConfig, next: &UplinkConfig) -> bool {
    !previous.inara_profile_consent && snapshot_key(next).is_some()
}

/// The uplink configuration for the user's choices (keys decrypted here, never logged).
pub fn config(acc: &Accounts) -> UplinkConfig {
    UplinkConfig {
        software_name: crate::identity::NAME.to_owned(),
        eddn: acc.eddn_upload,
        eddn_test_mode: acc.eddn_test_mode,
        eddn_paused: acc.eddn_paused,
        edsm: acc.edsm_enabled.then(|| EdsmCredentials {
            commander_name: acc.edsm_commander.trim().to_string(),
            api_key: secrets::open(&acc.edsm_api_key),
        }),
        edsm_paused: acc.edsm_paused,
        inara: acc.inara_enabled.then(|| InaraCredentials { api_key: secrets::open(&acc.inara_api_key) }),
        inara_paused: acc.inara_paused,
        inara_profile_consent: acc.inara_profile_consent,
        edastro: acc.edastro_enabled,
        edastro_paused: acc.edastro_paused,
        ..UplinkConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_upload_nothing() {
        let s = crate::settings::Settings::default();
        let cfg = config(&s.accounts);
        assert!(!cfg.any_enabled());
        assert_eq!(cfg.software(), "EliteNexus-Gen2");
        assert!(cfg.user_agent().starts_with("EliteNexus-Gen2/"));
    }

    #[test]
    fn production_gate_never_opens_upload_queues() {
        let fixture = std::env::temp_dir().join(format!("gen2-gated-uplink-{}", std::process::id()));
        let core = Core::start(Some(fixture.join("journals")), || {}, None, Default::default(), Default::default());
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let acc = Accounts { eddn_upload: true, edastro_enabled: true, ..Default::default() };
        let sync = SyncService::start(&core, &acc, &fixture, rt.handle());
        assert!(sync.uplink.is_none() && !sync.any_enabled());
        assert!(!fixture.join("uplink").exists());
    }

    #[test]
    fn inara_snapshot_is_not_reissued_on_resume_or_key_replacement() {
        let consent = UplinkConfig {
            inara: Some(InaraCredentials { api_key: "fixture-old".into() }),
            inara_profile_consent: true,
            ..UplinkConfig::default()
        };
        let paused = UplinkConfig { inara_paused: true, ..consent.clone() };
        let rekeyed = UplinkConfig { inara: Some(InaraCredentials { api_key: "fixture-new".into() }), ..consent.clone() };
        let revoked = UplinkConfig { inara_profile_consent: false, ..rekeyed.clone() };
        assert!(inara_snapshot_due(&UplinkConfig::default(), &consent));
        assert!(!inara_snapshot_due(&consent, &paused));
        assert!(!inara_snapshot_due(&paused, &consent));
        assert!(!inara_snapshot_due(&consent, &rekeyed));
        assert!(inara_snapshot_due(&revoked, &rekeyed));
    }

    #[test]
    fn edsm_needs_name_and_key() {
        let acc = Accounts { edsm_enabled: true, edsm_commander: "Jameson".into(), ..Default::default() };
        assert!(!config(&acc).edsm_enabled(), "no key yet");
        let acc = Accounts { edsm_api_key: secrets::seal("k").expect("synthetic key"), ..acc };
        assert!(config(&acc).edsm_enabled());
    }

    /// The header's button pauses every service that's on, and resumes them; one that's off isn't touched.
    #[test]
    fn pause_every_upload_that_is_on() {
        let mut acc = Accounts { eddn_upload: true, edsm_enabled: true, edsm_commander: "Jameson".into(), edsm_api_key: secrets::seal("k").expect("synthetic key"), edastro_enabled: true, ..Default::default() };
        assert_eq!(acc.uploads(), [("EDDN", false), ("EDSM", false), ("EDAstro", false)], "Inara is off");
        acc.pause_uploads(true);
        assert_eq!(acc.uploads(), [("EDDN", true), ("EDSM", true), ("EDAstro", true)]);
        assert!(!acc.inara_paused);
        assert!(!config(&acc).any_enabled(), "nothing is sent");
        acc.pause_uploads(false);
        assert!(config(&acc).edsm_enabled() && config(&acc).eddn_enabled() && config(&acc).edastro_enabled());
    }
}
