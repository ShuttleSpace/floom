//! Windows file dialogs: the classic pair.
//!
//! `GetOpenFileNameW`/`GetSaveFileNameW` (comdlg32) for files and
//! `SHBrowseForFolderW` (shell32) for directories. These are the legacy APIs
//! and they are fully supported; the modern spelling is the COM
//! `IFileDialog`, which costs a hand-written vtable walk with no Windows
//! machine to test on — the classic pair is a tenth of the code and equally
//! honest about what it shows.
//!
//! # `OFN_NOCHANGEDIR`, or the dialog that moved the process
//!
//! The comdlg32 pair has a documented side effect: by default the dialog
//! changes the process's current directory to wherever the user browsed.
//! A library silently moving the caller's cwd is a side effect nothing in
//! this crate is allowed to have, so `OFN_NOCHANGEDIR` is always set.
//!
//! # Cancel versus failure
//!
//! `GetOpenFileNameW` returns 0 for both. The difference is
//! `CommDlgExtendedError()`: 0 means the user cancelled, anything else names
//! the failure. That boundary is this crate's `Ok(None)` versus `Err`.

use super::Options;
use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::UI::Controls::Dialogs::{
    CommDlgExtendedError, GetOpenFileNameW, GetSaveFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR,
    OFN_OVERWRITEPROMPT, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Shell::{
    BIF_NEWDIALOGSTYLE, BIF_RETURNONLYFSDIRS, BROWSEINFOW, SHBrowseForFolderW, SHGetPathFromIDListW,
};

/// The comdlg32 path buffer. MAX_PATH (260) is the API's traditional limit,
/// but long paths are real; 4096 wide characters is the comfortable ceiling
/// the dialog respects when the buffer offers it.
const PATH_BUFFER: usize = 4096;

pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String> {
    file_dialog(options, false)
}

pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String> {
    file_dialog(options, true)
}

pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String> {
    directory_dialog(options)
}

/// The comdlg32 pair — one implementation because the open and save dialogs
/// differ only in flags and in which function is called.
fn file_dialog(options: &Options, save: bool) -> Result<Option<PathBuf>, String> {
    // The buffer carries the suggested name in, and the chosen path out.
    let mut buffer = [0u16; PATH_BUFFER];
    if let Some(name) = &options.suggested_name {
        copy_wide(name, &mut buffer);
    }
    let title: Option<Vec<u16>> = options.title.as_deref().map(wide);

    let mut ofn: OPENFILENAMEW = unsafe { core::mem::zeroed() };
    ofn.lStructSize = core::mem::size_of::<OPENFILENAMEW>() as u32;
    ofn.hwndOwner = core::ptr::null_mut();
    ofn.lpstrFile = buffer.as_mut_ptr();
    ofn.nMaxFile = buffer.len() as u32;
    ofn.lpstrTitle = title
        .as_deref()
        .map(|t| t.as_ptr())
        .unwrap_or(core::ptr::null());
    ofn.Flags = OFN_NOCHANGEDIR
        | OFN_PATHMUSTEXIST
        | if save {
            OFN_OVERWRITEPROMPT
        } else {
            OFN_FILEMUSTEXIST
        };

    let chosen = unsafe {
        if save {
            GetSaveFileNameW(&mut ofn)
        } else {
            GetOpenFileNameW(&mut ofn)
        }
    };
    if chosen == 0 {
        let extended = unsafe { CommDlgExtendedError() };
        if extended == 0 {
            return Ok(None); // the user cancelled
        }
        return Err(format!(
            "the file dialog failed (CommDlgExtendedError {extended})"
        ));
    }
    Ok(Some(wide_to_path(&buffer)))
}

/// `SHBrowseForFolderW` — the folder dialog.
fn directory_dialog(options: &Options) -> Result<Option<PathBuf>, String> {
    let title: Option<Vec<u16>> = options.title.as_deref().map(wide);
    // The display-name buffer the API wants filled; the chosen *path* comes
    // from `SHGetPathFromIDListW`, so this one is scratch.
    let mut display = [0u16; 1024];

    let mut bi: BROWSEINFOW = unsafe { core::mem::zeroed() };
    bi.hwndOwner = core::ptr::null_mut();
    bi.pszDisplayName = display.as_mut_ptr();
    bi.lpszTitle = title
        .as_deref()
        .map(|t| t.as_ptr())
        .unwrap_or(core::ptr::null());
    // FSDIRS keeps the answer a real filesystem path (not a virtual folder
    // like a library), and the new style gives the resizable modern dialog.
    bi.ulFlags = BIF_RETURNONLYFSDIRS | BIF_NEWDIALOGSTYLE;

    let pidl = unsafe { SHBrowseForFolderW(&bi) };
    if pidl.is_null() {
        return Ok(None); // the user cancelled
    }
    let mut path = [0u16; PATH_BUFFER];
    let ok = unsafe { SHGetPathFromIDListW(pidl, path.as_mut_ptr()) };
    // The PIDL is the caller's to free, on both the success and failure
    // paths — leaking it on failure would be the quiet kind of leak.
    unsafe { CoTaskMemFree(pidl.cast()) };
    if ok == 0 {
        return Err("the chosen folder has no file-system path".into());
    }
    Ok(Some(wide_to_path(&path)))
}

/// A Rust string as a nul-terminated wide string.
fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain([0]).collect()
}

/// Copy a string into a wide buffer, nul-terminated, truncating to fit.
///
/// Only used for the suggested name going *in* to the dialog; a name longer
/// than the buffer is a suggestion, not a contract, and truncation is the
/// documented comdlg32 behaviour for over-long inputs.
fn copy_wide(s: &str, buffer: &mut [u16]) {
    for (slot, unit) in buffer
        .iter_mut()
        .zip(OsStr::new(s).encode_wide().chain([0]))
    {
        *slot = unit;
    }
}

/// A wide buffer's contents up to its nul, as a path.
fn wide_to_path(buffer: &[u16]) -> PathBuf {
    let len = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len]))
}
