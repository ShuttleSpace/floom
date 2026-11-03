//! Key-value persistence and restoration for Floom 2.0.
//!
//! Widget-free (framework sits below widgets). Three layers:
//!
//! * [`KeyValueStore`] — the backend contract. Backends are byte-oriented and
//!   namespaced; the reference implementations are [`MemoryStore`] and
//!   [`FileStore`].
//! * [`RestorationSnapshot`] — a versioned bag of entries captured from a
//!   store and replayed into another (or the same one later), so "restore the
//!   previous session" is a copy, not a migration script.
//! * typed helpers ([`get_string`]/[`set_string`]/[`get`]) for the common
//!   string/`serde`-free cases; binary values stay the backend's business.
//!
//! There is deliberately no async and no serde dependency here: app state
//! snapshots are small, I/O happens on explicit calls, and the format is the
//! backend's choice (files on disk, rows in SQLite, …).

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// A key-value persistence backend.
///
/// Keys are namespaced with `/` (`ui/counter`, `session/drafts/3`). Backends
/// must treat keys as opaque UTF-8 paths and values as opaque bytes.
pub trait KeyValueStore {
    /// Store-level error type.
    type Error: fmt::Display;

    /// Read a value; `Ok(None)` means "absent", not "error".
    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Self::Error>;

    /// Write a value, creating or overwriting.
    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), Self::Error>;

    /// Delete a key. Deleting an absent key is not an error.
    fn delete(&mut self, key: &str) -> Result<(), Self::Error>;

    /// All keys currently present, sorted (so snapshot capture is stable).
    fn keys(&self) -> Result<Vec<String>, Self::Error>;
}

/// In-memory backend: the reference implementation and the test double.
#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    map: BTreeMap<String, Vec<u8>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of live entries.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl KeyValueStore for MemoryStore {
    type Error = std::convert::Infallible;

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
        Ok(self.map.get(key).cloned())
    }

    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), Self::Error> {
        self.map.insert(key.to_string(), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), Self::Error> {
        self.map.remove(key);
        Ok(())
    }

    fn keys(&self) -> Result<Vec<String>, Self::Error> {
        Ok(self.map.keys().cloned().collect())
    }
}

/// File-backed backend: one file per key under a directory.
///
/// Keys containing `/` map to nested paths; `..` components are rejected so a
/// hostile key cannot escape the store directory. Values are written
/// atomically (temp file + rename) so a crash mid-write cannot tear a value.
#[derive(Debug, Clone)]
pub struct FileStore {
    root: PathBuf,
}

impl FileStore {
    /// A store rooted at `root` (created on first write if missing).
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Filesystem path for a key, after validating it.
    fn path_for(&self, key: &str) -> Result<PathBuf, FileError> {
        if key.is_empty() {
            return Err(FileError("empty key"));
        }
        let mut path = self.root.clone();
        for component in key.split('/') {
            match component {
                "" | "." => return Err(FileError("empty path component")),
                ".." => return Err(FileError("'..' is not allowed in keys")),
                name => path.push(name),
            }
        }
        Ok(path)
    }
}

/// File backend errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileError(&'static str);

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "file store: {}", self.0)
    }
}

impl std::error::Error for FileError {}

impl KeyValueStore for FileStore {
    type Error = StoreError;

    fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
        let path = self.path_for(key)?;
        match std::fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn set(&mut self, key: &str, value: &[u8]) -> Result<(), Self::Error> {
        let path = self.path_for(key)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Atomic-ish write: temp file in the same directory, then rename.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, value)?;
        std::fs::rename(&tmp, &path)?;
        Ok(())
    }

    fn delete(&mut self, key: &str) -> Result<(), Self::Error> {
        let path = self.path_for(key)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    fn keys(&self) -> Result<Vec<String>, Self::Error> {
        let mut keys = Vec::new();
        collect_keys(&self.root, String::new(), &mut keys)?;
        keys.sort();
        Ok(keys)
    }
}

/// Wrapper so validation and IO errors can flow through
/// [`KeyValueStore::Error`] as one type.
#[derive(Debug)]
pub enum StoreError {
    Io(std::io::Error),
    InvalidKey(FileError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "store io: {err}"),
            Self::InvalidKey(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<FileError> for StoreError {
    fn from(err: FileError) -> Self {
        Self::InvalidKey(err)
    }
}

fn collect_keys(dir: &Path, prefix: String, out: &mut Vec<String>) -> std::io::Result<()> {
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(Result::ok).collect(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            let nested = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            collect_keys(&path, nested, out)?;
        } else if name.ends_with(".tmp") {
            // Left-over temp file from a crashed write: not a key.
            continue;
        } else if prefix.is_empty() {
            out.push(name);
        } else {
            out.push(format!("{prefix}/{name}"));
        }
    }
    Ok(())
}

/// A versioned capture of a store's contents.
///
/// Capture → store anywhere → restore: the snapshot is plain data, so it can
/// be serialized by the app (its own format), moved between backends, or kept
/// for an "undo everything" affordance. `version` lets an app reject
/// snapshots from incompatible future versions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RestorationSnapshot {
    /// App-chosen schema version.
    pub version: u32,
    entries: BTreeMap<String, Vec<u8>>,
}

impl RestorationSnapshot {
    /// An empty snapshot stamped with `version`.
    pub fn new(version: u32) -> Self {
        Self {
            version,
            entries: BTreeMap::new(),
        }
    }

    /// Capture every entry of `store` into a snapshot.
    pub fn capture<S: KeyValueStore>(store: &S, version: u32) -> Result<Self, S::Error> {
        let mut snapshot = Self::new(version);
        for key in store.keys()? {
            if let Some(value) = store.get(&key)? {
                snapshot.entries.insert(key, value);
            }
        }
        Ok(snapshot)
    }

    /// Replay the snapshot into `store`, replacing the keys it holds.
    ///
    /// Entries in the store that the snapshot does not mention are left alone;
    /// call [`clear`](Self::clear) first via [`wipe_then_restore`] semantics if
    /// a whole-session restore is wanted (see [`RestorationSnapshot::restore_all`]).
    pub fn restore<S: KeyValueStore>(&self, store: &mut S) -> Result<usize, S::Error> {
        let mut written = 0;
        for (key, value) in &self.entries {
            store.set(key, value)?;
            written += 1;
        }
        Ok(written)
    }

    /// [`restore`](Self::restore), but first deleting every key currently in
    /// the store that the snapshot does not contain — a faithful "back in
    /// time" restore.
    pub fn restore_all<S: KeyValueStore>(&self, store: &mut S) -> Result<usize, S::Error> {
        for key in store.keys()? {
            if !self.entries.contains_key(&key) {
                store.delete(&key)?;
            }
        }
        self.restore(store)
    }

    /// Entry count (tests and diagnostics).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert/overwrite one entry directly (seeding a snapshot in tests or
    /// migrations).
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Vec<u8>>) {
        self.entries.insert(key.into(), value.into());
    }

    /// Remove every entry from the snapshot (keeps the version stamp).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// String convenience: UTF-8 read (invalid UTF-8 reads as `None`).
pub fn get_string<S: KeyValueStore>(store: &S, key: &str) -> Result<Option<String>, S::Error> {
    Ok(store
        .get(key)?
        .and_then(|bytes| String::from_utf8(bytes).ok()))
}

/// String convenience write.
pub fn set_string<S: KeyValueStore>(store: &mut S, key: &str, value: &str) -> Result<(), S::Error> {
    store.set(key, value.as_bytes())
}

/// Typed convenience over a string encoding defined by `FromStr`/`Display`.
///
/// Good for counters, indexes and other simple scalars; structured data should
/// go through the app's own serialization into bytes.
pub fn get<T: std::str::FromStr, S: KeyValueStore>(
    store: &S,
    key: &str,
) -> Result<Option<T>, S::Error> {
    match get_string(store, key)? {
        Some(text) => Ok(text.parse().ok()),
        None => Ok(None),
    }
}

pub fn set<T: fmt::Display, S: KeyValueStore>(
    store: &mut S,
    key: &str,
    value: &T,
) -> Result<(), S::Error> {
    set_string(store, key, &value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips_and_lists_sorted() {
        let mut store = MemoryStore::new();
        set_string(&mut store, "ui/theme", "dark").unwrap();
        set_string(&mut store, "session/draft", "hello").unwrap();
        set_string(&mut store, "ui/theme", "light").unwrap();

        assert_eq!(
            get_string(&store, "ui/theme").unwrap(),
            Some("light".into())
        );
        assert_eq!(store.len(), 2);
        assert_eq!(
            store.keys().unwrap(),
            vec!["session/draft".to_string(), "ui/theme".to_string()]
        );

        store.delete("ui/theme").unwrap();
        assert_eq!(get_string(&store, "ui/theme").unwrap(), None);
        store.delete("ui/theme").unwrap(); // absent delete is fine
    }

    #[test]
    fn typed_helpers_parse_and_format() {
        let mut store = MemoryStore::new();
        set(&mut store, "ui/counter", &41).unwrap();
        let value: Option<i32> = get(&store, "ui/counter").unwrap();
        assert_eq!(value, Some(41));

        // Unparseable content reads as None, not an error.
        set_string(&mut store, "ui/other", "not-a-number").unwrap();
        let value: Option<i32> = get(&store, "ui/other").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn file_store_persists_and_survives_reopening() {
        let root = std::env::temp_dir().join(format!("floom-persist-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        {
            let mut store = FileStore::new(&root);
            set_string(&mut store, "ui/theme", "dark").unwrap();
            set_string(&mut store, "session/drafts/1", "hello").unwrap();
        }
        // A fresh instance over the same directory sees the values.
        let mut store = FileStore::new(&root);
        assert_eq!(get_string(&store, "ui/theme").unwrap(), Some("dark".into()));
        assert_eq!(
            get_string(&store, "session/drafts/1").unwrap(),
            Some("hello".into())
        );
        assert_eq!(
            store.keys().unwrap(),
            vec!["session/drafts/1".to_string(), "ui/theme".to_string()]
        );

        store.delete("ui/theme").unwrap();
        assert_eq!(get_string(&store, "ui/theme").unwrap(), None);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn file_store_path_validation() {
        let store = FileStore::new("/tmp/floom-should-not-exist");
        assert!(store.get("../escape").is_err(), "'..' must be rejected");
        assert!(store.get("a//b").is_err(), "empty components rejected");
        assert!(store.get("").is_err(), "empty key rejected");
    }

    #[test]
    fn snapshot_capture_restore_is_faithful() {
        let mut source = MemoryStore::new();
        set_string(&mut source, "a", "1").unwrap();
        set_string(&mut source, "b", "2").unwrap();

        let snapshot = RestorationSnapshot::capture(&source, 3).unwrap();
        assert_eq!(snapshot.version, 3);
        assert_eq!(snapshot.len(), 2);

        // Restore into a *different* backend.
        let mut target = FileStore::new(
            std::env::temp_dir().join(format!("floom-restore-test-{}", std::process::id())),
        );
        let written = snapshot.restore(&mut target).unwrap();
        assert_eq!(written, 2);
        assert_eq!(get_string(&target, "a").unwrap(), Some("1".into()));

        let _ = std::fs::remove_dir_all(
            std::env::temp_dir().join(format!("floom-restore-test-{}", std::process::id())),
        );
    }

    #[test]
    fn restore_all_removes_keys_the_snapshot_does_not_hold() {
        let mut store = MemoryStore::new();
        set_string(&mut store, "a", "old").unwrap();
        set_string(&mut store, "extra", "stale").unwrap();

        let mut snapshot = RestorationSnapshot::new(1);
        snapshot.insert("a", "new");

        let written = snapshot.restore_all(&mut store).unwrap();
        assert_eq!(written, 1);
        assert_eq!(get_string(&store, "a").unwrap(), Some("new".into()));
        assert_eq!(get_string(&store, "extra").unwrap(), None);
    }
}
