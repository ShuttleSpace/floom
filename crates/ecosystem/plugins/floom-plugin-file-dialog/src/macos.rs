//! macOS file dialogs via `NSOpenPanel` / `NSSavePanel`.
//!
//! The panels AppKit itself shows — the same objects every native
//! application's Open and Save commands use. `runModal` runs the panel's own
//! event loop and blocks until the user answers, which is exactly the
//! blocking shape this crate's API promises.
//!
//! # What `runModal`'s return means
//!
//! `NSModalResponseOK` (1) is "the user pressed Open/Save";
//! `NSModalResponseCancel` (0) is "the user cancelled". Everything else
//! (rare stop-modal codes) is treated as cancel, because from this side of
//! the API a stopped dialog has produced no choice — and "no choice" is
//! `Ok(None)`, the user's answer, not the system's failure.
//!
//! # Why there is no host test for this arm
//!
//! An attempt was made: create the panel and send the setup messages without
//! `runModal`, asserting the selectors resolve — everything short of showing
//! UI. It aborted, and the abort is the finding: `NSOpenPanel` cannot even be
//! *created* outside an `NSApplication` context. AppKit throws an Objective-C
//! exception, and Rust cannot catch foreign exceptions, so the test binary
//! died at the first `msg_send!`. The arm's real audience is a Floom app,
//! where the application object exists by the time a widget can press
//! anything. The honest coverage a host build can give this arm is
//! compilation plus the gallery's action button inside a real app — which is
//! the same verification path `open-url` documents.

use super::Options;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::c_char;
use std::path::PathBuf;

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

const NS_MODAL_RESPONSE_OK: i64 = 1;

pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String> {
    unsafe {
        let panel: *mut AnyObject = msg_send![class!(NSOpenPanel), openPanel];
        if panel.is_null() {
            return Err("pick_file: the system returned no open panel".into());
        }
        set_title(panel, options)?;
        // Files yes, directories no — this is the "choose a file" question.
        let _: () = msg_send![panel, setCanChooseFiles: true];
        let _: () = msg_send![panel, setCanChooseDirectories: false];
        // One file: the crate's answer is a single path, so the panel is not
        // allowed to offer the multi-select that would have nowhere to go.
        let _: () = msg_send![panel, setAllowsMultipleSelection: false];
        answer_from_open_panel(panel)
    }
}

pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String> {
    unsafe {
        let panel: *mut AnyObject = msg_send![class!(NSSavePanel), savePanel];
        if panel.is_null() {
            return Err("pick_save_file: the system returned no save panel".into());
        }
        set_title(panel, options)?;
        // The save panel's name field is where a suggestion belongs; an
        // `NSOpenPanel` has no name field, so the option is ignored there
        // rather than routed into a directory guess.
        if let Some(name) = &options.suggested_name {
            let name = ns_string(name)?;
            let _: () = msg_send![panel, setNameFieldStringValue: name];
        }
        let response: i64 = msg_send![panel, runModal];
        if response != NS_MODAL_RESPONSE_OK {
            return Ok(None);
        }
        let url: *mut AnyObject = msg_send![panel, URL];
        if url.is_null() {
            return Ok(None);
        }
        path_from_url(url)
    }
}

pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String> {
    unsafe {
        let panel: *mut AnyObject = msg_send![class!(NSOpenPanel), openPanel];
        if panel.is_null() {
            return Err("pick_directory: the system returned no open panel".into());
        }
        set_title(panel, options)?;
        // The directory question, precisely inverted from `pick_file`.
        let _: () = msg_send![panel, setCanChooseFiles: false];
        let _: () = msg_send![panel, setCanChooseDirectories: true];
        answer_from_open_panel(panel)
    }
}

/// `setTitle:` when a title was given — absent means the platform's own.
unsafe fn set_title(panel: *mut AnyObject, options: &Options) -> Result<(), String> {
    if let Some(title) = &options.title {
        let title = unsafe { ns_string(title) }?;
        let _: () = msg_send![panel, setTitle: title];
    }
    Ok(())
}

/// Run an open panel and take the first (only) URL as the answer.
unsafe fn answer_from_open_panel(panel: *mut AnyObject) -> Result<Option<PathBuf>, String> {
    let response: i64 = msg_send![panel, runModal];
    if response != NS_MODAL_RESPONSE_OK {
        return Ok(None);
    }
    let urls: *mut AnyObject = msg_send![panel, URLs];
    if urls.is_null() {
        return Ok(None);
    }
    let first: *mut AnyObject = msg_send![urls, firstObject];
    if first.is_null() {
        return Ok(None);
    }
    unsafe { path_from_url(first) }
}

/// An `NSURL`'s file-system path, as a `PathBuf`.
unsafe fn path_from_url(url: *mut AnyObject) -> Result<Option<PathBuf>, String> {
    let path: *mut AnyObject = msg_send![url, path];
    if path.is_null() {
        return Ok(None);
    }
    let cstr: *const c_char = msg_send![path, UTF8String];
    if cstr.is_null() {
        return Ok(None);
    }
    let s = unsafe { std::ffi::CStr::from_ptr(cstr) }
        .to_string_lossy()
        .into_owned();
    if s.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(s)))
    }
}

/// `+[NSString stringWithUTF8String:]` — a C string in, an `NSString` out,
/// with no Foundation binding needed to build one.
unsafe fn ns_string(value: &str) -> Result<*mut AnyObject, String> {
    let c = std::ffi::CString::new(value)
        .map_err(|_| format!("the string contains a NUL byte: {value:?}"))?;
    let string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
    if string.is_null() {
        return Err("could not create an NSString".into());
    }
    Ok(string)
}
