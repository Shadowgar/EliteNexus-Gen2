//! Identifiers for Praetorium-owned application state and operating-system surfaces.

pub const NAME: &str = nexus_update::PRODUCT;
pub const APP_ID: &str = "io.github.shadowgar.praetorium";
pub const PORTABLE_DATA: &str = "praetorium-data";
pub const REPOSITORY: &str = "https://github.com/Shadowgar/Praetorium";
pub const USER_AGENT: &str = concat!("Praetorium/", env!("CARGO_PKG_VERSION"));
pub const ATTRIBUTION: &str = "Praetorium is an independently maintained, modified version of Elite Nexus by CMDR Alka Uruskan and its contributors. It is not an official Elite Nexus release, successor or endorsed product.";
