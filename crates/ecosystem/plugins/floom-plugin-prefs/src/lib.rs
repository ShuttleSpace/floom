//! The OS's own key-value store, for user preferences.
//!
//! This is `shared_preferences`'s question, and the catalog's A5
//! (`docs/plugins.md`): one string-to-string map per store name,
//! whose *where* is the platform's choice — `SharedPreferences` on Android,
//! `NSUserDefaults` on Apple, a file in the user's config directory on Linux
//! and Windows (the same shape Flutter's own `shared_preferences_linux` and
//! `_windows` implementations chose, for the same reason: those platforms
//! have no OS preferences store to ask).
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # The split from `floom-persistence` — read this before using either
//!
//! These are different questions, and the answer decides which crate you
//! want:
//!
//! | | `floom-persistence` | this crate |
//! |---|---|---|
//! | Where | A backend *you* hand it (`FileStore` at a path you chose, `MemoryStore`) | The OS's own store — nowhere to configure |
//! | What for | Session state, restorable UI, snapshots | User settings that outlive every session |
//! | Restores | `RestorationSnapshot`, versioned capture/replay | Nothing — a preference has no lifecycle |
//!
//! `floom-persistence` does not know where your app lives; pair it with
//! `floom-plugin-paths` when you need that. This crate *is* the "where":
//! asking it to also take a path would be asking the OS's store to not be
//! the OS's store.
//!
//! # What `set` promises
//!
//! `Ok(())` means the value is stored — visible to the next `get` and to the
//! next launch. On Android that is `commit()` (the synchronous write), not
//! `apply()` (fire-and-forget): `apply()` returning immediately would make
//! `Ok` a claim about a write that may not have happened. On Apple the
//! suite's in-memory cache is updated synchronously and the plist flush is
//! the system's; on the file arm the write is temp-file-then-rename, so a
//! reader never sees a half-written store.
//!
//! # Store names, keys, values
//!
//! A store name is an identifier, not a path: non-empty, no separators, no
//! NUL. Keys and values are arbitrary UTF-8 except NUL (which no arm can
//! carry — it breaks every one of the three transports). The file arm
//! percent-encodes beyond that, transparently.

/// A named string-to-string store backed by the OS's own preferences storage.
///
/// Cheap to hold: the arms re-reach the underlying store per call (the Java
/// side caches `SharedPreferences` by name; `NSUserDefaults` is a singleton
/// graph), so there is nothing expensive to keep alive and no handle to leak.
#[derive(Debug, Clone)]
pub struct Preferences {
    store: String,
}

impl Preferences {
    /// Open (creating if absent) the named store.
    ///
    /// The name is an identifier: non-empty, no `/` or `\`, no NUL, not `.`
    /// or `..` — a name that would be a bad filename is a bad store name on
    /// every platform, even the ones that never use it as a filename.
    pub fn open(store: &str) -> Result<Preferences, String> {
        validate_name(store)?;
        Ok(Preferences {
            store: store.to_string(),
        })
    }

    /// The value for `key`, or `None` when the key is absent.
    ///
    /// A missing key is not an error: asking for an unset preference is a
    /// normal question with a normal answer.
    pub fn get(&self, key: &str) -> Option<String> {
        let key = key_str(key).ok()?;
        #[cfg(target_os = "android")]
        {
            android::get(&self.store, &key)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::get(&self.store, &key)
        }
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            not(target_env = "ohos")
        ))]
        {
            file::get(&file_store_path(&self.store), &key)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::get(&self.store, &key)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )))]
        {
            None
        }
    }

    /// Store `value` under `key`.
    ///
    /// `Ok(())` means stored — see the crate doc for what each arm does to
    /// make that literally true rather than approximately.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let key = key_str(key)?;
        let value = key_str(value)?;
        #[cfg(target_os = "android")]
        {
            android::set(&self.store, &key, &value)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::set(&self.store, &key, &value)
        }
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            not(target_env = "ohos")
        ))]
        {
            file::set(&file_store_path(&self.store), &key, &value)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::set(&self.store, &key, &value)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )))]
        {
            let _ = (key, value);
            Err(format!(
                "prefs: this target ({}) has no preferences storage",
                std::env::consts::OS
            ))
        }
    }

    /// Remove `key` from the store.
    ///
    /// Removing an absent key is `Ok(())` — after the call the key is absent,
    /// which is the entire contract.
    pub fn remove(&self, key: &str) -> Result<(), String> {
        let key = key_str(key)?;
        #[cfg(target_os = "android")]
        {
            android::remove(&self.store, &key)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::remove(&self.store, &key)
        }
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            not(target_env = "ohos")
        ))]
        {
            file::remove(&file_store_path(&self.store), &key)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::remove(&self.store, &key)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )))]
        {
            let _ = key;
            Err(format!(
                "prefs: this target ({}) has no preferences storage",
                std::env::consts::OS
            ))
        }
    }
}

/// A key or value as a validated string: NUL is the one character no
/// transport in any arm can carry.
fn key_str(s: &str) -> Result<String, String> {
    if s.contains('\0') {
        return Err(format!(
            "prefs: keys and values may not contain NUL ({s:?})"
        ));
    }
    Ok(s.to_string())
}

fn validate_name(store: &str) -> Result<(), String> {
    if store.is_empty() {
        return Err("prefs: the store name is empty".into());
    }
    if store.contains('/') || store.contains('\\') || store.contains('\0') {
        return Err(format!(
            "prefs: the store name {store:?} is an identifier, not a path — \
             no separators"
        ));
    }
    if store == "." || store == ".." {
        return Err(format!("prefs: {store:?} is not a usable store name"));
    }
    Ok(())
}

/// The file arm's path for a store: the user's config directory, plus the
/// store name with a suffix that names what the file is.
#[cfg(all(
    any(target_os = "linux", target_os = "windows"),
    not(target_env = "ohos")
))]
fn file_store_path(store: &str) -> std::path::PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
    path.push(format!("{store}.prefs"));
    path
}

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;
#[cfg(all(
    any(target_os = "linux", target_os = "windows"),
    not(target_env = "ohos")
))]
mod file;
#[cfg(target_env = "ohos")]
mod ohos;

#[cfg(test)]
mod tests {
    use super::*;

    /// A store name no other run — or user — can collide with.
    ///
    /// `Preferences::open` reaches the real OS store on every arm, so a test
    /// that used a fixed name would write into the user's own preferences and
    /// leave the domain behind. Process id plus a nanosecond stamp makes the
    /// name unique across *runs*, but not within one: this machine's clock has
    /// 1 µs granularity, so two tests starting together got the same stamp
    /// about 23% of the time (measured), and the cleanup of one erased the
    /// other's live domain. The atomic counter is what makes it unique among
    /// the tests in a process.
    pub(super) fn unique_store_name() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let seq = NEXT.fetch_add(1, Ordering::Relaxed);
        format!(
            "floom-prefs-test-{}.local-{:x}-{seq}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    /// Remove every trace of a test store, best-effort.
    ///
    /// Both arms that have a real store to clean up are covered: the file arm
    /// removes its file, the Apple arm its plist and its persistent domain.
    /// Android's `SharedPreferences` file is the app's own and has no
    /// process-external handle to delete, which is why this is best-effort
    /// rather than a guarantee.
    fn cleanup_store(name: &str) {
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::remove_domain(name);
        }
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            not(target_env = "ohos")
        ))]
        {
            let _ = std::fs::remove_file(file_store_path(name));
        }
        #[cfg(target_os = "android")]
        {
            let _ = name;
        }
    }

    #[test]
    fn names_are_identifiers_not_paths() {
        assert!(Preferences::open("gallery").is_ok());
        assert!(Preferences::open("com.example.app").is_ok());
        for bad in ["", "..", ".", "a/b", "a\\b", "a\0b"] {
            assert!(
                Preferences::open(bad).is_err(),
                "{bad:?} should not be a usable store name"
            );
        }
    }

    #[test]
    fn nul_is_the_one_universal_rejection() {
        // A store name unique to this run, cleaned up after: `Preferences::open`
        // reaches the *real* OS store, so a fixed name like "test" would write
        // into the user's own preferences and leave a domain behind for every
        // later test to trip over. The name is a valid identifier either way —
        // which is the point, since this test is about what the *values* may
        // contain.
        let name = unique_store_name();
        let prefs = Preferences::open(&name).unwrap();
        assert!(prefs.set("k\0", "v").is_err());
        assert!(prefs.set("k", "v\0").is_err());
        // Every other character is a value like any other, on every arm.
        assert!(prefs.set("k", "line\nbreak = equals\ttab").is_ok());
        prefs.remove("k").unwrap();
        cleanup_store(&name);
    }

    /// The file arm, end to end, in a directory of its own.
    ///
    /// `Preferences::open` resolves to the user's real config directory, which
    /// a test must not write into — so the file arm is exercised through the
    /// same internal entry point `open` uses, pointed at a temp directory.
    /// The encoding, the roundtrip, and the absent-key contract all get real
    /// coverage this way, on any host with the arm.
    #[cfg(all(
        any(target_os = "linux", target_os = "windows"),
        not(target_env = "ohos")
    ))]
    #[test]
    fn file_arm_roundtrip_including_the_characters_the_format_cares_about() {
        // The shared helper: a hand-rolled `pid + nanos` name is not unique
        // among tests in one process (see `unique_store_name`).
        let dir = std::env::temp_dir().join(unique_store_name());
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.prefs");

        assert_eq!(file::get(&path, "absent"), None);

        file::set(&path, "key with spaces", "value with = and\ttab\nnewline").unwrap();
        assert_eq!(
            file::get(&path, "key with spaces").as_deref(),
            Some("value with = and\ttab\nnewline")
        );

        // A second key does not disturb the first; removal does not disturb
        // the second.
        file::set(&path, "second", "2").unwrap();
        file::remove(&path, "key with spaces").unwrap();
        assert_eq!(file::get(&path, "key with spaces"), None);
        assert_eq!(file::get(&path, "second").as_deref(), Some("2"));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
