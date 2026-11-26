use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const GPU_RESET: &str = "--gpu-reset";
pub const DATA_HOME: &str = "ELITE_NEXUS_GEN2_HOME";

static ISOLATED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub fn initialize(args: &[OsString]) -> bool {
    let isolated = is_isolated(args);
    let _ = ISOLATED.set(isolated);
    isolated
}

pub fn isolated() -> bool {
    ISOLATED.get().copied().unwrap_or(false)
}

fn is_isolated(args: &[OsString]) -> bool {
    args.iter().any(|arg| arg == "--isolated" || arg == "--journal-dir" || arg == "--screenshot")
}

pub fn journal_argument(args: &[OsString]) -> std::io::Result<Option<&OsStr>> {
    let Some(index) = args.iter().position(|word| word == "--journal-dir") else { return Ok(None) };
    args.get(index + 1)
        .filter(|path| !path.is_empty() && !path.to_string_lossy().starts_with("--"))
        .map(|path| Some(path.as_os_str()))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "--journal-dir needs a directory"))
}

pub fn recovery_arguments(original: &[OsString]) -> Vec<OsString> {
    let mut args = original.to_vec();
    if !args.iter().any(|arg| arg == GPU_RESET) {
        args.push(GPU_RESET.into());
    }
    args
}

pub struct Recovery {
    exe: PathBuf,
    cwd: PathBuf,
    data_dir: PathBuf,
    args: Vec<OsString>,
}

impl Recovery {
    pub fn capture(original: &[OsString], data_dir: &Path, journal_dir: Option<&Path>) -> Option<Self> {
        let exe = std::env::current_exe().ok()?;
        let cwd = std::env::current_dir().ok()?;
        Self::prepare(exe, cwd, data_dir, original, journal_dir)
    }

    fn prepare(exe: PathBuf, cwd: PathBuf, data_dir: &Path, original: &[OsString], journal_dir: Option<&Path>) -> Option<Self> {
        if original.iter().any(|arg| arg == "--screenshot") {
            return None;
        }
        let mut args = recovery_arguments(original);
        if let Some(dir) = journal_dir {
            let dir = cwd.join(dir).into_os_string();
            if let Some(index) = original.iter().position(|word| word == "--journal-dir") {
                *args.get_mut(index + 1)? = dir;
            } else {
                args.extend([OsString::from("--journal-dir"), dir]);
            }
        }
        Some(Self { exe, data_dir: cwd.join(data_dir), cwd, args })
    }

    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.exe);
        command.current_dir(&self.cwd).args(&self.args).env(DATA_HOME, &self.data_dir);
        command
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_restart_pins_the_profile_and_working_directory() {
        let cwd = std::env::temp_dir();
        let plan = Recovery::prepare(cwd.join("fixture.exe"), cwd.clone(), Path::new("fixture profile"), &[OsString::from("--isolated")], Some(Path::new("fixture journals"))).unwrap();
        let command = plan.command();
        assert_eq!(command.get_current_dir(), Some(cwd.as_path()));
        let args: Vec<_> = command.get_args().map(OsStr::to_owned).collect();
        let source = cwd.join("fixture journals");
        assert_eq!(journal_argument(&args).unwrap(), Some(source.as_os_str()));
        assert!(journal_argument(&[OsString::from("--journal-dir"), OsString::from("--mute")]).is_err());
        let expected = cwd.join("fixture profile");
        assert_eq!(command.get_envs().find(|(name, _)| *name == DATA_HOME).and_then(|(_, value)| value), Some(expected.as_os_str()));
        assert!(Recovery::prepare(cwd.join("fixture.exe"), cwd, Path::new("fixture profile"), &[OsString::from("--screenshot")], None).is_none());
    }

    #[test]
    fn fixture_switches_preserve_isolation_under_recovery() {
        assert!(!is_isolated(&[OsString::from("--mute")]));
        for fixture in ["--journal-dir", "--screenshot", "--isolated"] {
            let args = recovery_arguments(&[OsString::from(fixture)]);
            assert!(is_isolated(&args), "{fixture}");
        }
    }

    #[test]
    fn recovery_keeps_fixture_flags_and_arguments_with_spaces() {
        let args: Vec<_> = [
            "--isolated", "--journal-dir", "test source / journals", "--mute",
            "--page", "Logbook", "--size", "1100x700", "--tab", "travel",
        ].into_iter().map(OsString::from).collect();
        let result = recovery_arguments(&args);
        assert_eq!(&result[..result.len() - 1], args);
        assert_eq!(result.last().map(OsString::as_os_str), Some(OsStr::new(GPU_RESET)));
    }

    #[test]
    fn recovery_marker_does_not_accumulate() {
        let args = [OsString::from("--mute"), OsString::from(GPU_RESET)];
        assert_eq!(recovery_arguments(&args), args);
    }
}
