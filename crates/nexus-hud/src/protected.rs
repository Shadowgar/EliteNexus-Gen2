//! Compare-and-replace edits for game files. Snapshot targets before asking to write them.

use std::fs::{self, Metadata, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) fn linked(metadata: &Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    metadata.file_type().is_symlink()
}

pub(crate) fn directory(path: &Path) -> io::Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    if linked(&metadata) || !metadata.is_dir() {
        return Err(io::Error::other("expected a regular directory"));
    }
    path.canonicalize()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Snapshot {
    path: PathBuf,
    parent: PathBuf,
    pub(crate) bytes: Option<Vec<u8>>,
}

impl Snapshot {
    pub(crate) fn read(path: &Path) -> io::Result<Self> {
        let parent = directory(path.parent().ok_or_else(|| io::Error::other("missing parent directory"))?)?;
        let bytes = match fs::symlink_metadata(path) {
            Ok(m) if m.is_file() && !linked(&m) => Some(fs::read(path)?),
            Ok(_) => return Err(io::Error::other("expected a regular file")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e),
        };
        Ok(Self { path: path.into(), parent, bytes })
    }

    fn verify(&self) -> io::Result<()> {
        if *self == Self::read(&self.path)? {
            Ok(())
        } else {
            Err(io::Error::other(format!("{} changed on disk; reload it before editing", self.path.display())))
        }
    }

    pub(crate) fn edit(self, next: Option<Vec<u8>>) -> io::Result<Edit> {
        self.verify()?;
        let staged = next.as_deref().map(|bytes| Staged::new(&self.parent, bytes)).transpose()?;
        Ok(Edit { previous: self, next, staged })
    }
}

struct Staged(PathBuf);

impl Staged {
    fn new(parent: &Path, bytes: &[u8]) -> io::Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        loop {
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".gen2-stage-{}-{n}", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let stage = Self(path);
                    let result = file.write_all(bytes).and_then(|()| file.sync_all());
                    drop(file);
                    result?;
                    return Ok(stage);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
    }

    fn place(&self, target: &Path, existing: bool) -> io::Result<()> {
        if existing {
            fs::rename(&self.0, target)
        } else {
            // Linking publishes an already complete file without replacing an unexpected arrival.
            fs::hard_link(&self.0, target)
        }
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(crate) struct Edit {
    previous: Snapshot,
    next: Option<Vec<u8>>,
    staged: Option<Staged>,
}

impl Edit {
    fn apply(&self) -> io::Result<()> {
        self.previous.verify()?;
        let path = &self.previous.path;
        match &self.staged {
            Some(file) => file.place(path, self.previous.bytes.is_some()),
            None if self.previous.bytes.is_some() => fs::remove_file(path),
            None => Ok(()),
        }
    }

    fn undo(&self) -> io::Result<()> {
        let current = Snapshot { bytes: self.next.clone(), ..self.previous.clone() };
        current.edit(self.previous.bytes.clone())?.apply()
    }
}

pub(crate) fn commit(edits: &[Edit]) -> io::Result<()> {
    for edit in edits {
        edit.previous.verify()?;
    }
    for (i, edit) in edits.iter().enumerate() {
        if let Err(error) = edit.apply() {
            let failures: Vec<_> = edits[..i].iter().rev().filter_map(|e| e.undo().err()).collect();
            if !failures.is_empty() {
                return Err(io::Error::other(format!("{error}; rollback incomplete: {failures:?}")));
            }
            return Err(error);
        }
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn a_later_conflict_rolls_back_the_first_edit_without_replacing_a_foreign_write() {
    let root = std::env::temp_dir().join(format!("nexus-hud-rollback-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let path = root.join("file");
    fs::write(&path, b"original").unwrap();
    let before = Snapshot::read(&path).unwrap();
    let change = before.clone().edit(Some(b"first".to_vec())).unwrap();
    let conflict = before.clone().edit(Some(b"second".to_vec())).unwrap();
    assert!(commit(&[change, conflict]).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"original");
    let change = before.edit(Some(b"ours".to_vec())).unwrap();
    change.apply().unwrap();
    fs::write(&path, b"foreign").unwrap();
    assert!(change.undo().is_err());
    assert_eq!(fs::read(&path).unwrap(), b"foreign");
    drop(change);
    fs::remove_file(path).unwrap();
    fs::remove_dir(root).unwrap();
}
