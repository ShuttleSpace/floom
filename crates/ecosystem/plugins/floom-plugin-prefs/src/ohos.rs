//! OpenHarmony preferences need an N-API bridge that is not wired yet.

pub fn get(_store: &str, _key: &str) -> Option<String> {
    None
}

pub fn set(_store: &str, _key: &str, _value: &str) -> Result<(), String> {
    Err("prefs: OpenHarmony preferences are not wired yet".into())
}

pub fn remove(_store: &str, _key: &str) -> Result<(), String> {
    Err("prefs: OpenHarmony preferences are not wired yet".into())
}
