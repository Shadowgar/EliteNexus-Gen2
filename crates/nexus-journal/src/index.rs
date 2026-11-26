//! Compact index of every journal event, including the owner and observation scope.
//! the JSON itself stays on disk and is read on demand.

use crate::time::Timestamp;
use rustc_hash::FxHashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub struct IndexEntry {
    pub ts: i64,
    pub file: u32,
    pub offset: u64,
    pub len: u32,
    pub name: u16,
    owner: u32,
    scope: u64,
    fingerprint: Option<u64>,
}

#[derive(Debug, Default)]
pub struct JournalIndex {
    pub files: Vec<PathBuf>,
    pub source_generation: u64,
    pub entries: Vec<IndexEntry>,
    identities: Vec<Option<crate::watcher::FileIdentity>>,
    owners: Vec<String>,
    owner_ids: FxHashMap<String, u32>,
    names: Vec<String>,
    name_ids: FxHashMap<String, u16>,
    counts: Vec<u32>,
}

impl JournalIndex {
    pub fn add_file(&mut self, path: PathBuf) -> u32 {
        if let Some(i) = self.files.iter().position(|p| *p == path) {
            return i as u32;
        }
        self.files.push(path);
        self.identities.push(None);
        (self.files.len() - 1) as u32
    }

    pub fn bind_file(&mut self, path: PathBuf, identity: crate::watcher::FileIdentity) -> u32 {
        let file = self.add_file(path);
        self.identities[file as usize] = Some(identity);
        file
    }

    pub fn owner(&self, entry: &IndexEntry) -> Option<&str> {
        let id = entry.owner.checked_sub(1)?;
        self.owners.get(id as usize).map(String::as_str)
    }

    pub fn in_scope(&self, entry: &IndexEntry, scope: u64) -> bool {
        entry.scope == scope
    }

    pub fn belongs_to(&self, entry: &IndexEntry, fid: &str) -> bool {
        self.owner(entry) == Some(fid)
    }

    fn intern(&mut self, name: &str) -> u16 {
        if let Some(&id) = self.name_ids.get(name) {
            return id;
        }
        let id = self.names.len().min(u16::MAX as usize) as u16;
        self.names.push(name.to_string());
        self.counts.push(0);
        self.name_ids.insert(name.to_string(), id);
        id
    }

    pub fn push(&mut self, ts: Timestamp, file: u32, offset: u64, len: usize, name: &str) {
        self.record(IndexEntry { ts: ts.0, file, offset, len: len as u32, name: 0, owner: 0, scope: 0, fingerprint: None }, name);
    }

    pub fn push_observation(&mut self, observation: IndexedObservation<'_>) {
        let owner = observation.fid.map_or(0, |fid| {
            if let Some(id) = self.owner_ids.get(fid) {
                return *id;
            }
            self.owners.push(fid.to_owned());
            let id = self.owners.len() as u32;
            self.owner_ids.insert(fid.to_owned(), id);
            id
        });
        let len = observation.line.len();
        if let Ok(len) = u32::try_from(len) {
            self.record(IndexEntry {
                ts: observation.timestamp.0, file: observation.source.file, offset: observation.source.offset,
                len, name: 0, owner, scope: observation.scope, fingerprint: Some(fingerprint(observation.line)),
            }, observation.name);
        }
    }

    fn record(&mut self, mut entry: IndexEntry, name: &str) {
        entry.name = self.intern(name);
        if let Some(count) = self.counts.get_mut(entry.name as usize) {
            *count += 1;
        }
        self.entries.push(entry);
    }

    pub fn name(&self, id: u16) -> &str {
        self.names.get(id as usize).map(String::as_str).unwrap_or("?")
    }

    /// Event names with counts, most frequent first.
    pub fn name_counts(&self) -> Vec<(u16, &str, u32)> {
        let mut v: Vec<(u16, &str, u32)> = self.names.iter().enumerate().map(|(i, n)| (i as u16, n.as_str(), self.counts[i])).collect();
        v.sort_by(|a, b| b.2.cmp(&a.2).then(a.1.cmp(b.1)));
        v
    }

    pub fn id_of(&self, name: &str) -> Option<u16> {
        self.name_ids.get(name).copied()
    }

    /// Reads the raw JSON line of an entry from disk.
    pub fn read_line(&self, e: &IndexEntry) -> Option<String> {
        let path = self.files.get(e.file as usize)?;
        let mut f = std::fs::File::open(path).ok()?;
        if let Some(identity) = self.identities.get(e.file as usize).copied().flatten()
            && crate::watcher::file_identity(&f).ok()? != identity
        {
            return None;
        }
        f.seek(SeekFrom::Start(e.offset)).ok()?;
        let mut buf = vec![0u8; e.len as usize];
        f.read_exact(&mut buf).ok()?;
        let text = String::from_utf8(buf).ok()?;
        if e.fingerprint.is_some_and(|expected| fingerprint(&text) != expected) {
            return None;
        }
        Some(text)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct IndexedObservation<'a> {
    pub timestamp: Timestamp,
    pub source: crate::raw::LineSource,
    pub scope: u64,
    pub fid: Option<&'a str>,
    pub name: &'a str,
    pub line: &'a str,
}

fn fingerprint(line: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = rustc_hash::FxHasher::default();
    line.hash(&mut h);
    h.finish()
}
