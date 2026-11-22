// SPDX-License-Identifier: MIT
//! Rust logs into logcat, behind a switch.
//!
//! `adb shell setprop debug.floom.log debug` then restart the app. Without the
//! property nothing is installed: logging costs a JNI call per record, and an
//! app that never asked for it should not pay for it.
//!
//! The switch is a system property rather than an env var because an APK has no
//! environment to read — `setprop` is the one knob `adb` can turn on a device.

#[cfg(target_os = "android")]
use std::sync::Once;

use log::LevelFilter;

/// The property `adb shell setprop` writes.
pub const SWITCH: &str = "debug.floom.log";

/// The logcat tag every record carries; `adb logcat -s floom` filters to ours.
pub const TAG: &str = "floom";

/// Level named by `value`, or `None` when logging is off.
///
/// `"1"` is accepted as info: the shortest thing to type when you only want
/// to know whether it works at all.
pub fn level_from_property(value: &str) -> Option<LevelFilter> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "off" | "none" => None,
        "1" | "info" => Some(LevelFilter::Info),
        "debug" => Some(LevelFilter::Debug),
        "trace" => Some(LevelFilter::Trace),
        "warn" | "warning" => Some(LevelFilter::Warn),
        "error" => Some(LevelFilter::Error),
        _ => None,
    }
}

/// Installs the logcat logger if the switch is set. Safe to call more than once.
#[cfg(target_os = "android")]
pub fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let Some(level) = level_from_property(&property(SWITCH)) else {
            return;
        };
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag(TAG)
                .with_max_level(level),
        );
        log::info!("logging to logcat at {level}; {SWITCH} set the level");
    });
}

/// Installs the logcat logger at `level`, whatever the property says.
///
/// For the tests and for an app that wants a level without an `adb` step.
#[cfg(target_os = "android")]
pub fn install_at(level: LevelFilter) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        android_logger::init_once(
            android_logger::Config::default()
                .with_tag(TAG)
                .with_max_level(level),
        );
    });
}

/// Reads an Android system property (bionic's `__system_property_get`).
#[cfg(target_os = "android")]
fn property(name: &str) -> String {
    use std::ffi::{CStr, CString};

    const VALUE_MAX: usize = 92;

    unsafe extern "C" {
        fn __system_property_get(
            name: *const std::ffi::c_char,
            value: *mut std::ffi::c_char,
        ) -> i32;
    }

    let Ok(name) = CString::new(name) else {
        return String::new();
    };
    let mut value = [0 as std::ffi::c_char; VALUE_MAX];
    // SAFETY: both buffers are valid for the call, and bionic writes at most
    // `PROP_VALUE_MAX` bytes into `value`.
    let len = unsafe { __system_property_get(name.as_ptr(), value.as_mut_ptr()) };
    if len <= 0 {
        return String::new();
    }
    unsafe { CStr::from_ptr(value.as_ptr()) }
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_switch_reads_a_level_or_nothing() {
        assert_eq!(level_from_property(""), None);
        assert_eq!(level_from_property("0"), None);
        assert_eq!(level_from_property("off"), None);
        assert_eq!(level_from_property("1"), Some(LevelFilter::Info));
        assert_eq!(level_from_property(" DeBuG "), Some(LevelFilter::Debug));
        assert_eq!(level_from_property("warn"), Some(LevelFilter::Warn));
        // A typo installs nothing rather than guessing a level.
        assert_eq!(level_from_property("verbose"), None);
    }
}
