//! The Gen2 update protocol and local installer. Production updates require an owner-approved Gen2 release key.

pub mod archive;
pub mod install;
pub mod manifest;
pub mod notes;
pub mod version;

pub use manifest::{Asset, Manifest, ManifestError};
pub use version::Version;

pub const PRODUCT: &str = "EliteNexus-Gen2";
pub const PROGRAM: &str = if cfg!(windows) { "EliteNexus-Gen2.exe" } else { PRODUCT };
/// The repository authorized by the Gen2 manifest policy.
pub const REPO: &str = "Shadowgar/EliteNexus-Gen2";
/// The manifest's file name in every release.
pub const MANIFEST: &str = "update.json";
/// Its signature's.
pub const SIGNATURE: &str = "update.json.minisig";
/// Must be provisioned in a separately reviewed change after Gen2 signing ownership is approved.
pub const PRODUCTION_PUBLIC_KEY: Option<&str> = None;
/// Every release, newest first.
pub const RELEASES_PAGE: &str = "https://github.com/Shadowgar/EliteNexus-Gen2/releases";
/// The releases' feed: stable releases and betas alike, without the API's hourly limit.
pub const FEED: &str = "https://github.com/Shadowgar/EliteNexus-Gen2/releases.atom";

/// A file of the latest stable release: GitHub's download address, not its API, so no hourly limit applies. A beta is
/// never the latest release.
pub fn latest_file(file: &str) -> String {
    format!("https://github.com/{REPO}/releases/latest/download/{file}")
}

/// A file of the release tagged `tag`.
pub fn release_file(tag: &str, file: &str) -> String {
    format!("https://github.com/{REPO}/releases/download/{tag}/{file}")
}

/// The release page of `tag`.
pub fn release_page(tag: &str) -> String {
    format!("https://github.com/{REPO}/releases/tag/{tag}")
}

/// CHANGELOG.md as it was at `tag`: "What's new" for a version not installed yet.
pub fn changelog_at(tag: &str) -> String {
    format!("https://raw.githubusercontent.com/{REPO}/{tag}/CHANGELOG.md")
}

/// The release tags in the releases' feed (`releases.atom`), as its entries link to them (`…/releases/tag/v0.9.0`).
pub fn feed_tags(feed: &str) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for part in feed.split(&format!("{RELEASES_PAGE}/tag/")).skip(1) {
        let tag: String = part.chars().take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+')).collect();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

/// The newest version among `tags`, betas included, and its tag.
pub fn newest_tag(tags: &[String]) -> Option<(Version, String)> {
    tags.iter().filter_map(|t| Version::parse(t).map(|v| (v, t.clone()))).max_by(|a, b| a.0.cmp(&b.0))
}

/// SHA-256 in lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feed_gives_its_tags_and_the_newest_wins() {
        let feed = r#"<feed><entry><id>tag:github.com,2008:Repository/1/v0.9.0-beta.1</id><link rel="alternate" type="text/html" href="https://github.com/Shadowgar/EliteNexus-Gen2/releases/tag/v0.9.0-beta.1"/></entry>
<entry><link href="https://github.com/Shadowgar/EliteNexus-Gen2/releases/tag/v0.8.3"/></entry><entry><link href="https://github.com/Shadowgar/EliteNexus-Gen2/releases/tag/v0.8.3"/></entry>
<entry><link href="https://github.com/uruskan/EliteNexus/releases/tag/v99.0.0"/></entry>
<entry><link href="https://github.com/Shadowgar/EliteNexus-Gen2/releases/tag/nightly"/></entry></feed>"#;
        let tags = feed_tags(feed);
        assert_eq!(tags, ["v0.9.0-beta.1", "v0.8.3", "nightly"]);
        let (v, tag) = newest_tag(&tags).expect("newest");
        assert_eq!((v.to_string().as_str(), tag.as_str()), ("0.9.0-beta.1", "v0.9.0-beta.1"));
        assert!(newest_tag(&feed_tags("<feed/>")).is_none());
    }

    #[test]
    fn addresses() {
        assert_eq!(latest_file(MANIFEST), "https://github.com/Shadowgar/EliteNexus-Gen2/releases/latest/download/update.json");
        assert_eq!(release_file("v0.9.0-beta.1", SIGNATURE), "https://github.com/Shadowgar/EliteNexus-Gen2/releases/download/v0.9.0-beta.1/update.json.minisig");
        assert_eq!(changelog_at("v0.8.3"), "https://raw.githubusercontent.com/Shadowgar/EliteNexus-Gen2/v0.8.3/CHANGELOG.md");
        assert!(FEED.starts_with(RELEASES_PAGE));
        assert_eq!(sha256_hex(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

}
