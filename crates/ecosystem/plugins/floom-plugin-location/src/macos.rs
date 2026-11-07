//! macOS location via `CLLocationManager`.
//!
//! CoreLocation on macOS is the same `CLLocationManager` API as iOS — Touch ID's
//! neighbour, the system Location Services switch and the authorization state all
//! read the same way. Adapted from the iOS arm (`ios.rs`), which documents the
//! synchronous/async split in detail; the two questions macOS answers
//! synchronously are the same two:
//!
//! * `authorizationStatus` on a `CLLocationManager` instance returns the current
//!   `CLAuthorizationStatus` — no prompt, no callback, just the value.
//! * `+[CLLocationManager locationServicesEnabled]` is the system-wide switch,
//!   a different fact from the app's permission.
//!
//! `requestWhenInUseAuthorization` is available on macOS 11.0+ and behaves like
//! iOS: it returns immediately, the answer arrives later through
//! `locationManagerDidChangeAuthorization:`, and the caller polls
//! [`authorization_status`]. It refuses when already `Denied` and when the app
//! bundle has no usage description (the prompt genuinely never appears then).
//!
//! `current_location` refuses for the same run-loop reason as iOS: fixes are
//! delivered to the delegate on the creating thread's run loop (the main
//! thread), and a synchronous read cannot wait for one without stopping the run
//! loop that would deliver it. There is no cached-fix shortcut either.

use super::LocationAuthorization;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use objc2_core_location::{CLAuthorizationStatus, CLLocationManager};
use std::cell::RefCell;
use std::ffi::CString;

thread_local! {
    /// The manager that owns the outstanding authorization request.
    ///
    /// `requestWhenInUseAuthorization` is delivered on behalf of the manager
    /// instance, so the instance has to outlive the call — dropping it
    /// immediately would let `Ok` be returned for a prompt the system never
    /// shows. It is a thread-local rather than a static because
    /// `CLLocationManager` is not `Send`, and Floom's macOS event loop is
    /// main-thread.
    static REQUESTING_MANAGER: RefCell<Option<Retained<CLLocationManager>>> =
        const { RefCell::new(None) };
}

pub fn authorization_status() -> LocationAuthorization {
    // A fresh manager is documented as cheap, and the instance property is the
    // non-deprecated route to the same value the class method returns.
    let manager = unsafe { CLLocationManager::new() };
    let status = unsafe { manager.authorizationStatus() };
    if status == CLAuthorizationStatus::NotDetermined {
        LocationAuthorization::NotDetermined
    } else if status == CLAuthorizationStatus::Restricted {
        LocationAuthorization::Restricted
    } else if status == CLAuthorizationStatus::Denied {
        LocationAuthorization::Denied
    } else if status == CLAuthorizationStatus::AuthorizedWhenInUse {
        LocationAuthorization::AuthorizedWhenInUse
    } else if status == CLAuthorizationStatus::AuthorizedAlways {
        LocationAuthorization::AuthorizedAlways
    } else {
        // `kCLAuthorizationStatusAuthorized` is the deprecated alias of
        // `AuthorizedAlways` and compares equal to it, so this arm is only
        // reached by a value newer than the binding knows. `NotDetermined` is
        // the safe report: it makes the caller ask rather than assume a grant.
        LocationAuthorization::NotDetermined
    }
}

pub fn is_location_enabled() -> bool {
    // The class method is the system-wide switch. It is not deprecated and
    // needs no instance, so no manager is allocated for this question.
    unsafe { CLLocationManager::locationServicesEnabled_class() }
}

pub fn request_authorization() -> Result<(), String> {
    match authorization_status() {
        LocationAuthorization::AuthorizedAlways | LocationAuthorization::AuthorizedWhenInUse => {
            Ok(())
        }
        LocationAuthorization::Restricted => Err(
            "location: the app is restricted (parental controls or device \
             management), so macOS will not show a permission prompt"
                .into(),
        ),
        LocationAuthorization::Denied => Err(
            "location: permission was denied; macOS will not prompt a second time. \
             The user has to re-grant it from System Settings → Privacy & Security \
             → Location Services."
                .into(),
        ),
        LocationAuthorization::NotDetermined => {
            // Without a usage description in Info.plist the prompt is suppressed
            // by the system — no error, no dialog. Checking the key first is
            // what lets `Ok` mean "a prompt is on screen" rather than "a call
            // returned".
            if !has_usage_description() {
                return Err(
                    "location: Info.plist has no NSLocationWhenInUseUsageDescription, \
                     so macOS will not show the prompt. Add that key with the \
                     sentence the user should see."
                        .into(),
                );
            }
            let manager = unsafe { CLLocationManager::new() };
            // Fire-and-forget by design: the answer arrives in
            // `locationManagerDidChangeAuthorization:`, which this plugin does
            // not take, so the caller polls `authorization_status()`.
            unsafe { manager.requestWhenInUseAuthorization() };
            // The request belongs to the instance, so it is kept alive here
            // rather than dropped at the end of this arm.
            REQUESTING_MANAGER.with(|slot| *slot.borrow_mut() = Some(manager));
            Ok(())
        }
    }
}

/// Whether `Info.plist` carries `NSLocationWhenInUseUsageDescription`.
///
/// `objectForInfoDictionaryKey:` returns nil for a missing key, so this is a
/// real lookup rather than a guess about the build configuration.
fn has_usage_description() -> bool {
    unsafe {
        let bundle: *mut AnyObject = msg_send![class!(NSBundle), mainBundle];
        if bundle.is_null() {
            return false;
        }
        let key = match CString::new("NSLocationWhenInUseUsageDescription") {
            Ok(key) => key,
            Err(_) => return false,
        };
        let string: *mut AnyObject =
            msg_send![class!(NSString), stringWithUTF8String: key.as_ptr()];
        if string.is_null() {
            return false;
        }
        let value: *mut AnyObject = msg_send![bundle, objectForInfoDictionaryKey: string];
        !value.is_null()
    }
}

pub fn current_location() -> Result<super::Location, String> {
    Err(
        "location: macOS delivers fixes to CLLocationManager's delegate on the \
         creating thread's run loop (the main thread), so a synchronous read \
         cannot wait for one without stopping the run loop that would deliver \
         it. CLLocationManager.location is nil until a delegate has been \
         called, so there is no cached-fix shortcut either. Use \
         CLLocationManager's delegate from the app's AppKit layer; \
         authorization_status() and is_location_enabled() do work here."
            .into(),
    )
}
