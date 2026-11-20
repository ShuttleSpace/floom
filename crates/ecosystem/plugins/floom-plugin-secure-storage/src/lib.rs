//! Secrets that outlive the process and must not sit in a readable file:
//! tokens, passwords, private keys.
//!
//! This is `flutter_secure_storage`'s question. It is **not** `prefs` with a
//! different name — `prefs` stores plaintext in the OS's preferences store,
//! which is the thing this crate exists to avoid. The split Flutter draws
//! between `shared_preferences` and `flutter_secure_storage` is the same one.
//!
//! | | `floom-plugin-prefs` | this crate |
//! |---|---|---|
//! | Readable by | anything that can read the user's files | only code running as this user (and, on Apple, only this app) |
//! | On disk | the OS's plaintext store | keychain item / Keystore-sealed / DPAPI-sealed / libsecret |
//! | For | settings | secrets |
//!
//! # What `set` promises
//!
//! `Ok(())` means the platform store accepted the value: the next `get`, in
//! this process or the next launch, returns it. On Android the ciphertext is
//! written with `commit()` for the same reason `prefs` does.
//!
//! # What `get` promises
//!
//! `Ok(None)` means *absent*; `Err` means the store could not be asked at all
//! (a locked keychain, no `secret-tool`, a Keystore key invalidated by a
//! restore). Folding those two into one `None` would report a broken store as
//! "no secret here", which is the one wrong answer worth distinguishing.
//!
//! # Arms
//!
//! - Apple: a keychain generic-password item, `service` = the store name,
//!   `account` = the key.
//! - Android: an AES-256-GCM key held by the Android Keystore (alias derived
//!   from the store name), ciphertext in `SharedPreferences`.
//! - Linux: libsecret through `secret-tool`.
//! - Windows: a file in the user's config directory, sealed with DPAPI.
//!
//! # Store names, keys, values
//!
//! A store name is an identifier: non-empty, no separators, no NUL — the same
//! rule `prefs` applies, because Windows turns it into a filename. Keys and
//! values are arbitrary UTF-8 except NUL.

/// A named store of secrets, backed by the OS's protected storage.
///
/// Cheap to hold: every arm re-reaches the underlying store per call, so
/// there is no handle to keep alive or leak.
#[derive(Debug, Clone)]
pub struct SecureStorage {
    service: String,
}

impl SecureStorage {
    /// Open (creating on first write) the named store.
    ///
    /// `service` is what the OS shows the user on Apple: use the app's
    /// bundle identifier, not a display name.
    pub fn open(service: &str) -> Result<SecureStorage, String> {
        validate_name(service)?;
        Ok(SecureStorage {
            service: service.to_string(),
        })
    }

    /// The secret for `key`, or `None` when the key is absent.
    pub fn get(&self, key: &str) -> Result<Option<String>, String> {
        let key = key_str(key)?;
        #[cfg(target_os = "android")]
        {
            android::get(&self.service, &key)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::get(&self.service, &key)
        }
        #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
        {
            linux::get(&self.service, &key)
        }
        #[cfg(target_os = "windows")]
        {
            windows::get(&self.service, &key)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::get(&self.service, &key)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )))]
        {
            let _ = &key;
            Err(unsupported())
        }
    }

    /// Store `value` under `key`, replacing any previous value.
    pub fn set(&self, key: &str, value: &str) -> Result<(), String> {
        let key = key_str(key)?;
        let value = key_str(value)?;
        #[cfg(target_os = "android")]
        {
            android::set(&self.service, &key, &value)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::set(&self.service, &key, &value)
        }
        #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
        {
            linux::set(&self.service, &key, &value)
        }
        #[cfg(target_os = "windows")]
        {
            windows::set(&self.service, &key, &value)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::set(&self.service, &key, &value)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )))]
        {
            let _ = (&key, &value);
            Err(unsupported())
        }
    }

    /// Remove `key`. Removing an absent key is `Ok(())`.
    pub fn remove(&self, key: &str) -> Result<(), String> {
        let key = key_str(key)?;
        #[cfg(target_os = "android")]
        {
            android::remove(&self.service, &key)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::remove(&self.service, &key)
        }
        #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
        {
            linux::remove(&self.service, &key)
        }
        #[cfg(target_os = "windows")]
        {
            windows::remove(&self.service, &key)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::remove(&self.service, &key)
        }
        #[cfg(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        )))]
        {
            let _ = &key;
            Err(unsupported())
        }
    }
}

#[cfg(not(any(
    target_os = "android",
    target_os = "ios",
    target_os = "macos",
    target_os = "linux",
    target_os = "windows"
)))]
fn unsupported() -> String {
    format!(
        "secure-storage: this target ({}) has no protected store",
        std::env::consts::OS
    )
}

fn key_str(s: &str) -> Result<String, String> {
    if s.contains('\0') {
        return Err(format!(
            "secure-storage: keys and values may not contain NUL ({s:?})"
        ));
    }
    Ok(s.to_string())
}

fn validate_name(service: &str) -> Result<(), String> {
    if service.is_empty() {
        return Err("secure-storage: the store name is empty".into());
    }
    if service.contains('/') || service.contains('\\') || service.contains('\0') {
        return Err(format!(
            "secure-storage: the store name {service:?} is an identifier, not a path — \
             no separators"
        ));
    }
    if service == "." || service == ".." {
        return Err(format!(
            "secure-storage: {service:?} is not a usable store name"
        ));
    }
    Ok(())
}

#[cfg(any(target_os = "android", test))]
mod hex;
#[cfg(any(target_os = "windows", test))]
mod map;

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
mod linux;
#[cfg(target_env = "ohos")]
mod ohos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(test)]
mod tests {
    use super::*;

    /// A store name no other run — or user — can collide with. The counter is
    /// what makes it unique within a process; the nanosecond stamp across runs.
    ///
    /// Only the Apple arm's tests need it (a Keychain item is global); on a host
    /// without that arm nothing calls it, which is not dead code but a helper
    /// for a platform this build does not have.
    #[cfg(target_vendor = "apple")]
    pub(super) fn unique_store_name() -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let seq = NEXT.fetch_add(1, Ordering::Relaxed);
        format!(
            "floom-secure-test-{}.local-{:x}-{seq}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    #[test]
    fn a_store_name_that_is_a_path_is_refused() {
        assert!(SecureStorage::open("").is_err());
        assert!(SecureStorage::open("a/b").is_err());
        assert!(SecureStorage::open("..").is_err());
        assert!(SecureStorage::open("com.example.app").is_ok());
    }
}
