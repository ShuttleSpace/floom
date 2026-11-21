// SPDX-License-Identifier: MIT
//! The desktop clipboard: the *system* one, not a process-local string.
//!
//! Until this module existed, [`crate::winit_backend::WinitPlatform`]
//! implemented [`Clipboard`] as a `String` field. That satisfies the trait and
//! every unit test, and proves nothing: text written by a Floom app could not
//! be pasted anywhere else, and text copied in another app could not be read
//! back. A clipboard that only one process can see is not a clipboard.
//!
//! Three targets, three ways to reach the OS through code that already existed:
//! `NSPasteboard` on macOS, `Win32` clipboard calls on Windows (the `windows-sys`
//! dependency the plugins already pin), and on Linux the `wl-clipboard`/`xclip`/
//! `xsel` command that the session already has. Linux needs the command because
//! owning a selection means owning an event loop, which is the window's job and
//! not a clipboard's.
//!
//! When no path is available — a headless session, a Linux box with none of the
//! three commands — the answer is `false`/`None`, so the caller can fall back to
//! its in-process copy instead of believing it reached the desktop.
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

/// Read the **general** clipboard as plain text. `None` when it holds nothing
/// readable as a string, including when another app owns a non-text item.
#[cfg(target_os = "macos")]
pub(crate) fn system_read_text() -> Option<String> {
    read_text(&NSPasteboard::generalPasteboard())
}

/// Write `text` to the **general** clipboard, replacing what was there.
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

/// The Win32 clipboard: `CF_UNICODETEXT` only, which is the format every app
/// writes. `OpenClipboard` takes a window handle so Windows can scope ownership;
/// `None` (the whole session) means "whoever asks", which is what a helper
/// library can offer without reaching into the app's window.
#[cfg(target_os = "windows")]
mod win32 {
    use windows_sys::Win32::Foundation::{GlobalFree, HGLOBAL};
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, OpenClipboard, SetClipboardData,
    };
    use windows_sys::Win32::System::Memory::{
        GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock,
    };
    // `CF_UNICODETEXT` is a clipboard *format*, filed under `Ole` and typed
    // `CLIPBOARD_FORMAT` (u16); the clipboard calls take a `u32`.
    use windows_sys::Win32::System::Ole::CF_UNICODETEXT;

    const FORMAT: u32 = CF_UNICODETEXT as u32;

    pub(super) fn read() -> Option<String> {
        // SAFETY: `OpenClipboard` is balanced by `CloseClipboard` on every path,
        // and the text is copied out of the shared block before it is closed.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let handle = GetClipboardData(FORMAT);
            let text = if handle.is_null() {
                None
            } else {
                let ptr = GlobalLock(handle) as *const u16;
                if ptr.is_null() {
                    None
                } else {
                    let mut len = 0usize;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
                    GlobalUnlock(handle);
                    Some(text)
                }
            };
            CloseClipboard();
            text
        }
    }

    pub(super) fn write(text: &str) -> bool {
        let mut utf16: Vec<u16> = text.encode_utf16().collect();
        utf16.push(0);
        let bytes = std::mem::size_of_val(utf16.as_slice());
        // SAFETY: the block is allocated, filled and handed over without
        // escaping this function. `SetClipboardData` takes ownership on success,
        // so the only correct `GlobalFree` is the one on the failure path.
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return false;
            }
            EmptyClipboard();
            let block: HGLOBAL = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if block.is_null() {
                CloseClipboard();
                return false;
            }
            let dst = GlobalLock(block) as *mut u16;
            if dst.is_null() {
                GlobalFree(block);
                CloseClipboard();
                return false;
            }
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), dst, utf16.len());
            GlobalUnlock(block);
            let wrote = !SetClipboardData(FORMAT, block).is_null();
            if !wrote {
                GlobalFree(block);
            }
            CloseClipboard();
            wrote
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn system_read_text() -> Option<String> {
    win32::read()
}

#[cfg(target_os = "windows")]
pub(crate) fn system_write_text(text: &str) -> bool {
    win32::write(text)
}

/// Linux has no clipboard API in the kernel or the C library: a selection is
/// owned by a live client, and passing it around is a protocol (Wayland
/// `wl_data_device`, X11 selections). The desktop tool is that client. Asking it
/// costs one process spawn and reaches exactly what a person's paste would.
#[cfg(all(unix, not(target_os = "macos")))]
mod command_line {
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// Readers first Wayland (the default `wl-clipboard` pair), then the two X11
    /// tools. Each entry is the argument list that prints the selection on stdout.
    pub(super) const READERS: &[(&str, &[&str])] = &[
        ("wl-paste", &["--no-newline"]),
        ("xclip", &["-o", "-selection", "clipboard"]),
        ("xsel", &["-b", "-o"]),
    ];

    /// Writers: the text arrives on stdin.
    pub(super) const WRITERS: &[(&str, &[&str])] = &[
        ("wl-copy", &[]),
        ("xclip", &["-i", "-selection", "clipboard"]),
        ("xsel", &["-b", "-i"]),
    ];

    pub(super) fn read() -> Option<String> {
        for (tool, args) in READERS {
            let Ok(output) = Command::new(tool).args(*args).output() else {
                continue;
            };
            if !output.status.success() {
                continue;
            }
            if let Some(text) = text_from(tool, &output.stdout) {
                return Some(text);
            }
        }
        None
    }

    pub(super) fn write(text: &str) -> bool {
        for (tool, args) in WRITERS {
            let Ok(mut child) = Command::new(tool)
                .args(*args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            else {
                continue;
            };
            let wrote = match child.stdin.take() {
                Some(mut stdin) => stdin.write_all(text.as_bytes()).is_ok(),
                None => false,
            };
            if wrote && child.wait().map(|status| status.success()).unwrap_or(false) {
                return true;
            }
        }
        false
    }

    /// The clipboard text a tool printed.
    ///
    /// `wl-paste --no-newline` prints exactly what was copied; `xclip` and
    /// `xsel` add one newline because their job is printing a line. Trimming
    /// only their newline keeps a deliberately copied trailing blank line
    /// intact. Empty output means the selection is not text, which is `None`
    /// for the same reason an empty macOS pasteboard is.
    pub(super) fn text_from(tool: &str, stdout: &[u8]) -> Option<String> {
        let text = String::from_utf8_lossy(stdout);
        let text = if tool == "wl-paste" {
            text.as_ref()
        } else {
            text.strip_suffix('\n').unwrap_or(&text)
        };
        (!text.is_empty()).then(|| text.to_string())
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn system_read_text() -> Option<String> {
    command_line::read()
}

#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn system_write_text(text: &str) -> bool {
    command_line::write(text)
}

/// Targets with no arm above (a non-desktop host, or one nobody wrote a path
/// for) answer honestly rather than pretending.
#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(unix, not(target_os = "macos"))
)))]
pub(crate) fn system_read_text() -> Option<String> {
    None
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "windows",
    all(unix, not(target_os = "macos"))
)))]
pub(crate) fn system_write_text(_text: &str) -> bool {
    false
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
        // A headless session (a CI runner) has no pasteboard daemon: the write
        // is accepted, `changeCount` never leaves zero, and reading back gives
        // nothing. Probing the counter keeps the skip honest — the round trip
        // still runs wherever there is a session, and still fails there.
        if pasteboard.changeCount() == 0 {
            eprintln!(
                "floom-platform-desktop: no pasteboard daemon in this session, \
                 skipping the round trip"
            );
            return;
        }
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

/// What the Linux arm reads out of a tool's stdout. Pure, so the rule survives
/// on the platforms that cannot run the tools.
#[cfg(all(test, unix, not(target_os = "macos")))]
mod command_line_tests {
    use super::command_line::text_from;

    /// `xlclip`/`xsel` print a line, so their newline is theirs, not the
    /// clipboard's. A copied trailing blank line must survive as one.
    #[test]
    fn a_printing_tools_newline_is_not_part_of_the_text() {
        assert_eq!(text_from("xclip", b"hello\n").as_deref(), Some("hello"));
        assert_eq!(text_from("xclip", b"hello\n\n").as_deref(), Some("hello\n"));
        assert_eq!(text_from("xsel", b"a\nb\n").as_deref(), Some("a\nb"));
    }

    /// `wl-paste --no-newline` is already exact; trimming it would eat a real
    /// trailing newline.
    #[test]
    fn a_wayland_read_is_taken_verbatim() {
        assert_eq!(
            text_from("wl-paste", b"hello\n").as_deref(),
            Some("hello\n")
        );
        assert_eq!(
            text_from("wl-paste", b"\xe4\xbd\xa0\xe5\xa5\xbd").as_deref(),
            Some("你好")
        );
    }

    /// Nothing selected, or a selection that is not text.
    #[test]
    fn an_empty_read_is_no_text() {
        assert_eq!(text_from("wl-paste", b""), None);
        assert_eq!(text_from("xclip", b""), None);
    }
}

/// The round trip on a session that has a clipboard.
///
/// Two guards, both printed so a green run says which one it took: a runner with
/// no clipboard (no display on Linux, a service session on Windows) has nothing
/// to round-trip, and a developer's machine has a clipboard worth not
/// overwriting — that is what `CI` is for.
#[cfg(all(test, any(target_os = "windows", all(unix, not(target_os = "macos")))))]
mod session_tests {
    use super::{system_read_text, system_write_text};

    const PROBE: &str = "floom2 clipboard probe 4b9e";

    fn session_allows_it() -> bool {
        if std::env::var_os("CI").is_none() {
            eprintln!(
                "floom-platform-desktop: not overwriting this session's clipboard, \
                 run with CI=1 to include the round trip"
            );
            return false;
        }
        if !system_write_text(PROBE) {
            eprintln!(
                "floom-platform-desktop: no clipboard in this session, \
                 skipping the round trip"
            );
            return false;
        }
        true
    }

    #[test]
    fn text_round_trips_through_the_session_clipboard() {
        if !session_allows_it() {
            return;
        }
        assert_eq!(system_read_text().as_deref(), Some(PROBE));

        // UTF-16 and UTF-8 both have to survive, and a CJK string is the cheap
        // way to catch a byte/character mix-up in either direction.
        assert!(system_write_text("你好，世界"));
        assert_eq!(system_read_text().as_deref(), Some("你好，世界"));
    }
}
