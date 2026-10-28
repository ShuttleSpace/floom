//! iOS share via `UIActivityViewController`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # The sheet is presented, not run
//!
//! `presentViewController:animated:completion:` returns as soon as the
//! presentation is *scheduled*; the user's choice arrives later, through a
//! completion handler this crate does not take. That is the whole contract of
//! `share_text` / `share_files`: return `Ok` when the sheet was handed to UIKit,
//! and report an `Err` only for the things that stop it from being handed over
//! (no root view controller, a missing file, a non-main thread).
//!
//! # Why the main-thread check
//!
//! UIKit is main-thread-only, and presenting off it is undefined behaviour, not
//! an error UIKit returns. Floom's iOS event loop runs on the main thread, so
//! the normal path is the main thread; the check exists so that a plugin called
//! from a worker reports that plainly instead of corrupting the view hierarchy.
//!
//! # Why the popover anchor is set
//!
//! On iPad the activity sheet is a popover, and UIKit raises
//! `NSInternalInconsistencyException` if `popoverPresentationController` has no
//! `sourceView`/`sourceRect`. Setting the anchor unconditionally is harmless on
//! iPhone (the controller is `nil` there, and messaging `nil` is a no-op).

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

// `UIActivityViewController` and `UIApplication` are UIKit classes reached
// through `objc_getClass`, which only finds them if UIKit is loaded.
#[link(name = "UIKit", kind = "framework")]
unsafe extern "C" {}

pub fn share_text(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Err("share_text: the text to share is empty".into());
    }
    unsafe {
        let string = ns_string(text)?;
        // `+[NSArray arrayWithObject:]` keeps the item list to one element, so
        // no mutable array is needed.
        let items: *mut AnyObject = msg_send![class!(NSArray), arrayWithObject: string];
        present(items)
    }
}

pub fn share_files(paths: &[&str]) -> Result<(), String> {
    if paths.is_empty() {
        return Err("share_files: no paths given".into());
    }
    unsafe {
        let urls: *mut AnyObject = msg_send![class!(NSMutableArray), array];
        if urls.is_null() {
            return Err("share_files: could not create the item array".into());
        }
        for path in paths {
            // A missing path would otherwise reach the sheet as a URL that
            // UIKit renders as a file icon with no name; naming it here points
            // at the caller's bug.
            if !std::path::Path::new(path).exists() {
                return Err(format!("share_files: no such file: {path}"));
            }
            let url = file_url(path)?;
            let _: () = msg_send![urls, addObject: url];
        }
        present(urls)
    }
}

/// Hand the item array to a `UIActivityViewController` and present it.
///
/// # Safety
///
/// `items` must be a live `NSArray` of `NSString` / `NSURL` objects, which is
/// what both callers build.
unsafe fn present(items: *mut AnyObject) -> Result<(), String> {
    if items.is_null() {
        return Err("share: could not build the item list".into());
    }
    if !is_main_thread() {
        return Err(
            "share: UIActivityViewController must be presented on the main thread; \
             call this from the UI thread"
                .into(),
        );
    }
    let root = root_view_controller()
        .ok_or_else(|| "share: no root view controller to present from".to_string())?;

    let controller: *mut AnyObject = msg_send![
        class!(UIActivityViewController),
        initWithActivityItems: items,
        applicationActivities: std::ptr::null_mut::<AnyObject>()
    ];
    if controller.is_null() {
        return Err("share: could not create UIActivityViewController".into());
    }

    // The popover anchor, for iPad. `popoverPresentationController` is `nil` on
    // iPhone, and a message to `nil` returns `nil`, so this block is a no-op
    // there.
    //
    // Only `sourceView` is set, not `sourceRect`: a zero `CGRect` is not a
    // struct `msg_send!` can encode without a CoreGraphics binding, and
    // `sourceView` alone satisfies the documented requirement — UIKit centres
    // the popover on the view and derives the rect from its bounds.
    let popover: *mut AnyObject = msg_send![controller, popoverPresentationController];
    if !popover.is_null() {
        let view: *mut AnyObject = msg_send![root, view];
        let _: () = msg_send![popover, setSourceView: view];
    }

    let _: () = msg_send![
        root,
        presentViewController: controller,
        animated: true,
        completion: std::ptr::null_mut::<AnyObject>()
    ];
    Ok(())
}

/// `UIApplication.sharedApplication.keyWindow.rootViewController`, walking down
/// to the topmost presented controller.
///
/// The presented chain matters: presenting from a controller that is already
/// presenting something else is ignored by UIKit with a console warning, and the
/// share sheet would never appear.
fn root_view_controller() -> Option<*mut AnyObject> {
    unsafe {
        let application: *mut AnyObject = msg_send![class!(UIApplication), sharedApplication];
        if application.is_null() {
            return None;
        }
        let window: *mut AnyObject = msg_send![application, keyWindow];
        if window.is_null() {
            return None;
        }
        let mut controller: *mut AnyObject = msg_send![window, rootViewController];
        if controller.is_null() {
            return None;
        }
        // `presentedViewController` is the next one up; the loop stops when it
        // is nil.
        loop {
            let presented: *mut AnyObject = msg_send![controller, presentedViewController];
            if presented.is_null() {
                return Some(controller);
            }
            controller = presented;
        }
    }
}

/// `+[NSThread isMainThread]`.
fn is_main_thread() -> bool {
    unsafe {
        let main: bool = msg_send![class!(NSThread), isMainThread];
        main
    }
}

/// `+[NSString stringWithUTF8String:]`, which takes a C string rather than an
/// `NSString` — so no Foundation binding is needed to build one.
unsafe fn ns_string(value: &str) -> Result<*mut AnyObject, String> {
    let c = std::ffi::CString::new(value)
        .map_err(|_| "share_text: the text contains a NUL byte".to_string())?;
    let string: *mut AnyObject = msg_send![class!(NSString), stringWithUTF8String: c.as_ptr()];
    if string.is_null() {
        return Err("share_text: could not create an NSString".into());
    }
    Ok(string)
}

/// `+[NSURL fileURLWithPath:]`.
unsafe fn file_url(path: &str) -> Result<*mut AnyObject, String> {
    let c = std::ffi::CString::new(path)
        .map_err(|_| format!("share_files: the path contains a NUL byte: {path}"))?;
    let url: *mut AnyObject = msg_send![class!(NSURL), fileURLWithPath: c.as_ptr()];
    if url.is_null() {
        return Err(format!("share_files: could not build a URL for {path}"));
    }
    Ok(url)
}
