//! Apple preferences via `NSUserDefaults`, per suite.
//!
//! `[[NSUserDefaults alloc] initWithSuiteName:store]` is the per-store
//! spelling — the same object the platform's own Settings panes read. Both
//! iOS and macOS answer through it, so one arm serves both.
//!
//! # What is synchronous here
//!
//! `setObject:forKey:` updates the suite's in-memory cache immediately — the
//! next `stringForKey:` in this process sees it — and the system owns the
//! plist flush. That is the platform's own contract for user defaults, and
//! this crate reports exactly it: `Ok` means stored as the OS stores it.

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::c_char;

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {}

pub fn get(store: &str, key: &str) -> Option<String> {
    // The Result-to-Option bridge is deliberate: a failed reach is not the
    // key's absence, and a caller asking for a value must not mistake one
    // for the other — the `ok()?` turns "could not even ask" into `None`
    // only at this boundary, where the API has no room for the reason.
    let defaults = unsafe { suite(store) }.ok()?;
    let key = unsafe { ns_string(key) }.ok()?;
    unsafe {
        let value: *mut AnyObject = msg_send![defaults, stringForKey: key];
        string_from_nsstring(value)
    }
}

pub fn set(store: &str, key: &str, value: &str) -> Result<(), String> {
    unsafe {
        let defaults = suite(store)?;
        let key = ns_string(key)?;
        let value = ns_string(value)?;
        let _: () = msg_send![defaults, setObject: value, forKey: key];
        Ok(())
    }
}

pub fn remove(store: &str, key: &str) -> Result<(), String> {
    unsafe {
        let defaults = suite(store)?;
        let key = ns_string(key)?;
        let _: () = msg_send![defaults, removeObjectForKey: key];
        Ok(())
    }
}

/// Remove a whole suite from the persistent store, best-effort.
///
/// `removePersistentDomainForName:` is an *instance* method (the header is
/// unambiguous, and an earlier draft sent it to the class, which objc2's
/// runtime check refused — correctly). Calling it drops every value in the
/// named domain and asks cfprefsd to delete the plist.
///
/// Exposed for tests: a test store reaches the user's *real* preferences, so it
/// must leave no domain behind. A production caller has no reason to erase a
/// whole store — that is the user's settings, not the app's to clear.
#[cfg(test)]
pub fn remove_domain(store: &str) {
    unsafe {
        let Ok(defaults) = suite(store) else {
            return;
        };
        let Ok(name) = ns_string(store) else {
            return;
        };
        let _: () = msg_send![defaults, removePersistentDomainForName: name];
    }
}

/// `[[NSUserDefaults alloc] initWithSuiteName:]`.
unsafe fn suite(store: &str) -> Result<*mut AnyObject, String> {
    let name = unsafe { ns_string(store) }?;
    let defaults: *mut AnyObject = msg_send![class!(NSUserDefaults), alloc];
    if defaults.is_null() {
        return Err("prefs: NSUserDefaults alloc returned null".into());
    }
    let suite: *mut AnyObject = msg_send![defaults, initWithSuiteName: name];
    if suite.is_null() {
        Err("prefs: initWithSuiteName returned null".into())
    } else {
        Ok(suite)
    }
}

/// An `NSString` object as a Rust string, with null meaning absent.
unsafe fn string_from_nsstring(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let cstr: *const c_char = unsafe { msg_send![value, UTF8String] };
    if cstr.is_null() {
        return None;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(cstr) }
        .to_string_lossy()
        .into_owned();
    if s.is_empty() { None } else { Some(s) }
}

/// `+[NSString stringWithUTF8String:]`.
unsafe fn ns_string(value: &str) -> Result<*mut AnyObject, String> {
    let c = std::ffi::CString::new(value)
        .map_err(|_| format!("prefs: the string contains a NUL byte: {value:?}"))?;
    let string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
    if string.is_null() {
        return Err("prefs: could not create an NSString".into());
    }
    Ok(string)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A real native roundtrip: set, read back, remove, read absence —
    /// through `NSUserDefaults` itself, on a suite named for this test and
    /// cleaned up after.
    ///
    /// This is the desktop-real doctrine with teeth: the arm's own code path
    /// (suite, `setObject:forKey:`, `stringForKey:`, `removeObjectForKey:`)
    /// runs against the platform's actual store in an ordinary host test.
    /// A selector typo or a lifetime mistake would fail here, not in a
    /// user's app.
    ///
    /// The suite name carries the process id *and* a nanosecond timestamp:
    /// unique per run, so a previous failed run that cleaned up late cannot
    /// make the roundtrip read another run's leftovers.
    #[test]
    fn apple_roundtrip_through_the_real_store() {
        // Named `suite_name`, not `suite`: the local would shadow the
        // `suite()` helper the cleanup below needs to call.
        let suite_name = format!(
            "floom-prefs-test-{}.local-{:x}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );

        set(&suite_name, "greeting", "hello = with\todd\ncharacters").unwrap();
        assert_eq!(
            get(&suite_name, "greeting").as_deref(),
            Some("hello = with\todd\ncharacters")
        );
        set(&suite_name, "second", "2").unwrap();
        remove(&suite_name, "greeting").unwrap();
        assert_eq!(get(&suite_name, "greeting"), None);
        assert_eq!(get(&suite_name, "second").as_deref(), Some("2"));

        // Leave no plist behind. `removePersistentDomainForName:` is an
        // *instance* method (the header is unambiguous about this, and an
        // earlier draft sent it to the class, which objc2's runtime check
        // refused — correctly); it removes every value from the named
        // domain, persistently. The now-empty plist file itself is
        // cfprefsd's to delete, asynchronously — this best-effort removal
        // covers the common case where it lingers past the test run.
        unsafe {
            let defaults = suite(&suite_name).unwrap();
            let name = ns_string(&suite_name).unwrap();
            let _: () = msg_send![defaults, removePersistentDomainForName: name];
        }
        if let Some(home) = std::env::var_os("HOME") {
            let plist = std::path::Path::new(&home)
                .join("Library/Preferences")
                .join(format!("{suite_name}.plist"));
            let _ = std::fs::remove_file(plist);
        }
    }
}
