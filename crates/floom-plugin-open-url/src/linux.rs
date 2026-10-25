//! Linux URL opening via `xdg-open`.
//!
//! `xdg-open` is the freedesktop convention: it resolves the URL's scheme
//! through the desktop environment's default applications and spawns the
//! handler. It is not guaranteed present — a minimal container has none — so
//! a missing binary is this arm's most likely failure and is reported as
//! exactly that rather than as a broken capability.
//!
//! # What the exit status means
//!
//! `xdg-open` exits 0 once it has *handed the URL off*; it does not wait for
//! the handler to appear, and its manual documents that a 0 exit is not proof
//! a window opened. That matches this crate's contract — `Ok` claims the
//! routing accepted the request, nothing more — so the status is taken at
//! face value and no further probing is done.

use std::process::Command;

pub fn open_url(url: &str) -> Result<(), String> {
    let status = Command::new("xdg-open").arg(url).status().map_err(|e| {
        format!(
            "open_url: `xdg-open` could not be run ({e}) — it is part of \
                 xdg-utils, which a minimal system may not have installed"
        )
    })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "open_url: `xdg-open` exited with {status} — no handler for this URL's scheme, \
             or the desktop environment refused"
        ))
    }
}
