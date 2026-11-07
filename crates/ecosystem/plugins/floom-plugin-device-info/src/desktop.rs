//! Desktop device facts, read through `std` — no FFI, no crate dependency.
//!
//! The Android/iOS arms reach into `Build` / `UIDevice`; the desktop arms read
//! what the OS already exposes to an ordinary process: the OS name is a
//! compile-time constant, the OS version lives in `/etc/os-release` (Linux) or
//! `SystemVersion.plist` (macOS), the locale is an environment variable, and the
//! host name is `/etc/hostname` or `$HOSTNAME`. None of it needs a native half.
//!
//! Fields the platform does not answer without a system API stay empty rather
//! than being invented: `model` and `manufacturer` have no `std` door on a
//! desktop (they would need DMI/`sysctl`/`WMI`, which is exactly the FFI this
//! arm avoids), and the Windows OS version has no `std` source either.

use std::env;
use std::fs;

use super::DeviceInfo;

/// The display OS name for the host triple, as a bare word a UI can show.
fn os_name() -> String {
    match env::consts::OS {
        "linux" => "Linux",
        "macos" => "macOS",
        "windows" => "Windows",
        other => other,
    }
    .to_string()
}

/// The OS version where `std` can read it; empty where it cannot.
fn os_version() -> String {
    read_os_version().unwrap_or_default()
}

/// The OS version for the active desktop platform, or `None` when `std` has no
/// door to it.
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
fn read_os_version() -> Option<String> {
    let blob = fs::read_to_string("/etc/os-release").ok()?;
    release_value(&blob, "PRETTY_NAME").or_else(|| release_value(&blob, "VERSION_ID"))
}

#[cfg(target_os = "macos")]
fn read_os_version() -> Option<String> {
    let plist = fs::read_to_string("/System/Library/CoreServices/SystemVersion.plist").ok()?;
    plist_value_after_key("ProductVersion")(plist)
}

#[cfg(not(any(target_os = "linux", target_env = "ohos", target_os = "macos")))]
fn read_os_version() -> Option<String> {
    // Windows has no `std` source for the version string; an honest `None`
    // beats a fabricated one.
    None
}

/// Parse `KEY="value"` from an os-release blob. Values may be single- or
/// double-quoted; the quotes are stripped so the caller gets the bare word.
#[cfg(all(target_os = "linux", not(target_env = "ohos")))]
fn release_value(blob: &str, key: &str) -> Option<String> {
    blob.lines()
        .find_map(|line| {
            let (k, v) = line.split_once('=')?;
            (k == key).then(|| v.trim().trim_matches('"').trim_matches('\'').to_string())
        })
        .filter(|v| !v.is_empty())
}

/// Split a macOS plist on a `<key>` and return the first `<string>` that
/// follows it. The file is Apple-generated XML, so a string search is reliable
/// enough for the two keys this arm reads.
#[cfg(target_os = "macos")]
fn plist_value_after_key<'a>(key: &'a str) -> impl Fn(String) -> Option<String> + 'a {
    move |plist: String| {
        let needle = format!("<key>{key}</key>");
        let after = plist.find(&needle)? + needle.len();
        let rest = &plist[after..];
        let start = rest.find("<string>")? + "<string>".len();
        let end = rest[start..].find("</string>")?;
        let value = rest[start..start + end].trim().to_string();
        (!value.is_empty()).then_some(value)
    }
}

/// The BCP-47 locale, from the standard POSIX precedence. The encoding suffix
/// (`en_US.UTF-8`) is dropped so the value stays `en_US`.
fn locale() -> String {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(value) = env::var(var) {
            let bare = value.split('.').next().unwrap_or(&value);
            if !bare.is_empty() {
                return bare.to_string();
            }
        }
    }
    String::new()
}

/// The host's name, where `std` can find it. Linux reads `/etc/hostname`; every
/// Unix/macOS host has `$HOSTNAME`; Windows has `$COMPUTERNAME`. Falls back to
/// `"unknown"` only when nothing is set.
fn host_name() -> String {
    #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
    {
        if let Ok(name) = fs::read_to_string("/etc/hostname")
            && !name.trim().is_empty()
        {
            return name.trim().to_string();
        }
    }
    for var in ["HOSTNAME", "COMPUTERNAME"] {
        if let Ok(name) = env::var(var)
            && !name.trim().is_empty()
        {
            return name.trim().to_string();
        }
    }
    "unknown".to_string()
}

pub fn device_info() -> DeviceInfo {
    DeviceInfo {
        model: String::new(),
        manufacturer: String::new(),
        os_name: os_name(),
        os_version: os_version(),
        sdk_version: None,
        // A desktop has no simulator/emulator distinction; the build runs on real
        // silicon by definition.
        is_physical_device: true,
        locale: locale(),
    }
}

pub fn device_name() -> String {
    host_name()
}
