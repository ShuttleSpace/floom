//! A native view, contributed by an Objective-C plugin, inside a real window.
//!
//! The plugin's native half is Objective-C and reaches AppKit. This example
//! is the whole chain, end to end:
//!
//! 1. a real `NSWindow` is made;
//! 2. the plugin's provider is handed that window;
//! 3. the framework drives the provider — bind, place, destroy;
//! 4. the Objective-C half makes a real `NSView`, puts it in the window's
//!    content view, and places it;
//! 5. the example checks that the view really is in the window.
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
//! The window is never shown: the point is the composition, not the pixels.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::{MainThreadMarker, msg_send};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};

use floom_native_ffi::floom_native::{
    NativeCompositionMode, NativeState, NativeViewController, NativeViewId, NativeViewKind, Rect,
};

use floom_plugin_nativeview::{diagnostics, provider};

fn main() {
    // The main thread: the example's `main` is the process's main thread, so
    // AppKit's requirement is met.
    let mtm = MainThreadMarker::new().expect("an example runs on the main thread");

    // A real window. It is never shown — the point is the composition.
    let rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(400.0, 300.0));
    let window: Retained<NSWindow> = unsafe {
        let this = mtm.alloc::<NSWindow>();
        let window: Retained<NSWindow> = msg_send![this,
            initWithContentRect: rect,
            styleMask: NSWindowStyleMask(0),
            backing: NSBackingStoreType::Buffered,
            defer: false];
        window
    };
    println!("window: {:?}", window);

    // The plugin's provider, with the window as its host.
    let host = Retained::as_ptr(&window) as *const c_void;
    let provider = provider(host).expect("the plugin registered a provider");

    let mut controller = NativeViewController::boxed(provider);
    let caps = controller.capabilities();
    println!(
        "capabilities: modes={:?} snapshot={} focusable={}",
        caps.modes.as_ref(),
        caps.snapshot,
        caps.focusable
    );

    // Bind a participant: the native half creates its view.
    let slot = floom_native_ffi::floom_native::NativeSlot {
        id: NativeViewId::new(1, 0),
        generation: 0,
    };
    let mode = controller
        .bind(
            slot,
            NativeViewKind::Custom("example.nativeview"),
            &[NativeCompositionMode::Overlay],
        )
        .expect("the native half offers overlay");
    println!("bound in mode {mode:?}");
    assert_eq!(diagnostics::created(), 1, "the native half made one view");
    assert!(
        diagnostics::view_is_attached(),
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
        input: floom_native_ffi::floom_native::InputMode::Passthrough,
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

    // Tear it down: the native half takes the view out of the window.
    controller.destroy(slot).expect("destroyed");
    assert_eq!(
        diagnostics::destroyed(),
        1,
        "the native half tore the view down"
    );
    assert!(
        !diagnostics::view_is_attached(),
        "the view is out of the window"
    );

    println!("\nthe native half did all of it: one view created, placed, and destroyed.");
}
