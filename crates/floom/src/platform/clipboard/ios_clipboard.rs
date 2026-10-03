//! iOS clipboard bridge via `UIPasteboard` (`objc2-ui-kit`).
//!
//! Unlike Android (JNI) or desktop (copypasta), iOS exposes the system
//! clipboard as a process-wide singleton `UIPasteboard.generalPasteboard()`
//! reachable from anywhere. Its `string`/`setString` accessors are marked
//! `unsafe` because UIKit requires them to run on the main thread; Floom's iOS
//! event loop (winit runs `UIApplicationMain` on the main thread) satisfies
//! that, and `Clipboard::get_contents`/`set_contents` are only ever called from
//! Floom event handling, so we are on the main thread here.

use objc2_foundation::NSString;
use objc2_ui_kit::UIPasteboard;

use crate::platform::ClipboardError;

/// Read the system pasteboard as plain text.
pub(crate) fn get_contents() -> Result<String, ClipboardError> {
    // SAFETY: `UIPasteboard`'s accessors require the main thread. Floom's iOS
    // event loop runs on the main thread (winit drives `UIApplicationMain`),
    // and this is invoked from event handling, so the requirement holds.
    unsafe {
        let pb = UIPasteboard::generalPasteboard();
        match pb.string() {
            Some(s) => Ok(s.to_string()),
            // An empty / non-string pasteboard reads back as the empty string,
            // matching the desktop and Android behaviour.
            None => Ok(String::new()),
        }
    }
}

/// Write `contents` to the system pasteboard as plain text.
pub(crate) fn set_contents(contents: &str) -> Result<(), ClipboardError> {
    // SAFETY: see `get_contents` — main-thread requirement holds here too.
    unsafe {
        let pb = UIPasteboard::generalPasteboard();
        let s = NSString::from_str(contents);
        pb.setString(Some(&s));
    }
    Ok(())
}
