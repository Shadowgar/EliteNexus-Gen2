//! Gen2's signed product, repository, version and archive metadata.

use crate::version::Version;
use crate::{PRODUCT, REPO, release_file};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The manifest format this version reads and writes.
pub const SCHEMA: u32 = 2;

/// The most a manifest may weigh.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
pub const MAX_ARCHIVE_BYTES: usize = 600 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Manifest {
    /// The format, [`SCHEMA`].
    pub schema: u32,
    pub product: String,
    pub repository: String,
    /// The release's version, without the `v` (`0.8.3`).
    pub version: String,
    /// Its tag (`v0.8.3`).
    pub tag: String,
    /// The day it was released (YYYY-MM-DD).
    pub date: String,
    /// The archive for each platform (`windows-x64`, `linux-x64`, `macos-arm64`), as the release page offers them.
    /// A platform the release has no archive for is missing (a release goes out without the Mac zip if its build
    /// fails).
    pub assets: BTreeMap<String, Asset>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Asset {
    /// The file's name on the release page (`Praetorium-v0.8.3-windows-x64.zip`).
    pub name: String,
    /// Where it downloads from.
    pub url: String,
    /// Its size in bytes.
    pub size: u64,
    /// Its SHA-256, lowercase hex.
    pub sha256: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("it doesn't match its signature: it was changed or damaged on the way")]
    BadSignature,
    #[error("its signing key does not match the trusted Gen2 key")]
    WrongKey,
    #[error("its signature can't be read ({0})")]
    Signature(String),
    #[error("the Gen2 public key can't be read ({0})")]
    Key(String),
    #[error("it can't be read ({0})")]
    Format(String),
    #[error("it's in a newer format ({0}) than this version reads")]
    NewerFormat(u32),
    #[error("format {0} is ineligible for Gen2 updates")]
    UnsupportedFormat(u32),
    #[error("its product or repository does not match Praetorium")]
    WrongIdentity,
    #[error("it names no canonical release version and tag")]
    NoVersion,
    #[error("its archive metadata is ineligible for platform {0}")]
    InvalidAsset(String),
    #[error("it names no Gen2 release archives")]
    NoAssets,
}

impl Manifest {
    /// Checks `bytes` against `signature` (the text of `update.json.minisig`) with `public_key` (a minisign public key
    /// file, or its base64 line), then reads them. Nothing is read from a manifest whose signature doesn't hold.
    pub fn verified(bytes: &[u8], signature: &str, public_key: &str) -> Result<Manifest, ManifestError> {
        if bytes.len() > MAX_MANIFEST_BYTES || signature.len() > 4096 {
            return Err(ManifestError::Format("manifest or signature exceeds its size limit".into()));
        }
        let key = public_key.trim();
        let key = if key.lines().count() > 1 { minisign_verify::PublicKey::decode(key) } else { minisign_verify::PublicKey::from_base64(key) };
        let key = key.map_err(|e| ManifestError::Key(e.to_string()))?;
        let signature = minisign_verify::Signature::decode(signature.trim()).map_err(|e| ManifestError::Signature(e.to_string()))?;
        // Require prehashed Minisign signatures.
        key.verify(bytes, &signature, false).map_err(|e| match e {
            minisign_verify::Error::InvalidSignature => ManifestError::BadSignature,
            minisign_verify::Error::UnexpectedKeyId => ManifestError::WrongKey,
            e => ManifestError::Signature(e.to_string()),
        })?;
        Manifest::read(bytes)
    }

    /// Reads a manifest without checking a signature (the signer, tests).
    pub fn read(bytes: &[u8]) -> Result<Manifest, ManifestError> {
        let m: Manifest = serde_json::from_slice(bytes).map_err(|e| ManifestError::Format(e.to_string()))?;
        m.validate()?;
        Ok(m)
    }

    fn validate(&self) -> Result<(), ManifestError> {
        if self.schema > SCHEMA {
            return Err(ManifestError::NewerFormat(self.schema));
        }
        if self.schema != SCHEMA {
            return Err(ManifestError::UnsupportedFormat(self.schema));
        }
        if self.product != PRODUCT || self.repository != REPO {
            return Err(ManifestError::WrongIdentity);
        }
        if self.version().is_none_or(|v| v.to_string() != self.version) || self.tag != format!("v{}", self.version) {
            return Err(ManifestError::NoVersion);
        }
        if self.assets.is_empty() {
            return Err(ManifestError::NoAssets);
        }
        for (platform, asset) in &self.assets {
            if !self.valid_asset(platform, asset) {
                return Err(ManifestError::InvalidAsset(platform.clone()));
            }
        }
        Ok(())
    }

    pub fn version(&self) -> Option<Version> {
        Version::parse(&self.version)
    }

    /// The archive for `platform`, if the release has one with what it takes to download and check it.
    pub fn asset(&self, platform: &str) -> Option<&Asset> {
        self.assets.get(platform).filter(|a| self.valid_asset(platform, a))
    }

    fn valid_asset(&self, platform: &str, asset: &Asset) -> bool {
        platform_of(&asset.name, &self.version) == Some(platform)
            && asset.url == release_file(&self.tag, &asset.name)
            && (1..=MAX_ARCHIVE_BYTES as u64).contains(&asset.size)
            && asset.sha256.len() == 64
            && asset.sha256.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    }

    /// Writes the unsigned metadata for the canonical Gen2 archives found in `dir`.
    pub fn from_release_files(dir: &Path, tag: &str, date: &str) -> std::io::Result<Manifest> {
        let version = tag.strip_prefix('v').unwrap_or(tag).to_string();
        let mut m = Manifest {
            schema: SCHEMA, product: PRODUCT.into(), repository: REPO.into(), version,
            tag: tag.into(), date: date.into(), assets: BTreeMap::new(),
        };
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(platform) = platform_of(name, &m.version) else { continue };
            if !entry.file_type()?.is_file() {
                continue;
            }
            if entry.metadata()?.len() > MAX_ARCHIVE_BYTES as u64 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, format!("oversized archive: {name}")));
            }
            let bytes = std::fs::read(entry.path())?;
            let asset = Asset { url: release_file(tag, name), size: bytes.len() as u64, sha256: crate::sha256_hex(&bytes), name: name.into() };
            m.assets.insert(platform.into(), asset);
        }
        m.validate().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(m)
    }

    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).unwrap_or_default();
        s.push('\n');
        s
    }
}

/// The platform of a canonical Gen2 archive. Each platform has one archive format.
pub fn platform_of<'a>(file_name: &'a str, version: &str) -> Option<&'a str> {
    let rest = file_name.strip_prefix(PRODUCT)?.strip_prefix("-v")?.strip_prefix(version)?.strip_prefix('-')?;
    match rest {
        "windows-x64.zip" | "macos-arm64.zip" | "macos-x86_64.zip" => rest.strip_suffix(".zip"),
        "linux-x64.tar.gz" => rest.strip_suffix(".tar.gz"),
        _ => None,
    }
}

/// This build's platform as release archives name it: `windows-x64`, `linux-x64`, `macos-arm64` (the Mac zip is named
/// after `uname -m`, so an Intel Mac is `macos-x86_64`).
pub fn this_platform() -> String {
    let arch = std::env::consts::ARCH;
    match std::env::consts::OS {
        "macos" => format!("macos-{}", if arch == "aarch64" { "arm64" } else { arch }),
        os => format!(
            "{os}-{}",
            match arch {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                a => a,
            }
        ),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A key pair of the tests' own and a signature for `bytes` with it: (public key file, signature file).
    pub(crate) fn signed(bytes: &[u8]) -> (String, String) {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().expect("key pair");
        let public = pair.pk.to_box().expect("public key box").into_string();
        let sig = minisign::sign(Some(&pair.pk), &pair.sk, std::io::Cursor::new(bytes), Some("test"), None).expect("signature").into_string();
        (public, sig)
    }

    fn sample() -> Manifest {
        let mut assets = BTreeMap::new();
        assets.insert(
            "windows-x64".to_string(),
            Asset { name: "Praetorium-v0.8.4-windows-x64.zip".into(), url: "https://github.com/Shadowgar/Praetorium/releases/download/v0.8.4/Praetorium-v0.8.4-windows-x64.zip".into(), size: 1234, sha256: "a".repeat(64) },
        );
        Manifest { schema: SCHEMA, product: PRODUCT.into(), repository: REPO.into(), version: "0.8.4".into(), tag: "v0.8.4".into(), date: "2026-12-01".into(), assets }
    }

    #[test]
    fn an_upstream_manifest_is_refused_even_with_a_valid_test_signature() {
        let json = br#"{"schema":1,"version":"0.8.4","tag":"v0.8.4","assets":{"windows-x64":{"name":"EliteNexus-v0.8.4-windows-x64.zip","url":"https://github.com/uruskan/EliteNexus/releases/download/v0.8.4/EliteNexus-v0.8.4-windows-x64.zip","size":3,"sha256":"0000000000000000000000000000000000000000000000000000000000000000"}}}"#;
        let (public, sig) = signed(json);
        assert!(matches!(Manifest::verified(json, &sig, &public), Err(ManifestError::UnsupportedFormat(1))));
    }

    #[test]
    fn a_gen2_manifest_cannot_authorize_an_upstream_download() {
        let json = br#"{"schema":2,"product":"Praetorium","repository":"Shadowgar/Praetorium","version":"0.8.4","tag":"v0.8.4","assets":{"windows-x64":{"name":"Praetorium-v0.8.4-windows-x64.zip","url":"https://github.com/uruskan/EliteNexus/releases/download/v0.8.4/Praetorium-v0.8.4-windows-x64.zip","size":3,"sha256":"0000000000000000000000000000000000000000000000000000000000000000"}}}"#;
        let (public, sig) = signed(json);
        assert!(matches!(Manifest::verified(json, &sig, &public), Err(ManifestError::InvalidAsset(p)) if p == "windows-x64"));
    }

    #[test]
    fn a_signed_praetorium_manifest_needs_matching_identity_tag_and_archive() {
        let json = sample().to_json();
        for (original, changed) in [
            ("\"Praetorium\"", "\"EliteNexus\""),
            ("\"Shadowgar/Praetorium\"", "\"uruskan/EliteNexus\""),
            ("\"Praetorium\"", "\"EliteNexus-Gen2\""),
            ("\"Shadowgar/Praetorium\"", "\"Shadowgar/EliteNexus-Gen2\""),
            ("Praetorium-v0.8.4-windows-x64.zip", "EliteNexus-Gen2-v0.8.4-windows-x64.zip"),
            ("\"v0.8.4\"", "\"v0.8.5\""),
            ("\"0.8.4\"", "\"0.8.4+local\""),
            ("Praetorium-v0.8.4-windows-x64.zip", "EliteNexus-v0.8.4-windows-x64.zip"),
            ("windows-x64.zip", "windows-x64.tar.gz"),
        ] {
            let invalid = json.replace(original, changed);
            let (public, sig) = signed(invalid.as_bytes());
            assert!(Manifest::verified(invalid.as_bytes(), &sig, &public).is_err(), "accepted {changed}");
        }
    }

    #[test]
    fn a_signed_manifest_reads_and_a_changed_one_doesnt() {
        let json = sample().to_json();
        let (public, sig) = signed(json.as_bytes());
        let m = Manifest::verified(json.as_bytes(), &sig, &public).expect("verified");
        assert_eq!(m, sample());
        assert_eq!(m.version().map(|v| v.to_string()).as_deref(), Some("0.8.4"));
        // The base64 line alone works as the key too.
        let line = public.lines().nth(1).expect("key line");
        assert!(Manifest::verified(json.as_bytes(), &sig, line).is_ok());
        // One byte changed: refused.
        let changed = json.replace("1234", "1235");
        assert!(matches!(Manifest::verified(changed.as_bytes(), &sig, &public), Err(ManifestError::BadSignature)));
        // Another key: refused.
        let (other, _) = signed(b"other");
        assert!(matches!(Manifest::verified(json.as_bytes(), &sig, &other), Err(ManifestError::WrongKey)));
        // Garbage for a signature or a key.
        assert!(matches!(Manifest::verified(json.as_bytes(), "not a signature", &public), Err(ManifestError::Signature(_))));
        assert!(matches!(Manifest::verified(json.as_bytes(), &sig, "not a key"), Err(ManifestError::Key(_))));
    }

    #[test]
    fn unknown_fields_are_ignored_and_newer_formats_reported() {
        let json = sample().to_json().replacen('{', r#"{"note":"later field","#, 1);
        let m = Manifest::read(json.as_bytes()).expect("reads");
        assert!(m.asset("windows-x64").is_some());
        assert!(m.asset("linux-x64").is_none(), "no archive for this platform");
        assert!(matches!(Manifest::read(br#"{"schema":3,"version":"1.0.0"}"#), Err(ManifestError::NewerFormat(3))));
        let bad_version = json.replace("\"0.8.4\"", "\"soon\"");
        assert!(matches!(Manifest::read(bad_version.as_bytes()), Err(ManifestError::NoVersion)));
        assert!(matches!(Manifest::read(b"<html>"), Err(ManifestError::Format(_))));
        // An archive without what it takes to download and check it isn't offered.
        let mut m = sample();
        m.assets.get_mut("windows-x64").expect("asset").url = "http://insecure/a.zip".into();
        assert!(m.asset("windows-x64").is_none());
    }

    #[test]
    fn release_file_names_give_their_platform() {
        assert_eq!(platform_of("Praetorium-v0.8.3-windows-x64.zip", "0.8.3"), Some("windows-x64"));
        assert_eq!(platform_of("Praetorium-v0.8.3-linux-x64.tar.gz", "0.8.3"), Some("linux-x64"));
        assert_eq!(platform_of("Praetorium-v0.9.0-beta.1-macos-arm64.zip", "0.9.0-beta.1"), Some("macos-arm64"));
        assert_eq!(platform_of("Praetorium-v0.8.3-windows-x64.zip.sha256", "0.8.3"), None);
        assert_eq!(platform_of("Praetorium-v0.8.2-windows-x64.zip", "0.8.3"), None, "another version");
        assert_eq!(platform_of("EliteNexus-v0.8.3-windows-x64.zip", "0.8.3"), None, "upstream artifact");
        assert_eq!(platform_of("update.json", "0.8.3"), None);
        let p = this_platform();
        assert!(p.contains('-') && !p.contains(' '), "{p}");
        if cfg!(all(windows, target_arch = "x86_64")) {
            assert_eq!(p, "windows-x64");
        }
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert_eq!(p, "linux-x64");
        }
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert_eq!(p, "macos-arm64");
        }
    }

    #[test]
    fn the_manifest_of_a_release_folder() {
        let dir = std::env::temp_dir().join(format!("nexus-update-manifest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        for (name, body) in [("Praetorium-v0.8.4-windows-x64.zip", "zip"), ("Praetorium-v0.8.4-windows-x64.zip.sha256", "sum"), ("Praetorium-v0.8.4-linux-x64.tar.gz", "tar"), ("EliteNexus-v0.8.4-macos-arm64.zip", "upstream"), ("notes.md", "notes")] {
            std::fs::write(dir.join(name), body).expect("file");
        }
        let m = Manifest::from_release_files(&dir, "v0.8.4", "2026-12-01").expect("manifest");
        assert_eq!(m.version, "0.8.4");
        assert_eq!(m.assets.keys().collect::<Vec<_>>(), ["linux-x64", "windows-x64"]);
        let w = &m.assets["windows-x64"];
        assert_eq!(w.size, 3);
        assert_eq!(w.sha256, crate::sha256_hex(b"zip"));
        assert_eq!(w.url, "https://github.com/Shadowgar/Praetorium/releases/download/v0.8.4/Praetorium-v0.8.4-windows-x64.zip");
        // What it writes, it reads.
        assert_eq!(Manifest::read(m.to_json().as_bytes()).expect("reads"), m);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
