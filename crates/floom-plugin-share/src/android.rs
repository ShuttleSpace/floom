//! Android share via `Intent.ACTION_SEND` / `ACTION_SEND_MULTIPLE`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

pub fn share_text(_text: &str) -> Result<(), String> {
    // TODO: build an `Intent.ACTION_SEND` with `EXTRA_TEXT` and
    // `type="text/plain"`, then `startActivity` with a chooser.
    Err("share requires an Android activity".into())
}

pub fn share_files(_paths: &[&str]) -> Result<(), String> {
    // TODO: build `Intent.ACTION_SEND_MULTIPLE` with `EXTRA_STREAM` (content
    // URIs from `FileProvider`) and start the chooser.
    Err("share requires an Android activity".into())
}
