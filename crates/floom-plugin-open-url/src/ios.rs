//! iOS URL opening via `UIApplication openURL:options:completionHandler:`.
//!
//! The modern API — the deprecated `openURL:` returns the same `BOOL` but
//! cannot report *why* a launch failed, and the options flavour can. The
//! completion handler is passed as a null block: the `BOOL` return is the
//! answer this crate reports, and a completion block here would be a second
//! answer for the same question.
//!
//! `sharedApplication` is null during app startup before the delegate is
//! live; an arm called that early reports exactly that rather than sending
//! messages to null.

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};
use std::ffi::c_void;

#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {}

pub fn open_url(url: &str) -> Result<(), String> {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if app.is_null() {
            return Err(
                "open_url: UIApplication is not available yet (the app is still starting up)"
                    .into(),
            );
        }

        // `+[NSURL URLWithString:]` returns null for a string it cannot parse.
        let c = std::ffi::CString::new(url)
            .map_err(|_| format!("open_url: the URL contains a NUL byte: {url}"))?;
        let ns_string: *mut AnyObject =
            msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
        let ns_url: *mut AnyObject = msg_send![class!(NSURL), URLWithString: ns_string];
        if ns_url.is_null() {
            return Err(format!(
                "open_url: the system could not parse `{url}` as a URL"
            ));
        }

        // An empty options dictionary — "no options" — and a null completion
        // block, which the API documents as allowed.
        let options: *mut AnyObject = msg_send![class!(NSDictionary), dictionary];
        let ok: bool = msg_send![
            app,
            openURL: ns_url,
            options: options,
            completionHandler: std::ptr::null_mut::<c_void>()
        ];
        if ok {
            Ok(())
        } else {
            // The system declined: no handler for the scheme, or the user's
            // restrictions forbid it. Which of the two is knowable only
            // through the completion handler's error, which this crate
            // deliberately does not install — see the module doc.
            Err(format!(
                "open_url: the system declined to open `{url}` — no app handles the scheme, \
                 or restrictions forbid it"
            ))
        }
    }
}
