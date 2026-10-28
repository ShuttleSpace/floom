//! iOS screen-awake via `UIApplication.idleTimerDisabled`.
//!
//! The property is the platform's whole surface: set `YES` and the system
//! stops auto-locking, set `NO` and it resumes. Reading it back is a plain
//! getter, so `is_keep_screen_on` reports the application's actual state.
//!
//! `sharedApplication` is null during app startup before the delegate is
//! live; an arm called that early reports exactly that.

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {}

pub fn keep_screen_on(on: bool) -> Result<(), String> {
    unsafe {
        let app = shared_application()?;
        let _: () = msg_send![app, setIdleTimerDisabled: on];
        Ok(())
    }
}

pub fn is_keep_screen_on() -> bool {
    unsafe {
        let Ok(app) = shared_application() else {
            return false;
        };
        let disabled: bool = msg_send![app, isIdleTimerDisabled];
        disabled
    }
}

unsafe fn shared_application() -> Result<*mut AnyObject, String> {
    let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
    if app.is_null() {
        Err(
            "keep_screen_on: UIApplication is not available yet (the app is still starting up)"
                .into(),
        )
    } else {
        Ok(app)
    }
}
