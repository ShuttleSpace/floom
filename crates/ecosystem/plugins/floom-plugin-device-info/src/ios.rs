//! iOS device facts via `UIDevice`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! `UIDevice.currentDevice` carries the model string, the OS name and the OS
//! version. Two `DeviceInfo` fields have no public iOS source and are reported
//! as such rather than guessed:
//!
//! * `sdk_version` — iOS has no API level in Android's sense; the OS version
//!   *is* the API surface, so this stays `None`.
//! * `is_physical_device` — answered from the simulator's own environment
//!   marker (see [`is_simulator`]).
//!
//! `model` is the internal identifier (`"iPhone15,2"`), which is what the public
//! API gives. Mapping it to a marketing name (`"iPhone 14 Pro"`) needs a lookup
//! table Apple does not publish and that changes every year, so it is not
//! attempted here — the identifier is honest and stable.

use super::DeviceInfo;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

/// The `NSString` behind a `UTF8String` read, copied into a Rust `String`.
///
/// `UTF8String` hands back a pointer valid for the receiver's lifetime, which
/// outlives this function because the receiver is an autoreleased object owned
/// by UIKit — so the copy happens well inside the borrow.
///
/// # Safety
///
/// `object` must be a live Objective-C object that responds to `UTF8String`
/// with an `NSString`-compatible result. Every caller passes a `UIDevice` or
/// `NSLocale` property, which does.
unsafe fn utf8_string(object: *mut AnyObject) -> String {
    if object.is_null() {
        return String::new();
    }
    // Edition 2024 requires the unsafe operation to be written out even inside
    // an `unsafe fn`; the message send and the C-string read are both covered
    // by this function's own safety contract.
    unsafe {
        let cstr: *const std::ffi::c_char = msg_send![object, UTF8String];
        if cstr.is_null() {
            return String::new();
        }
        std::ffi::CStr::from_ptr(cstr)
            .to_string_lossy()
            .into_owned()
    }
}

pub fn device_info() -> DeviceInfo {
    unsafe {
        let device: *mut AnyObject = msg_send![class!(UIDevice), currentDevice];
        if device.is_null() {
            return DeviceInfo::default();
        }
        // Each property is read with its own literal selector: `msg_send!`
        // resolves the selector at compile time, so a runtime string dispatch
        // is not available and would be the wrong shape anyway.
        DeviceInfo {
            model: utf8_string(msg_send![device, model]),
            manufacturer: "Apple".to_string(),
            os_name: utf8_string(msg_send![device, systemName]),
            os_version: utf8_string(msg_send![device, systemVersion]),
            sdk_version: None,
            is_physical_device: !is_simulator(),
            locale: locale(),
        }
    }
}

pub fn device_name() -> String {
    unsafe {
        let device: *mut AnyObject = msg_send![class!(UIDevice), currentDevice];
        if device.is_null() {
            return String::new();
        }
        // `UIDevice.name` is the user-visible name ("Humphrey's iPhone"), which
        // is what this function documents — unlike `model`, the internal
        // identifier.
        utf8_string(msg_send![device, name])
    }
}

/// Whether this process runs in the simulator.
///
/// The simulator exports `SIMULATOR_DEVICE_NAME` into the process environment
/// and a device does not, which makes the environment the documented signal. It
/// is read through `std::env` rather than `NSProcessInfo.environment` because
/// the two are the same dictionary and `std::env` needs no framework call — and
/// because a `TARGET_OS_SIMULATOR` compile-time check would answer the wrong
/// question for a universal binary that ships both slices.
fn is_simulator() -> bool {
    std::env::var_os("SIMULATOR_DEVICE_NAME").is_some()
}

/// The current locale as a BCP-47 tag, via `NSLocale.preferredLanguages`.
///
/// The first entry is the user's primary language and is already a BCP-47 tag
/// (`en-US`), so no conversion is needed — unlike `Locale.current.identifier`,
/// which uses `_`.
fn locale() -> String {
    unsafe {
        let languages: *mut AnyObject = msg_send![class!(NSLocale), preferredLanguages];
        if languages.is_null() {
            return String::new();
        }
        let first: *mut AnyObject = msg_send![languages, firstObject];
        utf8_string(first)
    }
}
