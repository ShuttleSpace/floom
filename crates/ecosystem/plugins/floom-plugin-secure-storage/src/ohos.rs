//! OpenHarmony's Asset Store is not wired to this crate yet.

pub fn get(_service: &str, _key: &str) -> Result<Option<String>, String> {
    Err("secure-storage: OpenHarmony's Asset Store is not wired yet".into())
}

pub fn set(_service: &str, _key: &str, _value: &str) -> Result<(), String> {
    Err("secure-storage: OpenHarmony's Asset Store is not wired yet".into())
}

pub fn remove(_service: &str, _key: &str) -> Result<(), String> {
    Err("secure-storage: OpenHarmony's Asset Store is not wired yet".into())
}
