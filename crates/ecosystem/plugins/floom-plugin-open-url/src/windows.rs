//! Windows URL opening via `ShellExecuteW`.
//!
//! The classic Windows routing: `ShellExecuteW(null, "open", url, …)` resolves
//! the URL's scheme through the registry's URL protocol associations and
//! spawns the handler. The return value is an `HINSTANCE` the documentation
//! defines as a status code — greater than 32 means accepted, anything else
//! is a specific failure value (`SE_ERR_FNF` and friends, all ≤ 32) — so that
//! boundary is the whole check and the raw value is passed through in the
//! error for a caller who wants which one.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// The `HINSTANCE` values ShellExecuteW documents as failure are all ≤ 32;
/// anything above is a handle-ish success token the caller does not need.
const SHELL_EXECUTE_SUCCESS_FLOOR: usize = 32;

pub fn open_url(url: &str) -> Result<(), String> {
    // `ShellExecuteW` takes wide strings, and the verb must be "open" —
    // a null verb would also mean "open" for a URL, but naming it keeps the
    // intent readable and the behaviour identical.
    let verb: Vec<u16> = OsStr::new("open").encode_wide().chain([0]).collect();
    let file: Vec<u16> = OsStr::new(url).encode_wide().chain([0]).collect();
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result as usize > SHELL_EXECUTE_SUCCESS_FLOOR {
        Ok(())
    } else {
        Err(format!(
            "open_url: ShellExecuteW returned {} — the documented failure \
             values (all ≤ 32) name the reason: no association for this URL's scheme, \
             the handler failed to launch, or it was blocked",
            result as usize
        ))
    }
}
