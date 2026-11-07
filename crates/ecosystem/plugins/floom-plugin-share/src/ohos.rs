//! OpenHarmony sharing is not wired to this crate yet.

pub fn share_text(_text: &str) -> Result<(), String> {
    Err("share: OpenHarmony sharing has no bridge in this crate yet".into())
}

pub fn share_files(_paths: &[&str]) -> Result<(), String> {
    Err("share: OpenHarmony sharing has no bridge in this crate yet".into())
}
