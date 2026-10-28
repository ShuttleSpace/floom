//! A **web view**, contributed by an Objective-C plugin, inside a real window.
//!
//! The plugin's native half is Objective-C and reaches AppKit and WebKit. This
//! example is the whole chain, end to end:
//!
//! 1. a real `NSWindow` is made;
//! 2. the plugin's provider is handed that window;
//! 3. the framework drives the provider — bind, place, destroy;
//! 4. the Objective-C half makes a real `WKWebView`, puts it in the window's
//!    content view, and places it;
//! 5. the example asks the native half to load a URL, and checks that the
//!    object it built really is a `WKWebView`.
//!
//! It also checks the *other* half of the claim: a kind this plugin cannot
//! build is refused, and the refusal names the kind. That is the difference
//! between a plugin that answers "web" and one that answers "anything" with a
//! blank rectangle.
//!
//! It is an example rather than a test for one reason: **AppKit wants the
//! main thread**, and a test harness does not give one. A `cargo test` runs
//! each test on a spawned thread, where `MainThreadMarker::new()` is `None`
//! and an `NSWindow` cannot be made.
//!
//! Run it with:
//!
//! ```text
//! cargo run -p floom-plugin-nativeview --example native_view
//! ```
//!
//! The window **is** shown, and it stays up for a few seconds so the web view
//! can be seen. The page it loads is a `data:` URL, so the example still works
//! on a machine with no network: what is being demonstrated is that a real
//! `WKWebView` was created, placed and painted, not that a site answered.
//!
//! The two `refused` lines in the output are assertions that passed, not
//! errors: the example proves that a kind this plugin cannot build is turned
//! away with a reason that names it.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow,
    NSWindowStyleMask,
};
use objc2_foundation::{NSDate, NSPoint, NSRect, NSRunLoop, NSSize};

use floom_native_ffi::floom_native::{
    NativeCompositionMode, NativeState, NativeViewController, NativeViewId, NativeViewKind, Rect,
};

use floom_plugin_nativeview::{diagnostics, load_url, provider};

fn main() {
    // The main thread: the example's `main` is the process's main thread, so
    // AppKit's requirement is met.
    let mtm = MainThreadMarker::new().expect("an example runs on the main thread");

    // A real window. It is never shown — the point is the composition.
    //
    // A real window, shown on screen. A titled, closable window rather than a
    // borderless one so the app can be moved and quit the way any other window
    // is — this example is meant to be looked at.
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    let rect = NSRect::new(NSPoint::new(200.0, 200.0), NSSize::new(480.0, 360.0));
    let window: Retained<NSWindow> = unsafe {
        let this = mtm.alloc::<NSWindow>();
        msg_send![this,
            initWithContentRect: rect,
            styleMask: NSWindowStyleMask::Titled
                | NSWindowStyleMask::Closable
                | NSWindowStyleMask::Resizable,
            backing: NSBackingStoreType::Buffered,
            defer: false]
    };
    window.setTitle(&objc2_foundation::NSString::from_str(
        "floom nativeview — a real WKWebView from an Objective-C plugin",
    ));
    window.makeKeyAndOrderFront(None);
    app.activate();
    println!("window: {window:?}");

    // The plugin's provider, with the window as its host.
    let host = Retained::as_ptr(&window) as *const c_void;
    // SAFETY: `window` is a live `NSWindow` for the rest of `main`, and the
    // provider built from it is driven only while it is still alive.
    let provider = unsafe { provider(host) }.expect("the plugin registered a provider");

    let mut controller = NativeViewController::boxed(provider);
    let caps = controller.capabilities();
    println!(
        "capabilities: modes={:?} snapshot={} focusable={} input={:?}",
        caps.modes.as_ref(),
        caps.snapshot,
        caps.focusable,
        caps.input
    );
    // A web view takes the pointer and can be focused: it is a browser, and a
    // browser that cannot be clicked or typed into is a screenshot.
    assert!(caps.focusable, "a web view has text fields");
    assert!(
        caps.input
            .supports(floom_native_ffi::floom_native::InputMode::Capture),
        "a web view takes clicks — links and scrolling need them"
    );

    let slot_of = |index: u32| floom_native_ffi::floom_native::NativeSlot {
        id: NativeViewId::new(index, 0),
        generation: 0,
    };

    // --- the kind this plugin cannot build is refused ---
    //
    // Asked *first*, so the refusal cannot be confused with a failure caused
    // by something having been created already.
    let camera_kind = NativeViewKind::Custom("example.camera");
    let refusal = controller
        .bind(slot_of(0), camera_kind, &[NativeCompositionMode::Overlay])
        .expect_err("this plugin builds web views only");
    let reason = format!("{refusal}");
    println!("camera kind refused: {reason}");
    assert!(
        reason.contains("example.camera"),
        "the refusal names the kind that was asked for, got: {reason}"
    );
    assert_eq!(
        diagnostics::refused(),
        1,
        "the native half counted the refusal"
    );
    assert_eq!(
        diagnostics::live(),
        0,
        "a refused kind creates nothing — no half-built view is left behind"
    );

    // --- the kind it can build ---
    let slot = slot_of(1);
    let mode = controller
        .bind(slot, NativeViewKind::Web, &[NativeCompositionMode::Overlay])
        .expect("a web view is what this plugin is for");
    println!("bound web participant in mode {mode:?}");
    assert_eq!(diagnostics::created(), 1, "the native half made one view");
    assert!(
        diagnostics::slot_is_web(slot),
        "the object it built is a WKWebView, not a container standing in for one"
    );
    assert!(
        diagnostics::slot_is_attached(slot),
        "the view is in the window's content view"
    );

    // Place it. The rect crosses in whole **device** pixels, and AppKit's
    // frames are in **points**, so the native half divides by the window's
    // backing scale. On a Retina display that is 2, and the numbers halve —
    // which is the whole chain in one line: logical pixels snap to device
    // pixels once, on the framework's side, and become points once, on the
    // native side.
    let scale: f64 = unsafe { msg_send![&*window, backingScaleFactor] };
    println!("backing scale: {scale}");
    let state = NativeState {
        rect: Rect {
            x0: 20.0,
            y0: 30.0,
            x1: 220.0,
            y1: 130.0,
        },
        clip: None,
        scale: 1.0,
        visible: true,
        z: 0,
        input: floom_native_ffi::floom_native::InputMode::Capture,
        mode: NativeCompositionMode::Overlay,
    };
    controller.apply(slot, state).expect("placed");
    let (x, y, w, h) = diagnostics::last_frame();
    println!("placed at {x},{y} {w}x{h} (points)");
    assert_eq!(
        (x, y, w, h),
        (20.0 / scale, 30.0 / scale, 200.0 / scale, 100.0 / scale),
        "the device pixels became points, divided by the backing scale exactly once"
    );

    // --- load a page ---
    //
    // A `data:` URL rather than a site: the example must pass on a machine
    // with no network, and what is being shown is the web view rendering
    // content of its own, not a particular server answering.
    let page = "data:text/html;charset=utf-8,\
        <html><body style='font-family:-apple-system;background:%23222;color:%23eee;\
        margin:0;display:flex;align-items:center;justify-content:center;height:100vh'>\
        <div style='text-align:center'>\
        <h1 style='margin:0 0 8px'>WKWebView</h1>\
        <p style='margin:0;opacity:.7'>created by the nativeview plugin</p>\
        </div></body></html>";
    load_url(slot, page).expect("WebKit accepted the request");
    println!("loaded an inline data: page");

    // A malformed URL is refused by the plugin's own ABI rather than handed to
    // WebKit to fail silently.
    let bad = load_url(slot, "not a url");
    println!("malformed url refused: {bad:?}");
    assert!(bad.is_err(), "a url that does not parse is refused");

    // --- a second participant, of the same kind ---
    //
    // The case one global view could not serve: two live web views at once.
    let second = slot_of(2);
    controller
        .bind(
            second,
            NativeViewKind::Web,
            &[NativeCompositionMode::Overlay],
        )
        .expect("a second web view");
    assert_eq!(diagnostics::live(), 2, "both participants are alive");
    assert!(
        diagnostics::slot_is_web(second) && diagnostics::slot_is_web(slot),
        "the second create did not steal the first one's view"
    );

    // --- teardown ---
    controller.destroy(second).expect("destroyed");
    controller.destroy(slot).expect("destroyed");
    assert_eq!(diagnostics::destroyed(), 2, "both were torn down");
    assert_eq!(diagnostics::live(), 0, "nothing is left behind");
    assert!(
        !diagnostics::slot_is_attached(slot),
        "the views are out of the window"
    );

    // Put a web view back on screen so the run ends with something to look at
    // rather than an empty window, then hold the window up for a few seconds.
    let shown = slot_of(3);
    controller
        .bind(shown, NativeViewKind::Web, &[NativeCompositionMode::Overlay])
        .expect("a web view to leave on screen");
    let state = NativeState {
        rect: Rect {
            x0: 40.0,
            y0: 40.0,
            x1: 440.0,
            y1: 340.0,
        },
        clip: None,
        scale: 1.0,
        visible: true,
        z: 0,
        input: floom_native_ffi::floom_native::InputMode::Capture,
        mode: NativeCompositionMode::Overlay,
    };
    controller.apply(shown, state).expect("placed");
    load_url(shown, page).expect("loaded");

    println!("\nthe native half did all of it: a real web view created, placed, loaded, and destroyed,");
    println!("and a kind it cannot build refused with a reason that names it.");
    println!("the window stays up for 10 more seconds — close it to quit early.");

    let deadline = NSDate::dateWithTimeIntervalSinceNow(10.0);
    NSRunLoop::mainRunLoop().runUntilDate(&deadline);
}
