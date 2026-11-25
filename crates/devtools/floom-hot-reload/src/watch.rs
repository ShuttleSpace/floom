//! Which source files changed since the last time anyone looked.
//!
//! Polling, not inotify: the platforms disagree about what a notification means
//! (a rename is a delete plus a create on one, a move on another), editors
//! disagree about write strategies (some truncate and rewrite, some rename a temp
//! file over the target), and every one of those cases is a bug someone has to
//! find. A `mtime` sweep of a source tree is a few thousand stat calls and is
//! right on every filesystem that has timestamps.
//!
//! The caller owns the cadence. [`Watch::poll`] answers "what changed", and the
//! answer is a *decision* the app makes with it: rebuild and remount, show a
//! banner, or nothing.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// A source tree, remembered as it was at the last poll.
#[derive(Debug)]
pub struct Watch {
    root: PathBuf,
    seen: Vec<(PathBuf, Option<SystemTime>)>,
    first: bool,
}

impl Watch {
    /// Start watching `root`, recording what is there now.
    ///
    /// The first poll reports nothing: an app that starts and immediately reloads
    /// itself has wasted a rebuild on the state it just built.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let mut watch = Self {
            root,
            seen: Vec::new(),
            first: true,
        };
        watch.seen = watch.sweep();
        watch
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Files that appeared or whose mtime moved since the previous poll.
    ///
    /// Deletions are reported too — a file that went away is a source change the
    /// app has to react to, and silence would leave the tree mounted from code
    /// that no longer exists.
    pub fn poll(&mut self) -> Vec<PathBuf> {
        let now = self.sweep();
        if self.first {
            self.first = false;
            self.seen = now;
            return Vec::new();
        }
        let previous: BTreeSet<&PathBuf> = self.seen.iter().map(|(p, _)| p).collect();
        let changed: Vec<PathBuf> = now
            .iter()
            .filter(|(path, stamp)| {
                !previous.contains(path)
                    || self
                        .seen
                        .iter()
                        .find(|(seen, _)| seen == path)
                        .is_some_and(|(_, was)| was != stamp)
            })
            .map(|(path, _)| path.clone())
            .collect();
        let current: BTreeSet<&PathBuf> = now.iter().map(|(p, _)| p).collect();
        let mut gone: Vec<PathBuf> = self
            .seen
            .iter()
            .filter(|(path, _)| !current.contains(path))
            .map(|(path, _)| path.clone())
            .collect();
        self.seen = now;
        let mut out = changed;
        out.append(&mut gone);
        out.sort();
        out.dedup();
        out
    }

    fn sweep(&self) -> Vec<(PathBuf, Option<SystemTime>)> {
        let mut files = Vec::new();
        collect(&self.root, &mut files);
        files.sort();
        files
    }
}

fn collect(dir: &Path, out: &mut Vec<(PathBuf, Option<SystemTime>)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // `target` is build output; a rebuild rewriting it would make the watcher
        // reload forever. Hidden directories are editor state, not source.
        if name == "target" || name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let stamp = entry.metadata().and_then(|m| m.modified()).ok();
            out.push((path, stamp));
        }
    }
}
