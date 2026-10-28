//! The Apple keychain: one generic-password item per key.
//!
//! `service` becomes `kSecAttrService`, the key becomes `kSecAttrAccount`, and
//! the value is the item's secret data. Accessibility is the framework's
//! default (`kSecAttrAccessibleWhenUnlocked`), which is also
//! `flutter_secure_storage`'s default.

use security_framework::base::Error;
use security_framework::passwords::{
    PasswordOptions, delete_generic_password, generic_password, set_generic_password,
};

/// `errSecItemNotFound`. `Error::code` is the raw `OSStatus`, and the crate
/// does not re-export the constant without pulling in `security-framework-sys`.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25_300;

pub fn get(service: &str, key: &str) -> Result<Option<String>, String> {
    match generic_password(PasswordOptions::new_generic_password(service, key)) {
        Ok(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| format!("secure-storage: the value under {key:?} is not UTF-8")),
        Err(e) if missing(&e) => Ok(None),
        Err(e) => Err(format!(
            "secure-storage: keychain read failed: {}",
            describe(&e)
        )),
    }
}

pub fn set(service: &str, key: &str, value: &str) -> Result<(), String> {
    set_generic_password(service, key, value.as_bytes())
        .map_err(|e| format!("secure-storage: keychain write failed: {}", describe(&e)))
}

pub fn remove(service: &str, key: &str) -> Result<(), String> {
    match delete_generic_password(service, key) {
        Ok(()) => Ok(()),
        Err(e) if missing(&e) => Ok(()),
        Err(e) => Err(format!(
            "secure-storage: keychain delete failed: {}",
            describe(&e)
        )),
    }
}

fn missing(e: &Error) -> bool {
    e.code() == ERR_SEC_ITEM_NOT_FOUND
}

fn describe(e: &Error) -> String {
    match e.message() {
        Some(message) => format!("{message} ({})", e.code()),
        None => format!("OSStatus {}", e.code()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SecureStorage;
    use crate::tests::unique_store_name;

    /// Removes the whole test store even when an assertion fails midway.
    struct Store(String);

    impl Store {
        fn open() -> Store {
            Store(unique_store_name())
        }

        fn storage(&self) -> SecureStorage {
            SecureStorage::open(&self.0).unwrap()
        }
    }

    impl Drop for Store {
        fn drop(&mut self) {
            for key in ["token", "overwritten", "absent"] {
                let _ = remove(&self.0, key);
            }
        }
    }

    #[test]
    fn a_secret_round_trips_through_the_keychain() {
        let store = Store::open();
        let storage = store.storage();

        assert_eq!(storage.get("token").unwrap(), None);
        storage.set("token", "s3cr3t").unwrap();
        assert_eq!(storage.get("token").unwrap().as_deref(), Some("s3cr3t"));
    }

    #[test]
    fn a_second_write_replaces_the_first() {
        let store = Store::open();
        let storage = store.storage();

        storage.set("overwritten", "first").unwrap();
        storage.set("overwritten", "second").unwrap();
        assert_eq!(
            storage.get("overwritten").unwrap().as_deref(),
            Some("second")
        );
    }

    #[test]
    fn removing_is_idempotent() {
        let store = Store::open();
        let storage = store.storage();

        storage.set("absent", "value").unwrap();
        storage.remove("absent").unwrap();
        assert_eq!(storage.get("absent").unwrap(), None);
        storage.remove("absent").unwrap();
    }

    #[test]
    fn an_empty_value_is_a_value_not_an_absence() {
        let store = Store::open();
        let storage = store.storage();

        storage.set("token", "").unwrap();
        assert_eq!(storage.get("token").unwrap().as_deref(), Some(""));
    }
}
