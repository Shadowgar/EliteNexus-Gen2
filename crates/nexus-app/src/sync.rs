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

pub struct SyncService {
    /// None for test runs: the uplink is never started, so its persisted queues are not touched.
    uplink: Option<Arc<Uplink>>,
    applied: UplinkConfig,
    /// Send the current session's profile to Inara once the journal history is loaded.
    inara_due: bool,
    attachment: Arc<parking_lot::Mutex<u64>>,
}

impl SyncService {
    pub fn start(core: &Core, acc: &Accounts, data_dir: &Path, rt: &tokio::runtime::Handle) -> SyncService {
        let cfg = config(acc);
        let uplink = Arc::new(Uplink::new(cfg.clone(), data_dir, rt));
        let inara_due = cfg.inara_enabled();
        let sync = SyncService { uplink: Some(uplink), applied: cfg, inara_due, attachment: Default::default() };
        sync.attach(core);
        sync
    }

    /// No uploads at all (screenshot / replay runs).
    pub fn disabled() -> SyncService {
        SyncService { uplink: None, applied: UplinkConfig::default(), inara_due: false, attachment: Default::default() }
    }

    /// Call every frame: brings the Inara profile up to date when it is due (at start-up and
    /// when sync is switched on or the key changes) and the journal history has been read.
    pub fn tick(&mut self, core: &Core) {
        if !self.inara_due || !matches!(core.status.read().phase, Phase::Live) {
            return;
        }
        self.inara_due = false;
        let Some(u) = &self.uplink else { return };
        let model = core.model.read();
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
            let mut attachment = self.attachment.lock();
            *attachment = attachment.wrapping_add(1);
            u.source_changed();
            forward(core.subscribe(), u.clone(), self.attachment.clone(), *attachment);
        }
    }

    /// Call after the account settings changed (decrypts the keys, so not every frame).
    pub fn apply(&mut self, acc: &Accounts) {
        let Some(u) = &self.uplink else { return };
        let cfg = config(acc);
        if cfg != self.applied {
            // Switched on, a new key, or resumed after a pause: the profile is brought up to date.
            let inara_key = |c: &UplinkConfig| c.inara_credentials().map(|i| i.api_key.clone());
            if cfg.inara_enabled() && (!self.applied.inara_enabled() || inara_key(&cfg) != inara_key(&self.applied)) {
                self.inara_due = true;
            }
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

fn forward(rx: Receiver<LiveMsg>, uplink: Arc<Uplink>, attachment: Arc<parking_lot::Mutex<u64>>, owner: u64) {
    let spawned = std::thread::Builder::new().name("nexus-sync".into()).spawn(move || {
        // Watcher order is preserved (EDSM swaps Cargo/ShipLocker events for their companion files).
        let mut scope = None;
        for msg in rx {
            let active = attachment.lock();
            if *active != owner { break; }
            let context = match &msg {
                LiveMsg::Event(_, c) | LiveMsg::Companion(_, c) => c,
            };
            let current = (context.source_generation, context.commander_generation);
            if scope != Some(current) {
                uplink.source_changed();
                scope = Some(current);
            }
            match msg {
                LiveMsg::Event(e, c) => uplink.on_event(&e, &upload_context(&c)),
                LiveMsg::Companion(cmp, c) => uplink.on_companion(&cmp, &upload_context(&c)),
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

/// The uplink configuration for the user's choices (keys decrypted here, never logged).
pub fn config(acc: &Accounts) -> UplinkConfig {
    UplinkConfig {
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
        assert!(!config(&s.accounts).any_enabled());
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
