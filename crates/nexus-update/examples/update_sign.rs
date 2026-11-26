//! Offline Gen2 manifest tooling. Neither command configures trust in a deployed app.
//!
//!   cargo run --locked -p nexus-update --example update_sign -- manifest <folder> <tag> <date>
//!   cargo run --locked -p nexus-update --example update_sign -- verify <file> <public key file>

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let done = match args.as_slice() {
        [command, dir, tag, date] if command == "manifest" => manifest(Path::new(dir), tag, date),
        [command, file, key] if command == "verify" => verify(Path::new(file), Path::new(key)),
        _ => Err("usage: update_sign manifest <folder> <tag> <date> | verify <file> <public key file>".to_string()),
    };
    match done {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("update_sign: {e}");
            ExitCode::FAILURE
        }
    }
}

fn manifest(dir: &Path, tag: &str, date: &str) -> Result<String, String> {
    let manifest = nexus_update::Manifest::from_release_files(dir, tag, date).map_err(|e| format!("{}: {e}", dir.display()))?;
    let out = dir.join(nexus_update::MANIFEST);
    std::fs::write(&out, manifest.to_json()).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(format!("Wrote unsigned {} for Gen2 {}", out.display(), manifest.version))
}

fn verify(file: &Path, key: &Path) -> Result<String, String> {
    let key = std::fs::read_to_string(key).map_err(|e| format!("{}: {e}", key.display()))?;
    let bytes = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
    let mut sig_file = file.as_os_str().to_owned();
    sig_file.push(".minisig");
    let sig_file = Path::new(&sig_file);
    let sig = std::fs::read_to_string(sig_file).map_err(|e| format!("{}: {e}", sig_file.display()))?;
    let manifest = nexus_update::Manifest::verified(&bytes, &sig, &key).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(format!("Verified {} for Gen2 {} with the supplied public key", file.display(), manifest.version))
}
