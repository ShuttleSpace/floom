//! Custom `UIApplicationDelegate` that forwards shared files/URLs to floom,
//! and adopts the UIKit scene lifecycle.
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
//! forwards the `NSURL` from both paths through a single C callback that floom
//! registers via `set_on_open_url`. This closes floom issue #109 (cold-start
//! share drop) and also covers the warm-start path.
//!
//! The delegate additionally adopts the **scene lifecycle**, which is mandatory
//! for apps linked against the iOS 26 SDK: UIKit creates a scene session at
//! launch and, if nothing adopts it, traps in
//! `___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption` before any
//! window can appear. The delegate answers
//! `application:configurationForConnectingSceneSession:options:` with a
//! configuration whose delegate is [`SceneDelegate`]; that scene delegate just
//! records the connected `UIWindowScene` in [`WINDOW_SCENE`], and `Window::new`
//! attaches it to the `WinitUIWindow` before making it key and visible
//! (windows created before the session connects are remembered in
//! [`EARLY_WINDOWS`] and receive the scene from the scene delegate instead).
//! All other lifecycle handling keeps using the classic
//! `UIApplication*Notification` observers in `event_loop.rs` — those still fire
//! alongside scenes.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::sync::Mutex;

use dispatch2::MainThreadBound;
use objc2::define_class;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol};
use objc2::{ClassType, DefinedClass, MainThreadOnly};
use objc2_foundation::{NSDictionary, NSObject, NSString, NSURL};
use objc2_ui_kit::{
    UIApplication, UIApplicationDelegate, UIApplicationLaunchOptionsKey,
    UIApplicationLaunchOptionsURLKey, UIApplicationOpenURLOptionsKey, UIScene, UISceneConfiguration,
    UISceneConnectionOptions, UISceneDelegate, UISceneSession, UIWindowScene,
};

// floom registers a single `unsafe extern "C" fn(*mut c_void)` callback. The
// `c_void` pointer is the borrowed `NSURL`; floom's trampoline reads it as
// `&NSURL`.
type OnOpenUrl = unsafe extern "C" fn(*mut c_void);

static ON_OPEN_URL: Mutex<Option<OnOpenUrl>> = Mutex::new(None);

/// The `UIWindowScene` UIKit connected for our (single) scene session.
///
/// Filled by [`SceneDelegate::scene_will_connect`] and cleared again when the
/// scene disconnects. NOTE: the first window of the app is typically created
/// before this is filled (the `Resumed` notification fires before scene
/// connection on iOS 26+); such windows are remembered in [`EARLY_WINDOWS`]
/// and receive the scene from the scene delegate instead.
// SAFETY: Creating `MainThreadBound` in a `const` context,
// where there is no concept of the main thread.
static WINDOW_SCENE: MainThreadBound<Cell<Option<Retained<UIWindowScene>>>> =
    MainThreadBound::new(Cell::new(None), unsafe { MainThreadMarker::new_unchecked() });

/// Windows created before the scene session connected.
///
/// On iOS 26+ the launch ordering delivers the winit `Resumed` notification
/// (where `Window::new` runs) BEFORE `scene:willConnectToSession:`, and a
/// window that is not part of a scene is invisible — worse, it is not even
/// enumerated by `UIApplication.windows()` (that API only reports
/// scene-attached windows under the mandatory scene lifecycle), so the only
/// way to find those early windows later is to remember them ourselves.
// SAFETY: Creating `MainThreadBound` in a `const` context,
// where there is no concept of the main thread.
static EARLY_WINDOWS: MainThreadBound<RefCell<Vec<Retained<crate::window::WinitUIWindow>>>> =
    MainThreadBound::new(RefCell::new(Vec::new()), unsafe { MainThreadMarker::new_unchecked() });

/// Record a window that was created before the scene session connected, so
/// [`SceneDelegate::scene_will_connect`] can attach the scene to it.
pub(crate) fn register_early_window(window: Retained<crate::window::WinitUIWindow>) {
    let mtm = MainThreadMarker::new().expect("windows are created on the main thread");
    EARLY_WINDOWS.get(mtm).borrow_mut().push(window);
}

/// The `UIWindowScene` of the connected scene session, if any.
///
/// `Window::new` attaches this to the `WinitUIWindow` so the window
/// participates in the (mandatory) scene session.
pub(crate) fn current_window_scene(mtm: MainThreadMarker) -> Option<Retained<UIWindowScene>> {
    // `Cell<Option<T>>` is not `Copy` for `Retained<T>`, so take-and-restore.
    let scene = WINDOW_SCENE.get(mtm).take();
    if let Some(scene) = &scene {
        WINDOW_SCENE.get(mtm).set(Some(scene.clone()));
    }
    scene
}

/// Register (or clear with `None`) the callback floom receives shared URLs
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
        // it is safe to hand a borrowed pointer that floom reads synchronously.
        unsafe { callback(url as *const NSURL as *mut c_void) };
    }
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FloomWinitApplicationDelegate"]
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

        // Scene lifecycle adoption (mandatory for the iOS 26+ SDK). Returning
        // a configuration with a delegate class here is what stops UIKit from
        // trapping in `NoSceneLifecycleAdoption` at launch. The return type is
        // an object, so the selector is registered with `method_id` (objc2's
        // `method` form only accepts `EncodeReturn` values).
        #[unsafe(method_id(application:configurationForConnectingSceneSession:options:))]
        fn configuration_for_connecting_scene_session(
            &self,
            _application: &UIApplication,
            connecting_scene_session: &UISceneSession,
            _options: &UISceneConnectionOptions,
        ) -> Retained<UISceneConfiguration> {
            let mtm = MainThreadMarker::new()
                .expect("scene session callbacks arrive on the main thread");
            let configuration = UISceneConfiguration::configurationWithName_sessionRole(
                None,
                &connecting_scene_session.role(),
                mtm,
            );
            // SAFETY: plain setters on a live, main-thread UIKit object.
            unsafe {
                configuration.setDelegateClass(Some(SceneDelegate::class()));
                configuration.setSceneClass(Some(UIWindowScene::class()));
            }
            configuration
        }
    }
);

/// Minimal `UISceneDelegate` that records the connected `UIWindowScene`.
///
/// Everything else continues to flow through the classic
/// `UIApplication*Notification` observers (they still fire alongside scenes),
/// so this delegate exists purely for the adoption check and to hand
/// `Window::new` the scene to attach to.
define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "FloomWinitSceneDelegate"]
    pub(crate) struct SceneDelegate;

    unsafe impl NSObjectProtocol for SceneDelegate {}

    unsafe impl UISceneDelegate for SceneDelegate {
        #[unsafe(method(scene:willConnectToSession:options:))]
        fn scene_will_connect(
            &self,
            scene: &UIScene,
            _session: &UISceneSession,
            _options: &UISceneConnectionOptions,
        ) {
            let mtm = MainThreadMarker::new()
                .expect("scene callbacks arrive on the main thread");
            // `Retained::retain` gives a `Retained<UIScene>` we can downcast
            // into the `UIWindowScene` our configuration requested.
            // SAFETY: `scene` is alive for the duration of the selector call
            // and the configuration forces `UIWindowScene`, so the downcast
            // always succeeds; the retained value is dropped on the main
            // thread when the scene disconnects or the app exits.
            let scene = unsafe { Retained::retain(scene as *const UIScene as *mut UIScene) };
            if let Some(scene) = scene {
                match scene.downcast::<UIWindowScene>() {
                    Ok(window_scene) => {
                        WINDOW_SCENE.get(mtm).set(Some(window_scene.clone()));
                        // The launch notification ordering on iOS 26+ runs our
                        // window creation (driven by the classic
                        // `UIApplication*Notification` observers) BEFORE
                        // `scene:willConnectToSession:options:`, so any window
                        // created that early is still scene-less and would
                        // never appear (it is not even enumerated by
                        // `UIApplication.windows()`). Attach the freshly
                        // connected scene to every window we remembered in
                        // [`EARLY_WINDOWS`] instead.
                        let early_windows = std::mem::take(&mut *EARLY_WINDOWS.get(mtm).borrow_mut());
                        for window in early_windows {
                            // SAFETY: plain setters on live, main-thread UIKit
                            // objects.
                            unsafe {
                                window.setWindowScene(Some(&window_scene));
                                window.makeKeyAndVisible();
                            }
                        }
                    }
                    // The configuration forces `UIWindowScene`, so the downcast
                    // cannot fail; drop the impossible error value silently.
                    Err(scene) => {
                        let _ = scene;
                    }
                }
            }
        }

        #[unsafe(method(sceneDidDisconnect:))]
        fn scene_did_disconnect(&self, _scene: &UIScene) {
            let mtm = MainThreadMarker::new()
                .expect("scene callbacks arrive on the main thread");
            WINDOW_SCENE.get(mtm).set(None);
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
