//! Ask the user for a file path — open, save-as, or pick a directory.
//!
//! This is `file_selector`'s question, and the catalog's A3: the other
//! implementation half of the handoff doc's C4 decision alongside
//! `open-url` (`docs/plugin-catalog.md`).
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # `None` is the user's answer, `Err` is the system's
//!
//! [`pick_file`] returning `Ok(None)` means the user looked at the dialog and
//! cancelled — a complete, successful interaction whose result is "no file".
//! `Err` means the dialog could not be shown or the system failed behind it.
//! Collapsing cancel into `Err` would force every caller to treat "the user
//! changed their mind" as an error path, and collapsing it into some sentinel
//! path would be worse. The distinction is the API.
//!
//! # Where the arms stand — and why two of them refuse
//!
//! - **macOS** is real: `NSOpenPanel`/`NSSavePanel` through AppKit, the same
//!   panels every native application shows.
//! - **Linux** is real: `zenity` (falling back to `kdialog`), the desktop's
//!   own dialog tool — the same spawn-the-tool shape `open-url` uses for
//!   `xdg-open`.
//! - **Windows** is real: the classic `GetOpenFileNameW`/`GetSaveFileNameW`
//!   and `SHBrowseForFolderW`.
//! - **Android and iOS refuse, precisely.** A file picker on a phone is not a
//!   blocking call: the system shows UI and hands the answer back through an
//!   `onActivityResult` (Android) or a picker-delegate callback (iOS), and the
//!   platform layer exposes no channel for either — the same gap
//!   `share_files` documents for `FileProvider`, and the same *shape* of gap
//!   the catalog's §8.1 records for the view plugins. The refusal names what
//!   is missing rather than pretending, and unblocking it is platform-layer
//!   work, not a plugin fix.
//!
//! # What this crate deliberately does not do
//!
//! No file *contents* — the answer is a path (on Android it would be a
//! `content://` URI needing `ContentResolver`, which is part of why that arm
//! is a refusal). No filters yet: every platform names them differently
//! (UTIs, MIME types, DOS patterns) and v1 answers the capability question —
//! "give me a path the user chose" — without prejudging whose filter
//! vocabulary becomes the crate's. No multiple selection: the answer is one
//! path; a multi-select API that has to degrade on two platforms would be a
//! promise this crate cannot keep uniformly.

use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

/// What the dialogs are asked, beyond the kind of pick itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The dialog's title. `None` leaves the platform's default ("Open",
    /// "Save", the folder dialog's own).
    pub title: Option<String>,
    /// A suggested file name — the save dialog's pre-filled name field, or
    /// the file dialogs' starting file. Ignored by the directory pick.
    pub suggested_name: Option<String>,
}

impl Options {
    /// No title, no suggested name — the platform defaults throughout.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Ask the user to choose an existing file.
///
/// `Ok(None)` is the user cancelling; see the crate doc for why that is not
/// an error.
pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::pick_file(options)
    }
    #[cfg(target_os = "linux")]
    {
        linux::pick_file(options)
    }
    #[cfg(target_os = "windows")]
    {
        windows::pick_file(options)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return refused("pick_file");
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "ios"
    )))]
    {
        let _ = options;
        Err(unsupported())
    }
}

/// Ask the user where to save a file.
///
/// The dialog confirms overwriting; the path is not created, and the file is
/// not written — this crate asks, the caller acts. `Ok(None)` is the user
/// cancelling.
pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::pick_save_file(options)
    }
    #[cfg(target_os = "linux")]
    {
        linux::pick_save_file(options)
    }
    #[cfg(target_os = "windows")]
    {
        windows::pick_save_file(options)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return refused("pick_save_file");
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "ios"
    )))]
    {
        let _ = options;
        Err(unsupported())
    }
}

/// Ask the user to choose an existing directory.
///
/// `Ok(None)` is the user cancelling.
pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "macos")]
    {
        macos::pick_directory(options)
    }
    #[cfg(target_os = "linux")]
    {
        linux::pick_directory(options)
    }
    #[cfg(target_os = "windows")]
    {
        windows::pick_directory(options)
    }
    #[cfg(any(target_os = "android", target_os = "ios"))]
    {
        return refused("pick_directory");
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows",
        target_os = "android",
        target_os = "ios"
    )))]
    {
        let _ = options;
        Err(unsupported())
    }
}

/// The phone arms' shared refusal, naming the missing plumbing.
///
/// Exists only on the phone targets — the arms that call it are cfg-gated to
/// them, and the reasons are platform-specific.
#[cfg(any(target_os = "android", target_os = "ios"))]
fn refused(call: &str) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "android")]
    let reason = "the picker's answer arrives as an activity result \
                  (ACTION_OPEN_DOCUMENT / ACTION_CREATE_DOCUMENT), and the platform layer's \
                  NativeActivity bridge exposes no onActivityResult channel";
    #[cfg(target_os = "ios")]
    let reason = "the picker's answer arrives as a UIDocumentPickerViewController \
                  delegate callback, and presenting one needs an Objective-C delegate class \
                  — the native half this plugin shape deliberately does not carry";
    Err(format!(
        "{call}: not available on {} yet — {reason}. \
         This is platform-layer plumbing (a result channel), not a plugin fix; \
         desktop pickers work today",
        std::env::consts::OS
    ))
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "android",
    target_os = "ios"
)))]
fn unsupported() -> String {
    format!(
        "file dialogs need platform UI — macOS, Linux and Windows have it; \
         this target ({}) does not",
        std::env::consts::OS
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_is_the_platforms_own() {
        // "No options" must mean the platform's defaults, not an implicit
        // title or name the caller never chose.
        let o = Options::new();
        assert_eq!(o.title, None);
        assert_eq!(o.suggested_name, None);
    }

    // Like `open-url`, there is deliberately no happy-path test: a successful
    // pick shows a real modal dialog and waits for a human. The verification
    // path is the `floom-capabilities` gallery's action button, which a human
    // presses and answers. What *is* testable is the refusal vocabulary, and
    // only on a target that has one.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    #[test]
    fn the_refusal_names_the_missing_plumbing() {
        let err = pick_file(&Options::new()).unwrap_err();
        // A refusal that said only "not supported" would hide the reason and
        // the way out; the assertion pins both halves of the sentence.
        assert!(err.contains("platform-layer plumbing"), "{err}");
        assert!(err.contains("not available on"), "{err}");
    }
}
