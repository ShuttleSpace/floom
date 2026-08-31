//! Custom `UIApplicationDelegate` that forwards shared files/URLs to floem.
//!
//! winit 25.10.0 drives `UIApplication::main(None, None, _)` and never installs
//! an application delegate, so it (a) never reads the cold-start launch URL and
//! (b) never receives warm-start `openURL:` calls. iOS delivers a file/URL
//! shared into the app through exactly those two paths:
//!   * cold-start -> `application:didFinishLaunchingWithOptions:` with the URL in
//!     `launchOptions[UIApplicationLaunchOptionsURLKey]`.
//!   * warm-start -> `application:openURL:options:` (app already running).
//!
//! We install a minimal delegate (passed by name to `UIApplication::main`) that
//! forwards the `NSURL` from both paths through a single C callback that floem
//! registers via `set_on_open_url`. This closes floem issue #109 (cold-start
//! share drop) and also covers the warm-start path.

use std::ffi::c_void;
use std::sync::Mutex;

use objc2::define_class;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{ClassType, DefinedClass, MainThreadOnly};
use objc2_foundation::{NSDictionary, NSObject, NSString, NSURL};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey,
    UIApplicationLaunchOptionsURLKey, UIApplicationOpenURLOptionsKey,
};

// floem registers a single `unsafe extern "C" fn(*mut c_void)` callback. The
// `c_void` pointer is the borrowed `NSURL`; floem's trampoline reads it as
// `&NSURL`.
type OnOpenUrl = unsafe extern "C" fn(*mut c_void);

static ON_OPEN_URL: Mutex<Option<OnOpenUrl>> = Mutex::new(None);

/// Register (or clear with `None`) the callback floem receives shared URLs
/// through. Must be called on the main thread, before `EventLoop::run`.
///
/// # Safety
/// `callback` must be a valid `extern "C"` function pointer; it is invoked
/// synchronously on the main thread with a borrowed `NSURL` that is only valid
/// for the duration of the call.
pub unsafe fn set_on_open_url(callback: Option<OnOpenUrl>) {
    *ON_OPEN_URL.lock().unwrap() = callback;
}

fn forward_open_url(url: &NSURL) {
    if let Some(callback) = *ON_OPEN_URL.lock().unwrap() {
        // The NSURL is owned by the caller (UIKit) for the selector's scope, so
        // it is safe to hand a borrowed pointer that floem reads synchronously.
        unsafe { callback(url as *const NSURL as *mut c_void) };
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FloemWinitApplicationDelegate"]
    pub(crate) struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl UIApplicationDelegate for AppDelegate {
        #[unsafe(method(application:didFinishLaunchingWithOptions:))]
        unsafe fn did_finish_launching_with_options(
            &self,
            _application: &UIApplication,
            launch_options: Option<&NSDictionary<UIApplicationLaunchOptionsKey, AnyObject>>,
        ) -> bool {
            if let Some(options) = launch_options {
                // `UIApplicationLaunchOptionsURLKey` is `&'static
                // UIApplicationLaunchOptionsKey`, which matches `&KeyType`.
                if let Some(url) = options.objectForKey(UIApplicationLaunchOptionsURLKey) {
                    if let Some(url) = url.downcast_ref::<NSURL>() {
                        forward_open_url(url);
                    }
                }
            }
            true
        }

        #[unsafe(method(application:openURL:options:))]
        #[allow(deprecated)]
        unsafe fn open_url(
            &self,
            _application: &UIApplication,
            url: &NSURL,
            _options: &NSDictionary<UIApplicationOpenURLOptionsKey, AnyObject>,
        ) -> bool {
            forward_open_url(url);
            true
        }
    }
);

/// The delegate class name to pass to `UIApplication::main`.
///
/// The class is registered lazily on first access (which happens on the main
/// thread, inside `EventLoop::run_app` before `UIApplicationMain` is entered).
pub(crate) fn delegate_class() -> Retained<NSString> {
    NSString::from_class(AppDelegate::class())
}
