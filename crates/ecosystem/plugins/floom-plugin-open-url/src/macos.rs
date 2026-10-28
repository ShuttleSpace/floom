//! macOS URL opening via `NSWorkspace openURL:`.
//!
//! `NSWorkspace` is AppKit's routing layer: it resolves the URL's scheme to
//! the user's default handler and returns a `BOOL` for "the request was
//! accepted" — which is exactly the contract this crate reports. Unlike iOS's
//! `UIApplication`, `sharedWorkspace` is always live, so there is no
//! startup-window case to refuse.

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {}

pub fn open_url(url: &str) -> Result<(), String> {
    unsafe {
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
        let workspace: *mut AnyObject = msg_send![class!(NSWorkspace), sharedWorkspace];
        let ok: bool = msg_send![workspace, openURL: ns_url];
        if ok {
            Ok(())
        } else {
            Err(format!(
                "open_url: the workspace found no application for `{url}`"
            ))
        }
    }
}
