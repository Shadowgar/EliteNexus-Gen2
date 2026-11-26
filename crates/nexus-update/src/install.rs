//! The Gen2 executable and allowlisted documents are staged beside the existing program. The application profile
//! is outside this operation.

use crate::archive::{self, File};
use crate::{Asset, PRODUCT, PROGRAM};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Why this copy of Elite Nexus doesn't update itself. The app then says so and offers the release page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocked {
    /// Built from source: a debug build, or one in a cargo `target` folder.
    Source,
    /// Started from inside its zip, which Windows unpacks to a new temporary folder every time.
    InsideZip,
    /// Its folder can't be written to (Program Files, /usr/bin, a package manager's folder).
    ReadOnly(PathBuf),
    /// A Mac: replacing the app bundle hasn't been tried on a Mac yet.
    Mac,
    /// The running binary's name does not identify it as Gen2.
    Identity,
    /// No release archive is made for this computer (only Windows and Linux on x64 update themselves).
    Platform(String),
}

impl Blocked {
    /// In plain words, for the banner and Settings.
    pub fn why(&self) -> String {
        match self {
            Blocked::Source => "This copy was built from source: update it as you built it.".into(),
            Blocked::InsideZip => "Elite Nexus is running from inside its zip file, so it can't replace itself: unzip it into a folder of its own first.".into(),
            Blocked::ReadOnly(dir) => format!("Elite Nexus can't write to its own folder ({}), so it can't replace itself: update it the way you installed it.", dir.display()),
            Blocked::Mac => "On a Mac, Elite Nexus doesn't replace its app yet: download the new one and drag it to Applications, over the old one. Your data stays where it is.".into(),
            Blocked::Identity => "Self-updates require the original EliteNexus-Gen2 executable name.".into(),
            Blocked::Platform(p) => format!("Releases have no archive for this computer ({p}), so it can't update itself."),
        }
    }
}

/// Whether the program at `exe` can be replaced by a release archive, and if not, why.
pub fn installable(exe: &Path) -> Result<(), Blocked> {
    if exe.file_name().is_none_or(|name| name != PROGRAM) {
        return Err(Blocked::Identity);
    }
    installable_as(exe, cfg!(debug_assertions), &crate::manifest::this_platform())
}

fn installable_as(exe: &Path, debug: bool, platform: &str) -> Result<(), Blocked> {
    if platform.starts_with("macos") {
        return Err(Blocked::Mac);
    }
    if platform != "windows-x64" && platform != "linux-x64" {
        return Err(Blocked::Platform(platform.to_string()));
    }
    let dir = exe.parent().ok_or(Blocked::Source)?;
    let in_target = dir.components().any(|c| {
        let c = c.as_os_str().to_string_lossy().to_ascii_lowercase();
        c == "target" || c.starts_with("target-")
    });
    if debug || in_target {
        return Err(Blocked::Source);
    }
    if inside_zip(dir) {
        return Err(Blocked::InsideZip);
    }
    if !writable(dir) {
        return Err(Blocked::ReadOnly(dir.to_path_buf()));
    }
    Ok(())
}

/// Explorer runs a program opened from inside a zip from a temporary `Temp1_EliteNexus-….zip` folder.
fn inside_zip(dir: &Path) -> bool {
    dir.components().any(|c| {
        let name = c.as_os_str().to_string_lossy().to_ascii_lowercase();
        name.starts_with("temp") && name.ends_with(".zip")
    })
}

fn writable(dir: &Path) -> bool {
    let probe = dir.join(format!(".elite-nexus-gen2-update-{}", std::process::id()));
    match std::fs::File::create_new(&probe) {
        Ok(file) => {
            drop(file);
            let _ = std::fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

/// Where the new program waits while the old one runs: `EliteNexus-Gen2.new.exe` beside `EliteNexus-Gen2.exe`.
pub fn staged_path(exe: &Path) -> PathBuf {
    with_suffix(exe, ".new")
}

/// Where the old program goes when the new one takes its place (Windows, where a running program can be renamed but
/// not deleted): `EliteNexus-Gen2.old.exe`, deleted at the next approved update startup.
pub fn old_path(exe: &Path) -> PathBuf {
    with_suffix(exe, ".old")
}

/// The executable name with `suffix` before its extension.
fn with_suffix(exe: &Path, suffix: &str) -> PathBuf {
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| PRODUCT.into());
    let ext = exe.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    exe.with_file_name(format!("{stem}{suffix}{ext}"))
}

/// A verified archive staged beside the executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Staged {
    /// The staged program's SHA-256.
    pub sha256: String,
    /// Documents that couldn't be replaced (open in an editor, say), with why: they stay as they were.
    pub kept: Vec<String>,
}

/// Checks the archive against the asset previously authenticated by the signed manifest before writing any files.
pub fn verify_and_stage(exe: &Path, asset: &Asset, archive: &[u8], cancelled: impl FnOnce() -> bool) -> Result<Staged, String> {
    if exe.file_name().is_none_or(|name| name != PROGRAM) {
        return Err(Blocked::Identity.why());
    }
    if archive.len() as u64 != asset.size {
        return Err("The download isn't as large as the release says, so nothing was changed.".into());
    }
    if crate::sha256_hex(archive) != asset.sha256 {
        return Err("The download isn't the file the release signed, so nothing was changed.".into());
    }
    let kind = archive::Kind::of(&asset.name).ok_or_else(|| format!("{} isn't a Gen2 archive.", asset.name))?;
    let files = archive::extract(archive, kind, PROGRAM).map_err(|e| format!("The release archive: {e}."))?;
    if cancelled() {
        return Err("cancelled".into());
    }
    stage(exe, &files, PROGRAM).map_err(|e| format!("The new version couldn't be put beside this one ({e})."))
}

/// Stages the program and replaces the allowlisted release documents.
fn stage(exe: &Path, files: &[File], program: &str) -> io::Result<Staged> {
    let dir = exe.parent().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "the program has no folder"))?;
    let new = files.iter().find(|f| f.name == program).ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("{program} isn't in the archive")))?;
    write_replacing(&staged_path(exe), &new.bytes, true)?;
    let mut kept = Vec::new();
    for f in files.iter().filter(|f| f.name != program) {
        // A document that can't be replaced stays as it was: the program is what matters.
        if let Err(e) = write_replacing(&dir.join(&f.name), &f.bytes, false) {
            kept.push(format!("{}: {e}", f.name));
        }
    }
    Ok(Staged { sha256: crate::sha256_hex(&new.bytes), kept })
}

/// A file's half-written copy while it's being written: `README.md.part`.
fn part_path(path: &Path) -> PathBuf {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    path.with_file_name(format!("{name}.part"))
}

/// Writes `bytes` to a temporary file beside `path`, then moves it over `path`: a reader never sees half a file.
fn write_replacing(path: &Path, bytes: &[u8], program: bool) -> io::Result<()> {
    let tmp = part_path(path);
    let mut file = std::fs::File::create_new(&tmp)?;
    let result = file.write_all(bytes).and_then(|()| {
        #[cfg(unix)]
        if program {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o755))?;
        }
        let _ = program;
        file.sync_all()
    });
    drop(file);
    result.and_then(|()| std::fs::rename(&tmp, path)).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Whether the program waiting at [`staged_path`] is the one Elite Nexus downloaded (`sha256`), not a file something
/// else left there.
pub fn staged_matches(exe: &Path, sha256: &str) -> bool {
    std::fs::read(staged_path(exe)).is_ok_and(|b| crate::sha256_hex(&b) == sha256)
}

/// Gives up a staged program (a newer version is out, or the commander updated another way).
pub fn discard_staged(exe: &Path) {
    let _ = std::fs::remove_file(staged_path(exe));
}

/// Puts the staged program in place of `exe`, once `exe` no longer runs. Windows: `exe` moves aside to [`old_path`]
/// and the new program takes its name; if that fails, the old one moves back. Elsewhere the new program replaces it in
/// one step. Rechecks the staged bytes against the digest kept by the process that verified and staged them.
pub fn swap(exe: &Path, sha256: &str) -> io::Result<()> {
    if exe.file_name().is_none_or(|name| name != PROGRAM) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, Blocked::Identity.why()));
    }
    let staged = staged_path(exe);
    if !staged_matches(exe, sha256) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "the staged Gen2 executable is missing or has changed"));
    }
    if cfg!(windows) {
        let old = free_old_path(exe);
        swap_with_backup(exe, &staged, &old, rename_patiently)
    } else {
        rename_patiently(&staged, exe)
    }
}

fn swap_with_backup(exe: &Path, staged: &Path, old: &Path, mut rename: impl FnMut(&Path, &Path) -> io::Result<()>) -> io::Result<()> {
    rename(exe, old)?;
    if let Err(install_error) = rename(staged, exe) {
        rename(old, exe).map_err(|rollback_error| {
            io::Error::other(format!("{install_error}. Could not restore the previous executable from {}: {rollback_error}", old.display()))
        })?;
        return Err(install_error);
    }
    Ok(())
}

/// A rename, tried again for up to three seconds while something else still holds the file (a virus scanner reading
/// the program that was just written).
fn rename_patiently(from: &Path, to: &Path) -> io::Result<()> {
    let mut tries = 0;
    loop {
        match std::fs::rename(from, to) {
            Err(e) if tries < 10 && e.kind() != io::ErrorKind::NotFound => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            done => return done,
        }
    }
}

/// The old executable's path, or a numbered alternative while an older one is still in use.
fn free_old_path(exe: &Path) -> PathBuf {
    let first = old_path(exe);
    let _ = std::fs::remove_file(&first);
    if !first.exists() {
        return first;
    }
    for n in 2..100 {
        let p = with_suffix(exe, &format!(".old-{n}"));
        let _ = std::fs::remove_file(&p);
        if !p.exists() {
            return p;
        }
    }
    with_suffix(exe, &format!(".old-{}", std::process::id()))
}

/// Deletes what an update left beside the program: the old program(s) a swap moved aside, and half-written files.
/// Called at start; a file still in use stays until the next time.
pub fn clean_up(exe: &Path) {
    let Some(dir) = exe.parent() else { return };
    let old = old_path(exe);
    let old_name = old.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let numbered_old = name.starts_with(&format!("{stem}.old-"));
        let half_written = name.ends_with(".part") && (name.starts_with(&stem) || crate::archive::DOCUMENTS.iter().any(|d| name.starts_with(d)));
        if name == old_name || numbered_old || half_written {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Starts the program at `exe` with `args` and doesn't wait for it.
pub fn launch(exe: &Path, args: &[&str]) -> io::Result<()> {
    std::process::Command::new(exe).args(args).spawn().map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nexus-update-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    fn exe_in(dir: &Path) -> PathBuf {
        dir.join(PROGRAM)
    }

    fn files(program: &str) -> Vec<File> {
        vec![File { name: program.into(), bytes: b"new program".to_vec() }, File { name: "README.md".into(), bytes: b"new readme".to_vec() }]
    }

    #[test]
    fn names_beside_the_program() {
        assert_eq!(staged_path(Path::new("C:/Games/EliteNexus-Gen2/EliteNexus-Gen2.exe")), Path::new("C:/Games/EliteNexus-Gen2/EliteNexus-Gen2.new.exe"));
        assert_eq!(old_path(Path::new("C:/Games/EliteNexus-Gen2/EliteNexus-Gen2.exe")), Path::new("C:/Games/EliteNexus-Gen2/EliteNexus-Gen2.old.exe"));
        assert_eq!(staged_path(Path::new("/home/cmdr/EliteNexus-Gen2/EliteNexus-Gen2")), Path::new("/home/cmdr/EliteNexus-Gen2/EliteNexus-Gen2.new"));
    }

    /// A test key signs the exact bytes and metadata used by the production manifest check.
    fn signed_asset(bytes: &[u8]) -> Asset {
        let tag = "v0.8.4";
        let name = "EliteNexus-Gen2-v0.8.4-windows-x64.zip";
        let asset = Asset { name: name.into(), url: crate::release_file(tag, name), size: bytes.len() as u64, sha256: crate::sha256_hex(bytes) };
        let manifest = crate::Manifest {
            schema: crate::manifest::SCHEMA,
            version: "0.8.4".into(),
            tag: tag.into(),
            product: PRODUCT.into(),
            repository: crate::REPO.into(),
            assets: [("windows-x64".into(), asset)].into_iter().collect(),
            ..Default::default()
        };
        let json = manifest.to_json();
        let (public, signature) = crate::manifest::tests::signed(json.as_bytes());
        let manifest = crate::Manifest::verified(json.as_bytes(), &signature, &public).expect("signed manifest");
        assert!(crate::Version::parse("0.8.3").expect("version").accepts_update(&manifest.version().expect("release version")));
        manifest.asset("windows-x64").expect("asset").clone()
    }

    #[test]
    fn a_signed_gen2_archive_stages_then_swaps_and_preserves_both_profiles() {
        let dir = temp("swap");
        let exe = exe_in(&dir);
        std::fs::write(&exe, b"old program").expect("exe");
        std::fs::write(dir.join("README.md"), b"old readme").expect("readme");
        let profiles = [("nexus-data", b"upstream profile"), ("nexus-gen2-data", b"Gen2 settings   ")];
        for (folder, contents) in profiles {
            std::fs::create_dir(dir.join(folder)).expect("data");
            std::fs::write(dir.join(folder).join("settings.json"), contents).expect("settings");
        }

        let archive = archive::tests::zip_of(&[
            (PROGRAM, b"new program"), ("README.md", b"new readme"),
            ("nexus-gen2-data/settings.json", b"ignore"), ("nexus-data/settings.json", b"ignore"),
        ]);
        let staged = verify_and_stage(&exe, &signed_asset(&archive), &archive, || false).expect("stage");
        let sha = staged.sha256;
        assert!(staged.kept.is_empty(), "{:?}", staged.kept);
        assert_eq!(sha, crate::sha256_hex(b"new program"));
        assert!(staged_matches(&exe, &sha));
        assert!(!staged_matches(&exe, &crate::sha256_hex(b"something else")));
        assert_eq!(std::fs::read(&exe).expect("exe"), b"old program", "the running program is left alone");
        assert_eq!(std::fs::read(dir.join("README.md")).expect("readme"), b"new readme");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(staged_path(&exe)).expect("staged").permissions().mode();
            assert_eq!(mode & 0o111, 0o111, "the new program can run");
        }

        swap(&exe, &sha).expect("swap");
        assert_eq!(std::fs::read(&exe).expect("exe"), b"new program");
        assert!(!staged_path(&exe).exists());
        for (folder, contents) in profiles {
            assert_eq!(std::fs::read(dir.join(folder).join("settings.json")).expect("settings"), contents);
        }
        if cfg!(windows) {
            assert_eq!(std::fs::read(old_path(&exe)).expect("old"), b"old program", "moved aside, deleted at the next start");
        }
        clean_up(&exe);
        assert!(!old_path(&exe).exists());
        assert!(exe.exists());
        assert!(dir.join("README.md").exists());
        // Nothing staged: an error, and nothing changes.
        assert!(swap(&exe, &sha).is_err());
        assert_eq!(std::fs::read(&exe).expect("exe"), b"new program");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_or_upstream_archive_cannot_stage_a_program_or_documents() {
        let dir = temp("bad-archive");
        let exe = exe_in(&dir);
        std::fs::write(&exe, b"old program").expect("exe");
        let docs = dir.join("README.md");
        std::fs::write(&docs, b"old readme").expect("readme");
        let mut archive = archive::tests::zip_of(&[(PROGRAM, b"new program"), ("README.md", b"new readme")]);
        let asset = signed_asset(&archive);
        archive[0] ^= 1;
        assert!(verify_and_stage(&exe, &asset, &archive, || false).is_err(), "changed after signing");
        assert!(verify_and_stage(&exe, &signed_asset(&archive), &archive, || false).is_err(), "signed corrupt archive");
        let upstream = if cfg!(windows) { "EliteNexus.exe" } else { "EliteNexus" };
        let archive = archive::tests::zip_of(&[(upstream, b"upstream"), ("README.md", b"upstream readme")]);
        assert!(verify_and_stage(&exe, &signed_asset(&archive), &archive, || false).is_err(), "upstream program in a renamed archive");
        assert!(!staged_path(&exe).exists());
        assert_eq!(std::fs::read(&exe).expect("exe"), b"old program");
        assert_eq!(std::fs::read(&docs).expect("readme"), b"old readme");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn interrupted_staging_leaves_the_installed_program_intact_and_allows_retry() {
        let dir = temp("interrupted");
        let exe = exe_in(&dir);
        std::fs::write(&exe, b"old program").expect("exe");
        let archive = archive::tests::zip_of(&[(PROGRAM, b"new program")]);
        let asset = signed_asset(&archive);
        assert!(verify_and_stage(&exe, &asset, &archive, || true).is_err(), "cancel before writing");
        assert!(!staged_path(&exe).exists());
        let part = part_path(&staged_path(&exe));
        std::fs::write(&part, b"incomplete staging").expect("interrupted file");
        assert!(swap(&exe, &crate::sha256_hex(b"new program")).is_err());
        assert_eq!(std::fs::read(&exe).expect("exe"), b"old program");
        clean_up(&exe);
        assert!(!part.exists());
        let staged = verify_and_stage(&exe, &asset, &archive, || false).expect("retry");
        std::fs::write(staged_path(&exe), b"changed after staging").expect("staged");
        assert!(swap(&exe, &staged.sha256).is_err(), "the waiting file is authenticated again");
        assert_eq!(std::fs::read(&exe).expect("exe"), b"old program");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_windows_install_rename_restores_the_old_executable() {
        let dir = temp("rollback");
        let exe = exe_in(&dir);
        let old = old_path(&exe);
        let staged = staged_path(&exe);
        std::fs::write(&exe, b"old program").expect("exe");
        std::fs::write(&staged, b"new program").expect("staged");
        let result = swap_with_backup(&exe, &staged, &old, |from, to| {
            if from == staged {
                return Err(io::Error::other("synthetic install interruption"));
            }
            std::fs::rename(from, to)
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&exe).expect("restored exe"), b"old program");
        assert_eq!(std::fs::read(&staged).expect("staged"), b"new program");
        assert!(!old.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_staged_file_can_be_given_up() {
        let dir = temp("discard");
        let exe = exe_in(&dir);
        std::fs::write(&exe, b"old").expect("exe");
        let program = exe.file_name().expect("name").to_string_lossy().into_owned();
        stage(&exe, &files(&program), &program).expect("stage");
        discard_staged(&exe);
        assert!(!staged_path(&exe).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copies_that_dont_update_themselves() {
        let dir = temp("installable");
        let exe = exe_in(&dir);
        assert_eq!(installable_as(&exe, false, "windows-x64"), Ok(()));
        assert_eq!(installable_as(&exe, false, "linux-x64"), Ok(()));
        assert_eq!(installable_as(&exe, true, "windows-x64"), Err(Blocked::Source), "a debug build");
        assert_eq!(installable_as(Path::new("C:/src/EliteNexus/target/release/EliteNexus.exe"), false, "windows-x64"), Err(Blocked::Source));
        assert_eq!(installable_as(Path::new("C:/src/EliteNexus/target-agents/me/release/EliteNexus.exe"), false, "windows-x64"), Err(Blocked::Source));
        assert_eq!(installable_as(Path::new("C:/Users/c/AppData/Local/Temp/Temp1_EliteNexus-v0.8.3-windows-x64.zip/EliteNexus.exe"), false, "windows-x64"), Err(Blocked::InsideZip));
        assert_eq!(installable_as(&exe, false, "macos-arm64"), Err(Blocked::Mac));
        assert_eq!(installable_as(&exe, false, "windows-arm64"), Err(Blocked::Platform("windows-arm64".into())));
        let missing = dir.join("no such folder").join("EliteNexus.exe");
        assert!(matches!(installable_as(&missing, false, "windows-x64"), Err(Blocked::ReadOnly(_))));
        for b in [Blocked::Source, Blocked::InsideZip, Blocked::ReadOnly(dir.clone()), Blocked::Mac, Blocked::Identity, Blocked::Platform("x".into())] {
            assert!(b.why().ends_with('.'), "{b:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
