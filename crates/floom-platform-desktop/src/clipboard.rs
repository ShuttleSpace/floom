//! The macOS clipboard: the *system* pasteboard, not a process-local string.
//!
//! Until this module existed, [`crate::winit_backend::WinitPlatform`]
//! implemented [`Clipboard`] as a `String` field. That satisfies the trait and
//! every unit test, and proves nothing: text written by a Floom app could not
//! be pasted anywhere else, and text copied in another app could not be read
//! back. A clipboard that only one process can see is not a clipboard.
//!
//! `NSPasteboard` is AppKit, so this module is macOS-only. Every other desktop
//! target keeps the honest in-process placeholder: a platform that has elected
//! not to reach the OS yet reports `false`/`None` rather than pretending.
//!
//! # Why the write clears first
//!
//! `NSPasteboard` accumulates types across writes, so a second `setString` can
//! leave a previous, different value readable under a different type (or leave
//! stale items in front of the new one). `clearContents` makes the write mean
//! "replace what is here" instead of "add another item".
//!
//! # Why `stringForType` is asked for `None`
//!
//! An empty pasteboard returns `nil`, not an empty string. Reporting that as
//! `Some("")` would make "nothing copied" indistinguishable from "somebody
//! copied nothing", which matters to a caller deciding whether to enable its
//! own paste action.

#[cfg(target_os = "macos")]
use objc2_app_kit::{NSPasteboard, NSPasteboardType, NSPasteboardTypeString};
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;

/// Read the **general** pasteboard as plain text. `None` when it holds nothing
/// readable as a string (including when another app owns a non-text item).
#[cfg(target_os = "macos")]
pub(crate) fn system_read_text() -> Option<String> {
    read_text(&NSPasteboard::generalPasteboard())
}

/// Write `text` to the **general** pasteboard, replacing what was there.
#[cfg(target_os = "macos")]
pub(crate) fn system_write_text(text: &str) -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    write_text(&pasteboard, text)
}

#[cfg(target_os = "macos")]
fn read_text(pasteboard: &NSPasteboard) -> Option<String> {
    let value = pasteboard.stringForType(string_type())?;
    Some(value.to_string())
}

#[cfg(target_os = "macos")]
fn write_text(pasteboard: &NSPasteboard, text: &str) -> bool {
    pasteboard.setString_forType(&NSString::from_str(text), string_type())
}

/// The plain-text UTI. A `&'static` AppKit global: reading an `extern "C"`
/// static is unsafe, which is the only reason this is a function rather than a
/// constant.
#[cfg(target_os = "macos")]
fn string_type() -> &'static NSPasteboardType {
    // SAFETY: `NSPasteboardTypeString` is an AppKit-provided global that lives
    // for the whole process; the &'static type alias is objc2's own modelling
    // of that lifetime.
    unsafe { NSPasteboardTypeString }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use objc2::rc::Retained;
    use objc2_app_kit::NSPasteboard;

    use super::{read_text, write_text};

    /// A pasteboard of its own, so a test never writes over what the person
    /// actually copied. `pasteboardWithUniqueName` is the API for exactly this.
    fn private_pasteboard() -> Retained<NSPasteboard> {
        NSPasteboard::pasteboardWithUniqueName()
    }

    /// The one property that separates a clipboard from a `String` field: text
    /// survives leaving the writer's frame, including through UTF-8 that the
    /// pasteboard is allowed to normalise (a Chinese sentence, not ASCII).
    #[test]
    fn text_round_trips_through_the_pasteboard() {
        let pasteboard = private_pasteboard();
        assert!(write_text(&pasteboard, "floom2 clipboard probe 4b9e"));
        assert_eq!(
            read_text(&pasteboard).as_deref(),
            Some("floom2 clipboard probe 4b9e")
        );

        assert!(write_text(&pasteboard, "你好，世界"));
        assert_eq!(read_text(&pasteboard).as_deref(), Some("你好，世界"));
    }

    /// An empty pasteboard reads as no text. `Some("")` here would make "paste"
    /// look available when there is nothing to paste.
    #[test]
    fn an_empty_pasteboard_reads_as_no_text() {
        let pasteboard = private_pasteboard();
        pasteboard.clearContents();
        assert_eq!(read_text(&pasteboard), None);
    }
}
