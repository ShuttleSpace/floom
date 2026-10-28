//! Apple platform identity via `NSBundle`.
//!
//! On iOS and macOS the running app's metadata lives in its bundle's
//! `Info.plist`. `NSBundle.mainBundle` is the app bundle; when the binary runs
//! outside a bundle (a `cargo test` binary, a CLI) the info dictionary is
//! empty, and every field is `None` — exactly what the type promises.
//!
//! `CFBundleShortVersionString` is the marketing version (`"1.2.3"`);
//! `CFBundleVersion` is the build number and is a string on Apple platforms
//! (`"456"`, sometimes `"1.2.3.456"`). `CFBundleDisplayName` falls back to
//! `CFBundleName`, which is what a bare executable would set if it had a
//! bundle at all.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).

use super::AppInfo;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::c_char;

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

pub fn app_info() -> AppInfo {
    unsafe { read_info() }
}

unsafe fn read_info() -> AppInfo {
    let bundle: *mut AnyObject = msg_send![class!(NSBundle), mainBundle];
    if bundle.is_null() {
        return AppInfo::default();
    }
    let dict: *mut AnyObject = msg_send![bundle, infoDictionary];
    if dict.is_null() {
        return AppInfo::default();
    }

    // Each read is independent: a bundle can carry a version and no display
    // name (CLI-ish app bundles do exactly that), so one missing key must not
    // blank the others.
    let name = unsafe { read_string(dict, "CFBundleDisplayName") }
        .or_else(|| unsafe { read_string(dict, "CFBundleName") });
    let package_name = unsafe { read_string(dict, "CFBundleIdentifier") };
    let version = unsafe { read_string(dict, "CFBundleShortVersionString") };
    let build_number = unsafe { read_string(dict, "CFBundleVersion") };

    AppInfo {
        name,
        package_name,
        version,
        build_number,
    }
}

/// One `infoDictionary` entry as a Rust string, or `None` when the key is
/// absent, null, or empty.
///
/// Marked `unsafe` because `dict` must be a live `NSDictionary` — the caller
/// got it from `infoDictionary` one line earlier, and this function does not
/// check.
unsafe fn read_string(dict: *mut AnyObject, key: &str) -> Option<String> {
    let ns_key = unsafe { ns_string(key) }?;
    let val: *mut AnyObject = unsafe { msg_send![dict, objectForKey: ns_key] };
    if val.is_null() {
        return None;
    }
    let cstr: *const c_char = unsafe { msg_send![val, UTF8String] };
    if cstr.is_null() {
        return None;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(cstr) }
        .to_string_lossy()
        .into_owned();
    if s.is_empty() { None } else { Some(s) }
}

/// `+[NSString stringWithUTF8String:]`, which takes a C string rather than an
/// `NSString` — so no Foundation binding is needed to build one.
unsafe fn ns_string(value: &str) -> Option<*mut AnyObject> {
    let c = std::ffi::CString::new(value).ok()?;
    let string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
    if string.is_null() { None } else { Some(string) }
}
