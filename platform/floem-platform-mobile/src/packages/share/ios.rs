//! iOS share via `UIActivityViewController`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

pub fn share_text(_text: &str) -> Result<(), String> {
    // TODO: present a `UIActivityViewController` with the text item from the
    // key window's root view controller.
    Err("share requires an iOS view controller".into())
}

pub fn share_files(_paths: &[&str]) -> Result<(), String> {
    // TODO: present a `UIActivityViewController` with `NSURL` file items.
    Err("share requires an iOS view controller".into())
}
