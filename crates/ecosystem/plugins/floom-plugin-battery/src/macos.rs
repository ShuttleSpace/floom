//! macOS battery state via IOKit's IOPS power-source API, called through the
//! raw C bindings in `IOKit.framework` / `CoreFoundation.framework`.
//!
//! `IOPSCopyPowerSourcesInfo` returns an opaque blob; `IOPSCopyPowerSourcesList`
//! turns it into a list of power-source handles; `IOPSGetPowerSourceDescription`
//! gives each one a dictionary keyed by the `kIOPS*` strings in IOPSKeys.h. The
//! values are real kernel reads, so this arm measures rather than stubs.
//!
//! No crate sits between this arm and the frameworks: the bindings are declared
//! here and linked against `IOKit` and `CoreFoundation`. The blob from
//! `IOPSCopyPowerSourcesInfo` owns the per-source dictionaries, so we retain it
//! for the whole read and `CFRelease` it (and the list) at the end.

use std::ffi::{CStr, c_char, c_int, c_void};

use super::BatteryState;

type CFTypeRef = *const c_void;
type CFStringRef = *const c_void;
type CFArrayRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFNumberRef = *const c_void;
type CFAllocatorRef = *const c_void;

const K_CF_STRING_ENCODING_UTF8: c_int = 0x0800_0100;
const K_CF_NUMBER_SINT32_TYPE: c_int = 3;

// Two framework links: clippy's `duplicated_attributes` lint flags the pair, but
// both are required — IOKit alone does not resolve the CoreFoundation symbols
// (`CFRelease`, `CFStringCreateWithCString`, ...). The suppression is local.
#[allow(clippy::duplicated_attributes)]
#[link(name = "IOKit", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFAllocatorDefault: CFAllocatorRef;
    fn IOPSCopyPowerSourcesInfo() -> CFTypeRef;
    fn IOPSCopyPowerSourcesList(blob: CFTypeRef) -> CFArrayRef;
    fn IOPSGetPowerSourceDescription(blob: CFTypeRef, ps: CFTypeRef) -> CFDictionaryRef;
    fn CFStringCreateWithCString(
        alloc: CFAllocatorRef,
        cstr: *const c_char,
        encoding: c_int,
    ) -> CFStringRef;
    fn CFDictionaryGetValue(dict: CFDictionaryRef, key: CFStringRef) -> CFTypeRef;
    fn CFArrayGetCount(array: CFArrayRef) -> c_int;
    fn CFArrayGetValueAtIndex(array: CFArrayRef, idx: c_int) -> CFTypeRef;
    fn CFNumberGetValue(number: CFNumberRef, typ: c_int, value_ptr: *mut c_void) -> u8;
    fn CFStringGetCString(
        string: CFStringRef,
        buffer: *mut c_char,
        max_len: c_int,
        encoding: c_int,
    ) -> u8;
    fn CFRelease(cf: CFTypeRef);
}

/// Look up a string-keyed value in an IOPS dictionary. Returns the raw
/// `CFTypeRef`, which is owned by the dictionary and must not be released here.
fn dict_get(dict: CFDictionaryRef, key: &str) -> Option<CFTypeRef> {
    unsafe {
        let key_ref = CFStringCreateWithCString(
            kCFAllocatorDefault,
            key.as_ptr() as *const c_char,
            K_CF_STRING_ENCODING_UTF8,
        );
        if key_ref.is_null() {
            return None;
        }
        let value = CFDictionaryGetValue(dict, key_ref);
        // The key we just created is ours to release.
        CFRelease(key_ref);
        if value.is_null() { None } else { Some(value) }
    }
}

/// Read an integer-valued IOPS field (e.g. `Current Capacity`, whole-number
/// percentage 0..100).
fn dict_i32(dict: CFDictionaryRef, key: &str) -> Option<i32> {
    let value = dict_get(dict, key)?;
    let mut out: i32 = 0;
    unsafe {
        if CFNumberGetValue(
            value as CFNumberRef,
            K_CF_NUMBER_SINT32_TYPE,
            &mut out as *mut i32 as *mut c_void,
        ) != 0
        {
            Some(out)
        } else {
            None
        }
    }
}

/// Read a string-valued IOPS field into an owned Rust `String`.
fn dict_str(dict: CFDictionaryRef, key: &str) -> Option<String> {
    let value = dict_get(dict, key)?;
    let mut buf = [0 as c_char; 64];
    unsafe {
        if CFStringGetCString(
            value as CFStringRef,
            buf.as_mut_ptr(),
            buf.len() as c_int,
            K_CF_STRING_ENCODING_UTF8,
        ) != 0
        {
            CStr::from_ptr(buf.as_ptr())
                .to_str()
                .ok()
                .map(|s| s.to_owned())
        } else {
            None
        }
    }
}

pub fn battery_state() -> BatteryState {
    unsafe {
        let blob = IOPSCopyPowerSourcesInfo();
        if blob.is_null() {
            return BatteryState::default();
        }
        let list = IOPSCopyPowerSourcesList(blob);
        if list.is_null() {
            CFRelease(blob);
            return BatteryState::default();
        }

        let mut best: Option<BatteryState> = None;
        let count = CFArrayGetCount(list);
        for i in 0..count {
            let ps = CFArrayGetValueAtIndex(list, i);
            let dict = IOPSGetPowerSourceDescription(blob, ps);
            if dict.is_null() {
                continue;
            }
            // `Current Capacity` is a whole-number percentage (0..100). An AC
            // adapter also appears in the list but reports no capacity, so skip
            // anything that is not a readable battery fraction.
            let Some(capacity) = dict_i32(dict, "Current Capacity").filter(|c| *c > 0) else {
                continue;
            };
            let is_charging = dict_str(dict, "Power Source State").as_deref() == Some("Charging");
            // `Is Charged` is a CFBoolean whose string form is "YES"/"NO".
            let is_full = dict_str(dict, "Is Charged")
                .as_deref()
                .is_some_and(|s| s.eq_ignore_ascii_case("yes"));
            best = Some(BatteryState {
                level: Some((capacity as f32 / 100.0).clamp(0.0, 1.0)),
                is_charging,
                is_full,
            });
        }

        CFRelease(list);
        CFRelease(blob);
        best.unwrap_or_default()
    }
}

pub fn is_low_power_mode() -> bool {
    // No public, version-stable macOS battery-saver flag reads cleanly here;
    // report honestly rather than guess.
    false
}
