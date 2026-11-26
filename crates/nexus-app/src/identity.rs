//! Identifiers for Gen2-owned application state and operating-system surfaces.

pub const NAME: &str = "EliteNexus-Gen2";
pub const APP_ID: &str = "io.github.shadowgar.elitenexus-gen2";
pub const PORTABLE_DATA: &str = "nexus-gen2-data";
pub const REPOSITORY: &str = "https://github.com/Shadowgar/EliteNexus-Gen2";
pub const USER_AGENT: &str = concat!("EliteNexus-Gen2/", env!("CARGO_PKG_VERSION"));
