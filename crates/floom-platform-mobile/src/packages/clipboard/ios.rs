//! iOS clipboard via `UIPasteboard`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

pub fn get_text() -> Result<Option<String>, String> {
    // TODO: `UIPasteboard.generalPasteboard.string` — `None` when the clipboard
    // holds no `NSString` (e.g. an image or a file URL).
    Err("clipboard requires the UIKit pasteboard".into())
}

pub fn set_text(_text: &str) -> Result<(), String> {
    // TODO: `UIPasteboard.generalPasteboard.string = text`.
    Err("clipboard requires the UIKit pasteboard".into())
}
