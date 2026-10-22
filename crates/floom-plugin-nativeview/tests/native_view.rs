//! The plugin's native half, driven through the framework.
//!
//! The native half is Objective-C and reaches AppKit. This test drives it
//! through the C ABI — the same path a Swift, Java or C++ plugin would take —
//! and checks that the calls arrived and that a real AppKit view was made.
//!
//! What is checked here is the **boundary**: that a call from Rust reached
//! native code, that the arguments crossed intact, and that the native side
//! really did the work. The window itself is a separate matter: AppKit wants
//! the main thread, and a test harness does not give it one, so the window
//! half is exercised by `examples/native_view.rs`, which runs on the main
//! thread like an app does.
//!
//! Only one test here creates a native view, and the reason is the counters:
//! the native half's diagnostics are **process-wide**, so two tests creating
//! views at the same time would each see the other's. One test owns the
//! numbers; the others never create anything.

use std::ffi::c_void;

use floom_native_ffi::floom_native::{
    NativeCompositionMode, NativeError, NativeState, NativeViewController, NativeViewId,
    NativeViewKind, NativeViewProvider, Rect,
};

use floom_plugin_nativeview::{diagnostics, provider};

fn slot(index: u32, generation: u32) -> floom_native_ffi::floom_native::NativeSlot {
    floom_native_ffi::floom_native::NativeSlot {
        id: NativeViewId::new(index, generation),
        generation,
    }
}

#[test]
fn the_plugin_answers_its_own_abi() {
    // The plugin's own four functions, implemented by the Objective-C half.
    assert!(
        floom_plugin_nativeview::version().starts_with("nativeview "),
        "the version comes from the native half: {:?}",
        floom_plugin_nativeview::version()
    );
    floom_plugin_nativeview::enable().expect("the native half enables");
    floom_plugin_nativeview::disable();
    assert_eq!(floom_plugin_nativeview::capability(), 0);
}

#[test]
fn the_provider_is_registered_by_the_native_half() {
    // The Objective-C half's constructor hands its vtable to the Rust half.
    // Without it there is no provider, and the error would say so rather than
    // handing back a null pointer.
    provider(std::ptr::null()).expect("the native half registered a provider");
}

#[test]
fn a_native_view_is_created_placed_and_destroyed() {
    let before_created = diagnostics::created();
    let before_destroyed = diagnostics::destroyed();

    // The provider, with no window: the native half still makes a real
    // AppKit view, which is what "a native half did real work" means.
    let mut controller = NativeViewController::boxed(provider(std::ptr::null()).expect("provider"));

    let caps = controller.capabilities();
    assert_eq!(
        caps.modes.as_ref(),
        &[NativeCompositionMode::Overlay],
        "the native half published overlay, in its own order"
    );
    assert!(
        !caps.snapshot,
        "a plain view cannot be frozen into a texture"
    );

    let bound = slot(1, 0);
    let mode = controller
        .bind(
            bound,
            NativeViewKind::Custom("example.view"),
            &[NativeCompositionMode::Overlay],
        )
        .expect("the native half offers overlay");
    assert_eq!(mode, NativeCompositionMode::Overlay);

    assert_eq!(
        diagnostics::created(),
        before_created + 1,
        "the native half really created a view"
    );

    // The placement crossed in device pixels and became points on the other
    // side: the scale is 1 here, so the numbers are the same.
    let state = NativeState {
        rect: Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 110.0,
            y1: 70.0,
        },
        clip: None,
        scale: 1.0,
        visible: true,
        z: 0,
        input: floom_native_ffi::floom_native::InputMode::Passthrough,
        mode: NativeCompositionMode::Overlay,
    };
    controller.apply(bound, state).expect("applied");
    let (x, y, w, h) = diagnostics::last_frame();
    assert_eq!((x, y, w, h), (10.0, 20.0, 100.0, 50.0));

    // The native half declares that it cannot snapshot, so the controller
    // says so instead of handing back a frozen texture that does not exist.
    let snap = controller
        .snapshot(bound)
        .expect("the snapshot was refused");
    assert!(
        snap.is_none(),
        "a provider that cannot freeze says so rather than returning a texture"
    );

    // Tearing down tears the native view down.
    controller.destroy(bound).expect("destroyed");
    assert_eq!(
        diagnostics::destroyed(),
        before_destroyed + 1,
        "the native half really tore the view down"
    );
    assert!(controller.unreleased().next().is_none());
}

#[test]
fn the_controller_refuses_a_mode_the_provider_does_not_offer() {
    let mut controller = NativeViewController::boxed(provider(std::ptr::null()).expect("provider"));
    let bound = slot(3, 0);
    // The native half offers overlay only.
    let error = controller
        .bind(
            bound,
            NativeViewKind::Video,
            &[NativeCompositionMode::Surface],
        )
        .expect_err("surface was refused");
    assert!(
        matches!(error, NativeError::NoSupportedMode),
        "the refusal is typed: {error:?}"
    );
}

/// The provider is a `NativeViewProvider`, so a host can hold it as one.
#[test]
fn the_provider_is_an_ordinary_native_view_provider() {
    let provider: Box<dyn NativeViewProvider> =
        floom_plugin_nativeview::boxed_provider(std::ptr::null() as *const c_void)
            .expect("provider");
    assert!(provider.capabilities().focusable == false);
}
