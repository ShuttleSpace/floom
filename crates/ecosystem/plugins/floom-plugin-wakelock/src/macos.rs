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
//! # Reading it back
//!
//! `is_keep_screen_on` asks the power manager about the held ID with
//! `IOPMAssertionCopyProperties`, which returns `NULL` once the ID is gone.
//! The mutex alone would be a shadow of what this crate last asked for; the
//! power manager is the only thing that knows whether the assertion still
//! stands (it reaps one whose process dies, and it can drop one for its own
//! reasons). The copy is a read — it neither creates nor changes anything —
//! and the dictionary it returns is ours to release.
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
pub const K_IO_RETURN_SUCCESS: i32 = 0;

/// `kIOReturnNotFound` — the power manager has no such ID.
///
/// For a *release* this is the goal already reached, not a failure: something
/// else (the reaper, or a release from elsewhere) took the assertion down. For
/// a create it would be a real error, which is why only the release path
/// treats it as success.
const K_IO_RETURN_NOT_FOUND: i32 = -536_870_206;

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(
        assertion_type: *const std::ffi::c_void,
        level: u32,
        assertion_name: *const std::ffi::c_void,
        assertion_id: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(assertion_id: u32) -> i32;
    /// `NULL` once the power manager no longer knows the ID — which is how
    /// this arm reads the assertion back instead of trusting its own memory.
    fn IOPMAssertionCopyProperties(assertion_id: u32) -> *const std::ffi::c_void;
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
            // `kIOReturnNotFound` is the state this call exists to reach:
            // the assertion is already gone, so the display is free to sleep.
            // Anything else that is not success is a real error the caller
            // should hear about — the state is cleared either way, because
            // the power manager reaps what the process held.
            if rc != K_IO_RETURN_SUCCESS && rc != K_IO_RETURN_NOT_FOUND {
                return Err(format!(
                    "keep_screen_on: IOPMAssertionRelease returned {rc}"
                ));
            }
            Ok(())
        }
    }
}

pub fn is_keep_screen_on() -> bool {
    let held = held().lock().ok().and_then(|held| *held);
    let Some(id) = held else {
        return false;
    };
    // Ask the power manager, not our own memory of what we asked for. This is
    // a read: `IOPMAssertionCopyProperties` neither creates nor changes an
    // assertion. A non-null dictionary means the ID is still live; null means
    // it is gone — reaped, or released behind our back — and reporting `true`
    // there would be the fabricated "the OS said so" the crate doc rules out.
    let properties = unsafe { IOPMAssertionCopyProperties(id) };
    if properties.is_null() {
        return false;
    }
    unsafe { CFRelease(properties) };
    true
}

/// The assertion ID this crate is tracking, for the test that proves the read
/// consults the power manager rather than this mutex.
#[cfg(test)]
pub fn held_id() -> Option<u32> {
    held().lock().ok().and_then(|held| *held)
}

/// Release an assertion without going through this crate's bookkeeping.
///
/// Test-only, and deliberately so: it exists to create the exact state the
/// reader must handle — the mutex says held, the power manager says gone.
#[cfg(test)]
pub unsafe fn release_out_of_band(id: u32) -> i32 {
    unsafe { IOPMAssertionRelease(id) }
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
