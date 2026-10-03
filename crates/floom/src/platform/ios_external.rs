#![cfg(target_os = "ios")]

//! Open external URLs / apps on iOS via `UIApplication.openURL`.
//!
//! iOS has no `open`/`xdg-open` equivalent; opening a URL means asking the
//! shared `UIApplication` to open an `NSURL` with the system handler (browser
//! for `http(s)://`, the matching app for other schemes). This mirrors
//! Android's `ACTION_VIEW` bridge (`android_external.rs`) but uses `objc2`
//! instead of JNI.

use objc2_foundation::{MainThreadMarker, NSString, NSURL};
use objc2_ui_kit::UIApplication;

/// Open `url` in the default external handler (browser for `http(s)://`,
/// appropriate app for other schemes).
///
/// Returns an error string on failure (e.g. the URL cannot be parsed, no app
/// handles the scheme, or we are not on the main thread).
// `openURL:` is deprecated in favor of `openURL:options:completionHandler:`,
// but the latter requires the `block2` feature plus a completion block. The
// deprecated form is still fully functional and is the simplest path here.
#[allow(deprecated)]
pub(crate) fn open_url(url: &str) -> Result<(), String> {
    let mtm = match MainThreadMarker::new() {
        Some(mtm) => mtm,
        None => return Err("UIApplication.openURL must run on the main thread".to_string()),
    };

    let ns_string = NSString::from_str(url);
    let ns_url = match NSURL::URLWithString(&ns_string) {
        Some(url) => url,
        None => return Err(format!("could not parse URL: {url}")),
    };

    let app = UIApplication::sharedApplication(mtm);
    let opened = unsafe { app.openURL(&ns_url) };

    if opened {
        Ok(())
    } else {
        Err(format!("UIApplication failed to open URL: {url}"))
    }
}
