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

use floom_native_ffi::floom_native::{
    NativeCompositionMode, NativeError, NativeViewController, NativeViewId, NativeViewKind,
    NativeViewProvider,
};

use floom_plugin_webview::{diagnostics, provider};

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
        floom_plugin_webview::version().starts_with("webview "),
        "the version comes from the native half: {:?}",
        floom_plugin_webview::version()
    );
    floom_plugin_webview::enable().expect("the native half enables");
    floom_plugin_webview::disable();
    assert_eq!(floom_plugin_webview::capability(), 0);
}

#[test]
fn the_provider_is_registered_by_the_native_half() {
    // The Objective-C half's constructor hands its vtable to the Rust half.
    // Without it there is no provider, and the error would say so rather than
    // handing back a null pointer.
    //
    // SAFETY: no window is passed on purpose. The provider is only asked to
    // exist here; nothing drives it, so nothing dereferences the handle.
    unsafe { provider(std::ptr::null()) }.expect("the native half registered a provider");
}

/// A web view cannot be created off the main thread, so this test cannot make
/// one.
///
/// It used to: a plain `NSView` is forgiving, and `[[NSView alloc] init]` on a
/// spawned thread happens to work. `WKWebView` is not — WebKit asserts the
/// main thread and the process takes a `SIGTRAP`, which is exactly what
/// happened when this test was first pointed at a web view: it vanished from
/// the output rather than failing. That is worth spelling out instead of
/// deleting, because a test that silently disappears is worse than one that
/// says why it did not run.
///
/// What is left here is the part that genuinely needs neither a window nor the
/// main thread: the capabilities the provider publishes, and its refusal path.
/// Creation, placement, loading and teardown are covered by
/// `examples/native_view.rs`, which runs on the main thread like an app does.
#[test]
fn capabilities_and_refusals_need_neither_a_window_nor_the_main_thread() {
    // SAFETY: no host window is passed on purpose. Neither call below
    // dereferences one: `capabilities` reads a constant, and the refusal path
    // returns before touching the window.
    let mut controller =
        NativeViewController::boxed(unsafe { provider(std::ptr::null()) }.expect("provider"));

    let caps = controller.capabilities();
    assert_eq!(
        caps.modes.as_ref(),
        &[NativeCompositionMode::Overlay],
        "the native half published overlay, in its own order"
    );
    assert!(
        !caps.snapshot,
        "a web view cannot be frozen into a texture on this backend"
    );
    assert!(
        caps.focusable,
        "a web view has text fields, so it joins the focus chain"
    );
    assert!(
        caps.input
            .supports(floom_native_ffi::floom_native::InputMode::Capture),
        "a web view takes clicks — links and scrolling need them"
    );

    // The kind this plugin cannot build is refused, and nothing is created:
    // if the refusal ran after a create, a half-built participant would leak.
    let before_created = diagnostics::created();
    let camera = slot(9, 0);
    let error = controller
        .bind(
            camera,
            NativeViewKind::Custom("example.camera"),
            &[NativeCompositionMode::Overlay],
        )
        .expect_err("this plugin builds web views only");
    let reason = format!("{error}");
    assert!(
        reason.contains("example.camera"),
        "the refusal names the kind that was asked for, got: {reason}"
    );
    assert_eq!(
        diagnostics::created(),
        before_created,
        "a refused kind creates nothing"
    );
}

/// The creation path, which needs the main thread and is therefore skipped.
///
/// Kept as a named test rather than removed so the skip is visible in the
/// output: `cargo test` prints the reason, and a reader who wonders where the
/// pixel-level coverage went finds the answer instead of a gap.
#[test]
fn a_web_view_is_created_on_the_main_thread_only() {
    eprintln!(
        "SKIP: a WKWebView must be created on the main thread; a `cargo test` thread is not \
         it and WebKit traps the process. Covered by \
         `cargo run -p floom-plugin-webview --example native_view`, which runs on the \
         main thread."
    );
}

#[test]
fn the_controller_refuses_a_mode_the_provider_does_not_offer() {
    // SAFETY: as above — no host window, and the provider is only asked to
    // refuse a mode, which never touches one.
    let mut controller =
        NativeViewController::boxed(unsafe { provider(std::ptr::null()) }.expect("provider"));
    let bound = slot(3, 0);
    // The native half offers overlay only. The kind is `Web` on purpose: this
    // test is about the *mode* being refused, and a kind this plugin cannot
    // build would be refused earlier, for a different reason.
    let error = controller
        .bind(
            bound,
            NativeViewKind::Web,
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
    // SAFETY: a null handle is accepted here on purpose. This test only asks
    // the provider for its capabilities, which never touch the host window;
    // the native half is stubbed out in the host build, so there is no window
    // to pass and nothing that would dereference one.
    let provider: Box<dyn NativeViewProvider> =
        unsafe { floom_plugin_webview::boxed_provider(std::ptr::null()) }.expect("provider");
    // Focusable, because a web view has text fields and a form the user
    // cannot type into is not an embedded browser.
    assert!(
        provider.capabilities().focusable,
        "a web view joins the focus chain"
    );
    // It takes the pointer too — otherwise links and scrolling would not work.
    assert!(
        provider
            .capabilities()
            .input
            .supports(floom_native_ffi::floom_native::InputMode::Capture),
        "a web view captures input"
    );
}
