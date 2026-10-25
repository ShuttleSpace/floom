//! macOS screen-awake via `IOPMAssertionCreateWithName` (IOKit).
//!
//! The power-management assertion is what a native video player holds:
//! `kIOPMAssertionTypePreventUserIdleDisplaySleep` with a name that shows up
//! in `pmset -g assertions` — which is also how a curious user can watch
//! this crate's ask appear and disappear.
//!
//! # Why the assertion ID is held
//!
//! An assertion is balanced by `IOPMAssertionRelease` with the ID the create
//! call returned; creating a second one instead of releasing the first is a
//! leak the power manager will happily hold forever. So the ID lives in a
//! mutex, and the second `keep_screen_on(true)` is a no-op — idempotence is
//! a correctness requirement here, not a courtesy.
//!
//! The assertion dies with the process in any case (the power manager
//! reaps it), which is the guarantee the crate documents: while your app is
//! alive and asking.

use std::sync::{Mutex, OnceLock};

/// `IOPMAssertionType` — a CFString naming the kind of ask.
/// `kIOPMAssertionTypePreventUserIdleDisplaySleep`, spelled as its C string
/// because no binding crate is involved.
const PREVENT_USER_IDLE_DISPLAY_SLEEP: &[u8] = b"PreventUserIdleDisplaySleep\0";

/// `kIOPMAssertionLevelOn`.
const ASSERTION_LEVEL_ON: u32 = 255;

/// `kCFStringEncodingUTF8`.
const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

/// `kIOReturnSuccess`.
const K_IO_RETURN_SUCCESS: i32 = 0;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: *const std::ffi::c_void,
        level: u32,
        assertion_name: *const std::ffi::c_void,
        assertion_id: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(assertion_id: u32) -> i32;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringCreateWithCString(
        alloc: *const std::ffi::c_void,
        c_str: *const std::ffi::c_char,
        encoding: u32,
    ) -> *const std::ffi::c_void;
    fn CFRelease(cf: *const std::ffi::c_void);
}

/// The assertion this process holds, if any.
///
/// Guarded rather than atomic because create/release need the ID read and
/// the create decision as one step — two enables racing must not mint two
/// assertions.
fn held() -> &'static Mutex<Option<u32>> {
    static HELD: OnceLock<Mutex<Option<u32>>> = OnceLock::new();
    HELD.get_or_init(|| Mutex::new(None))
}

pub fn keep_screen_on(on: bool) -> Result<(), String> {
    let mut held = held()
        .lock()
        .map_err(|_| "keep_screen_on: the assertion state is poisoned".to_string())?;
    match (on, *held) {
        // The no-ops that make idempotence a guarantee rather than luck.
        (true, Some(_)) => Ok(()),
        (false, None) => Ok(()),
        (true, None) => {
            let id = unsafe { create_assertion() }?;
            *held = Some(id);
            Ok(())
        }
        (false, Some(id)) => {
            let rc = unsafe { IOPMAssertionRelease(id) };
            *held = None;
            if rc != K_IO_RETURN_SUCCESS {
                // The state is cleared either way — the power manager reaps
                // what the process held — but the caller hears that the
                // release itself went wrong.
                return Err(format!(
                    "keep_screen_on: IOPMAssertionRelease returned {rc}"
                ));
            }
            Ok(())
        }
    }
}

pub fn is_keep_screen_on() -> bool {
    held().lock().map(|held| held.is_some()).unwrap_or(false)
}

/// One `PreventUserIdleDisplaySleep` assertion, named for this crate.
unsafe fn create_assertion() -> Result<u32, String> {
    let name = b"floom-plugin-wakelock\0";
    let type_str = unsafe {
        CFStringCreateWithCString(
            std::ptr::null(),
            PREVENT_USER_IDLE_DISPLAY_SLEEP.as_ptr().cast(),
            K_CF_STRING_ENCODING_UTF8,
        )
    };
    let name_str = unsafe {
        CFStringCreateWithCString(
            std::ptr::null(),
            name.as_ptr().cast(),
            K_CF_STRING_ENCODING_UTF8,
        )
    };
    if type_str.is_null() || name_str.is_null() {
        // Whatever was created must not leak just because the other was not.
        if !type_str.is_null() {
            unsafe { CFRelease(type_str) };
        }
        if !name_str.is_null() {
            unsafe { CFRelease(name_str) };
        }
        return Err("keep_screen_on: could not build the assertion strings".into());
    }

    let mut id: u32 = 0;
    let rc = unsafe {
        IOPMAssertionCreateWithName(
            type_str,
            ASSERTION_LEVEL_ON,
            name_str,
            std::ptr::addr_of_mut!(id),
        )
    };
    // Both strings are ours; the power manager copies what it needs.
    unsafe { CFRelease(type_str) };
    unsafe { CFRelease(name_str) };

    if rc != K_IO_RETURN_SUCCESS {
        return Err(format!(
            "keep_screen_on: IOPMAssertionCreateWithName returned {rc}"
        ));
    }
    Ok(id)
}
