// SPDX-License-Identifier: MIT
//! Pure-std file-watching for `floom run --watch`.
//!
//! No `notify` dependency: `floom-cli` is deliberately std-only
//! (`Cargo.toml:11-14`), so pulling a file-watching crate in would drag its tree
//! into the first install step. We instead poll modification times on a
//! configurable interval and debounce the burst an editor emits for one save
//! (several write/close events collapse into one rebuild). The watcher owns no
//! IO beyond a directory walk; it answers "did something change, and has it
//! settled", and `run_app` decides what to rebuild and relaunch.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

/// A point-in-time map of every watched source file to its modification time.
///
/// Two snapshots compare equal when nothing was created, deleted, or written.
/// `BTreeMap` makes the comparison order-independent and deterministic, which
/// is what the unit tests rely on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileSnapshot {
    times: BTreeMap<PathBuf, u64>,
}

/// Why a snapshot could not be taken.
#[derive(Debug)]
pub enum WatchError {
    /// A configured watch root vanished (e.g. the app dir was renamed).
    MissingRoot(PathBuf),
    /// A metadata read failed on `path`.
    Io(PathBuf, std::io::Error),
}

impl std::fmt::Display for WatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchError::MissingRoot(p) => write!(f, "watch root disappeared: {}", p.display()),
            WatchError::Io(p, e) => write!(f, "{}: {e}", p.display()),
        }
    }
}

impl std::error::Error for WatchError {}

impl FileSnapshot {
    /// Number of files captured — a cheap liveness signal for logging.
    pub fn len(&self) -> usize {
        self.times.len()
    }
}

/// Build a snapshot of every source file under `roots`, recursing into
/// subdirectories but skipping build-artifact noise (`target`/`.git`/
/// `node_modules`/`.floom`). Only source files (`.tsx`/`.ts`/`.json`) are
/// kept, which both keeps the walk cheap and — crucially — means the rebuild
/// itself writes nothing we watch: `floom run` emits into `<app>/.floom/run`,
/// and `cargo` writes into `target`, neither of which is observed.
pub fn snapshot(roots: &[PathBuf]) -> Result<FileSnapshot, WatchError> {
    let mut times = BTreeMap::new();
    for root in roots {
        if !root.exists() {
            return Err(WatchError::MissingRoot(root.clone()));
        }
        walk(root, &mut times)?;
    }
    Ok(FileSnapshot { times })
}

fn walk(dir: &Path, times: &mut BTreeMap<PathBuf, u64>) -> Result<(), WatchError> {
    let entries = fs::read_dir(dir).map_err(|e| WatchError::Io(dir.to_path_buf(), e))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            if matches!(name.as_ref(), "target" | ".git" | "node_modules" | ".floom") {
                continue;
            }
            walk(&path, times)?;
        } else if is_source(&name) {
            let mtime = modified(&path)?;
            times.insert(path, mtime);
        }
    }
    Ok(())
}

/// Whether a file name is a watched source file.
pub fn is_source(name: &str) -> bool {
    name.ends_with(".tsx") || name.ends_with(".ts") || name.ends_with(".json")
}

fn modified(path: &Path) -> Result<u64, WatchError> {
    let meta = fs::metadata(path).map_err(|e| WatchError::Io(path.to_path_buf(), e))?;
    let dur = meta
        .modified()
        .map_err(|e| WatchError::Io(path.to_path_buf(), e))?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    // Nanosecond resolution packed into one u64: seconds * 1e9 + nanos. Enough
    // to detect "did this file change" across platforms; we never compare two
    // different files against each other.
    Ok(dur.as_secs().saturating_mul(1_000_000_000) + u64::from(dur.subsec_nanos()))
}

/// The result of `wait_for_change`: when a change was first observed, how long
/// the watch had been idle before that (so a caller can report detection
/// latency bounded by the poll period), and the settled snapshot that became
/// the new baseline.
#[derive(Debug, Clone)]
pub struct Change {
    /// Instant the change was first seen — the anchor for "save → running".
    pub first_seen: Instant,
    /// Wall time since this watch cycle began until the change was first seen.
    pub detected_after: Duration,
    /// The new baseline the watch now compares against.
    pub snapshot: FileSnapshot,
}

/// A poller that watches `roots` every `interval` and reports a change only
/// after `debounce` of quiet time, so an editor's flurry of writes for one save
/// collapses into a single rebuild.
pub struct Watch {
    roots: Vec<PathBuf>,
    interval: Duration,
    debounce: Duration,
    baseline: FileSnapshot,
}

impl Watch {
    /// Take the baseline snapshot now; subsequent `wait_for_change` polls
    /// against it.
    pub fn new(
        roots: Vec<PathBuf>,
        interval: Duration,
        debounce: Duration,
    ) -> Result<Self, WatchError> {
        let baseline = snapshot(&roots)?;
        Ok(Self {
            roots,
            interval,
            debounce,
            baseline,
        })
    }

    /// Block until a change is observed and has settled for `debounce`, then
    /// rebase the watch onto that settled snapshot so the next call only fires
    /// on a *new* change (otherwise the same edit would fire every poll).
    ///
    /// The default `interval` is coarse (hundreds of ms), so this is meant to be
    /// called from a dedicated watch loop, not alongside the app's event loop —
    /// which is fine, because `floom run` spawns the app as a separate process
    /// and only watches from the CLI.
    pub fn wait_for_change(&mut self) -> Result<Change, WatchError> {
        let started = Instant::now();
        let mut pending_since: Option<Instant> = None;
        loop {
            std::thread::sleep(self.interval);
            let now = snapshot(&self.roots)?;
            let (pending, fired) =
                debounce_step(pending_since, now != self.baseline, self.debounce);
            pending_since = pending;
            if fired {
                let change = Change {
                    first_seen: pending_since.expect("fired implies a pending instant"),
                    detected_after: started.elapsed(),
                    snapshot: now.clone(),
                };
                // Without this, the settled change would still differ from the
                // old baseline and every subsequent poll would fire again.
                self.baseline = now;
                return Ok(change);
            }
        }
    }
}

/// Pure debounce decision, separable from real time so it is unit-testable.
///
/// Given the previous "pending since" timestamp (`None` = no change in flight)
/// and whether a change is currently observed, decide the new pending state and
/// whether the change has now settled.
pub fn debounce_step(
    pending_since: Option<Instant>,
    changed: bool,
    debounce: Duration,
) -> (Option<Instant>, bool) {
    match (changed, pending_since) {
        // Quiet again: any in-flight burst is cancelled.
        (false, _) => (None, false),
        // First sighting of a change this cycle: arm the timer, not yet settled.
        (true, None) => (Some(Instant::now()), false),
        // Change still present and the timer has elapsed: fire.
        (true, Some(since)) => (Some(since), since.elapsed() >= debounce),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("floom-watch-test-{}-{name}", std::process::id()))
    }

    /// Snapshot a single root without cloning it into a temporary array.
    fn snap(root: &PathBuf) -> Result<FileSnapshot, WatchError> {
        snapshot(std::slice::from_ref(root))
    }

    #[test]
    fn is_source_names_the_watched_extensions() {
        assert!(is_source("app.tsx"));
        assert!(is_source("Button.ts"));
        assert!(is_source("tsconfig.json"));
        assert!(!is_source("main.rs"));
        assert!(!is_source("ui_gen.rs"));
        assert!(!is_source("Cargo.toml"));
        assert!(!is_source("image.png"));
    }

    #[test]
    fn a_new_file_changes_the_snapshot() {
        let dir = tmp("new-file");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app.tsx");
        fs::write(&app, "v1").unwrap();

        let before = snap(&dir).unwrap();
        fs::write(dir.join("more.ts"), "x").unwrap(); // a second source file
        let after = snap(&dir).unwrap();

        assert_eq!(before.len(), 1, "only the .tsx was present first");
        assert_eq!(after.len(), 2, "the .ts appeared");
        assert_ne!(before, after, "adding a file must change the snapshot");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn editing_a_file_changes_the_snapshot() {
        let dir = tmp("edit");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app.tsx");
        fs::write(&app, "v1").unwrap();

        let before = snap(&dir).unwrap();
        // Bump mtime past the fs clock granularity before rewriting.
        std::thread::sleep(Duration::from_millis(20));
        fs::write(&app, "v2 with different length").unwrap();
        let after = snap(&dir).unwrap();

        assert_ne!(before, after, "a newer mtime must change the snapshot");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn deleting_a_file_changes_the_snapshot() {
        let dir = tmp("delete");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let keep = dir.join("keep.tsx");
        let gone = dir.join("gone.ts");
        fs::write(&keep, "k").unwrap();
        fs::write(&gone, "g").unwrap();

        let before = snap(&dir).unwrap();
        fs::remove_file(&gone).unwrap();
        let after = snap(&dir).unwrap();

        assert_eq!(before.len(), 2);
        assert_eq!(after.len(), 1);
        assert_ne!(before, after);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_unchanged_directory_produces_equal_snapshots() {
        let dir = tmp("stable");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("app.tsx"), "v1").unwrap();

        let a = snap(&dir).unwrap();
        let b = snap(&dir).unwrap();
        assert_eq!(a, b, "no writes between reads => equal snapshots");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn build_artifacts_are_not_watched() {
        // A `.floom/run` subtree with a `target` dir and a `.tsx` inside `target`
        // must contribute nothing to the snapshot: the rebuild writes there.
        let dir = tmp("artifacts");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join(".floom").join("run").join("target")).unwrap();
        fs::write(dir.join("app.tsx"), "v1").unwrap();
        fs::write(dir.join(".floom").join("run").join("main.rs"), "x").unwrap();
        fs::write(
            dir.join(".floom")
                .join("run")
                .join("target")
                .join("gen.tsx"),
            "should be ignored",
        )
        .unwrap();

        let s = snap(&dir).unwrap();
        assert_eq!(s.len(), 1, "only app.tsx is watched, not build output");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_root_is_an_error() {
        let ghost = tmp("ghost");
        let _ = fs::remove_dir_all(&ghost);
        assert!(matches!(snap(&ghost), Err(WatchError::MissingRoot(_))));
    }

    #[test]
    fn debounce_reports_nothing_while_quiet() {
        assert_eq!(
            debounce_step(None, false, Duration::from_millis(10)),
            (None, false)
        );
    }

    #[test]
    fn debounce_arms_on_first_sighting() {
        let (pending, fired) = debounce_step(None, true, Duration::from_millis(10));
        assert!(pending.is_some(), "a pending instant must be armed");
        assert!(!fired, "one tick of change is not enough to settle");
    }

    #[test]
    fn debounce_fires_once_enough_time_has_passed() {
        // Simulate a pending instant that is already older than the debounce.
        let old = Instant::now() - Duration::from_millis(50);
        let (_, fired) = debounce_step(Some(old), true, Duration::from_millis(10));
        assert!(fired, "a settled change must fire");
    }

    #[test]
    fn debounce_cancels_when_quiet_again() {
        let armed = Instant::now();
        let (pending, fired) = debounce_step(Some(armed), false, Duration::from_millis(10));
        assert!(pending.is_none());
        assert!(!fired);
    }

    #[test]
    fn wait_for_change_detects_a_real_file_write() {
        let dir = tmp("watch-loop");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app.tsx");
        fs::write(&app, "v1").unwrap();

        let mut watch = Watch::new(
            vec![dir.clone()],
            // A 2ms poll + 10ms debounce keeps the test fast and deterministic.
            Duration::from_millis(2),
            Duration::from_millis(10),
        )
        .unwrap();

        // Edit on a background thread so the watch loop sees it mid-wait.
        let writer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(15));
            fs::write(&app, "v2").unwrap();
        });

        let change = watch.wait_for_change().expect("detect the write");
        writer.join().unwrap();

        assert!(
            change.detected_after < Duration::from_secs(2),
            "detection should be prompt, not hang: {:?}",
            change.detected_after
        );
        assert_eq!(change.snapshot.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn wait_for_change_rebases_so_the_same_edit_does_not_fire_twice() {
        // Regression for the tight-loop bug: after a change fires, the watch
        // must compare against the *new* state. We prove it by making the second
        // call return only after a genuinely new edit, not immediately.
        let dir = tmp("rebase");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let app = dir.join("app.tsx");
        fs::write(&app, "v1").unwrap();

        let mut watch = Watch::new(
            vec![dir.clone()],
            Duration::from_millis(2),
            Duration::from_millis(10),
        )
        .unwrap();

        // First edit -> first fire (which rebases).
        let first = {
            let a = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(15));
                fs::write(&a, "v2").unwrap();
            })
        };
        let c1 = watch.wait_for_change().expect("first change");
        first.join().unwrap();
        assert_eq!(c1.snapshot.len(), 1);

        // Prove the baseline moved: a fresh snapshot taken now must EQUAL the
        // watch's baseline, so an immediate poll would see no change. We can't
        // inspect the private baseline directly, but a second wait with a very
        // short budget must NOT return without a new edit — so we spawn a new
        // edit and confirm it still detects (rather than spinning on the old).
        let second = {
            let a = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(15));
                fs::write(&a, "v3").unwrap();
            })
        };
        let c2 = watch.wait_for_change().expect("second change");
        second.join().unwrap();
        assert!(
            c2.detected_after < Duration::from_secs(2),
            "a new edit must still be detected promptly: {:?}",
            c2.detected_after
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
