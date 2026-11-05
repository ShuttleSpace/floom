// SPDX-License-Identifier: MIT
//! The ABI's calling convention and lifecycle, driven from Rust.
//!
//! The layout check (`c_layout.rs`) proves the header and the Rust mirror
//! describe the same memory. This one proves the *calls* work: a provider
//! implemented behind the C function-pointer table is driven through the
//! ordinary Rust [`NativeViewProvider`] trait and through the controller, and
//! every call is observed.
//!
//! The provider here is written in Rust but reached through the C ABI's
//! shape — the same `extern "C"` function pointers, the same struct, the same
//! calling convention a Swift or Java implementation would use. The
//! difference is only where the code came from.
//!
//! What this proves that the layout check cannot:
//!
//! - the conversion from the framework's logical coordinates to the device
//!   pixels the ABI carries;
//! - the crop, which crosses as device pixels and as a "there is no crop"
//!   flag rather than as an optional rect;
//! - the error channel, where the provider's own message crosses back.

use std::ffi::{CStr, c_char, c_void};
use std::ptr;

use floom_native::{
    ClipSupport, InputModes, NativeCompositionMode, NativeError, NativeState, NativeViewController,
    NativeViewId, NativeViewKind, to_device,
};
use floom_native_ffi::abi::{
    ABI_VERSION, FloomNativeCapabilities, FloomNativeDeviceRect, FloomNativeProviderV1,
    FloomNativeSlot, FloomNativeState, INPUT_CAPTURE, INPUT_PASSTHROUGH, KIND_CUSTOM,
};
use floom_native_ffi::{CProvider, check};

use floom_types::Rect;

/// What the fake provider remembers, so the test can see that the calls
/// arrived and with what.
#[derive(Default)]
struct Recorded {
    created: Vec<(u32, u32, u32, String)>,
    destroyed: Vec<(u32, u32)>,
    applied: Vec<(u32, u32, FloomNativeState)>,
    focused: Vec<(u32, u32)>,
    blurred: Vec<(u32, u32)>,
}

/// The provider's state, as the native side would own it.
struct Provider {
    recorded: Recorded,
    /// Whether `create` refuses.
    refuse: bool,
}

unsafe extern "C" fn attach(_self: *mut c_void, _host_window: *const c_void) -> i32 {
    0
}

unsafe extern "C" fn capabilities(self_: *mut c_void) -> FloomNativeCapabilities {
    let _ = self_;
    FloomNativeCapabilities {
        // Overlay first: the provider's own order of preference.
        modes: [0, 1, 0, 0],
        mode_count: 2,
        snapshot: 0,
        input_passthrough: 1,
        input_capture: 1,
        focusable: 1,
        clip: 1,
        reserved: [0; 3],
    }
}

unsafe extern "C" fn create(
    self_: *mut c_void,
    slot: FloomNativeSlot,
    kind: u32,
    kind_name: *const c_char,
    out_error: *mut c_char,
    out_capacity: u32,
) -> i32 {
    let provider = unsafe { &mut *(self_ as *mut Provider) };
    let name = if kind_name.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(kind_name) }
            .to_string_lossy()
            .into_owned()
    };
    provider
        .recorded
        .created
        .push((slot.index, slot.generation, kind, name));
    if provider.refuse {
        let message = b"the provider refuses on purpose\0";
        let len = message.len().min(out_capacity as usize);
        unsafe {
            ptr::copy_nonoverlapping(message.as_ptr() as *const c_char, out_error, len);
        }
        return 7;
    }
    0
}

unsafe extern "C" fn destroy(self_: *mut c_void, slot: FloomNativeSlot) {
    let provider = unsafe { &mut *(self_ as *mut Provider) };
    provider
        .recorded
        .destroyed
        .push((slot.index, slot.generation));
}

unsafe extern "C" fn apply(
    self_: *mut c_void,
    slot: FloomNativeSlot,
    state: *const FloomNativeState,
) {
    let provider = unsafe { &mut *(self_ as *mut Provider) };
    let state = unsafe { *state };
    provider
        .recorded
        .applied
        .push((slot.index, slot.generation, state));
}

unsafe extern "C" fn focus(self_: *mut c_void, slot: FloomNativeSlot) {
    let provider = unsafe { &mut *(self_ as *mut Provider) };
    provider
        .recorded
        .focused
        .push((slot.index, slot.generation));
}

unsafe extern "C" fn blur(self_: *mut c_void, slot: FloomNativeSlot) {
    let provider = unsafe { &mut *(self_ as *mut Provider) };
    provider
        .recorded
        .blurred
        .push((slot.index, slot.generation));
}

/// Build a vtable over `provider`, the way a native side would.
fn vtable(provider: &mut Provider) -> FloomNativeProviderV1 {
    FloomNativeProviderV1 {
        abi_version: ABI_VERSION,
        self_: provider as *mut Provider as *mut c_void,
        attach: Some(attach),
        capabilities: Some(capabilities),
        create: Some(create),
        destroy: Some(destroy),
        apply: Some(apply),
        focus: Some(focus),
        blur: Some(blur),
    }
}

fn slot(index: u32, generation: u32) -> floom_native::NativeSlot {
    floom_native::NativeSlot {
        id: NativeViewId::new(index, generation),
        generation,
    }
}

#[test]
fn a_provider_behind_the_c_abi_is_driven_through_the_controller() {
    let mut provider = Provider {
        recorded: Recorded::default(),
        refuse: false,
    };
    let table = vtable(&mut provider);

    // SAFETY: `table` is a live local and `check` only reads it.
    unsafe { check(&table as *const FloomNativeProviderV1) }.expect("the vtable is usable");
    // SAFETY: the table and the provider both outlive the controller below.
    let c_provider = unsafe { CProvider::from_raw(&table as *const FloomNativeProviderV1) };
    let mut controller = NativeViewController::boxed(c_provider);

    // The capabilities crossed, in the provider's own order.
    let caps = controller.capabilities();
    assert_eq!(
        caps.modes.as_ref(),
        &[
            NativeCompositionMode::Overlay,
            NativeCompositionMode::Texture
        ],
        "the provider published its modes in an order, and that order crossed"
    );
    assert!(caps.focusable);
    assert!(caps.can_crop());
    assert_eq!(caps.clip, ClipSupport::Platform);
    assert!(!caps.snapshot);

    // Binding negotiates a mode and creates the participant.
    let bound = slot(3, 1);
    let mode = controller
        .bind(
            bound,
            NativeViewKind::Video,
            &[NativeCompositionMode::Overlay],
        )
        .expect("the provider offers overlay");
    assert_eq!(mode, NativeCompositionMode::Overlay);
    assert_eq!(provider.recorded.created.len(), 1);
    assert_eq!(provider.recorded.created[0].0, 3);
    assert_eq!(provider.recorded.created[0].1, 1);

    // The state crossed in **device pixels**, at the scale the framework says.
    let state = NativeState {
        rect: Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 110.0,
            y1: 70.0,
        },
        clip: None,
        scale: 2.0,
        visible: true,
        z: 5,
        input: floom_native::InputMode::Capture,
        mode: NativeCompositionMode::Overlay,
    };
    controller.apply(bound, state).expect("applied");
    let (_, _, crossed) = &provider.recorded.applied[0];
    assert_eq!(
        crossed.rect,
        FloomNativeDeviceRect {
            x: 20,
            y: 40,
            width: 200,
            height: 100,
        },
        "the rect crossed as whole device pixels, converted once"
    );
    assert_eq!(crossed.visible, 1);
    assert_eq!(
        crossed.has_clip, 0,
        "no crop is not the same as a crop of nothing"
    );
    assert_eq!(crossed.z, 5);
    assert_eq!(crossed.input, INPUT_CAPTURE);
    assert_eq!(crossed.mode, 0);

    // A crop crosses as device pixels plus a flag.
    let cropped = NativeState {
        clip: Some(Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 60.0,
            y1: 45.0,
        }),
        ..state
    };
    controller.apply(bound, cropped).expect("applied");
    let (_, _, crossed) = &provider.recorded.applied[1];
    assert_eq!(crossed.has_clip, 1);
    assert_eq!(
        crossed.clip,
        FloomNativeDeviceRect {
            x: 20,
            y: 40,
            width: 100,
            height: 50,
        }
    );

    // Focus and blur crossed.
    controller.focus(bound).expect("focused");
    assert_eq!(provider.recorded.focused, vec![(3, 1)]);
    controller.blur(bound).expect("blurred");
    assert_eq!(provider.recorded.blurred, vec![(3, 1)]);

    // Tearing down destroys the participant.
    controller.destroy(bound).expect("destroyed");
    assert_eq!(provider.recorded.destroyed, vec![(3, 1)]);
    assert!(controller.unreleased().next().is_none());
}

#[test]
fn the_custom_kind_name_crosses_and_the_refusal_message_comes_back() {
    let mut provider = Provider {
        recorded: Recorded::default(),
        refuse: true,
    };
    let table = vtable(&mut provider);
    // SAFETY: the table and the provider both outlive the controller below.
    let c_provider = unsafe { CProvider::from_raw(&table as *const FloomNativeProviderV1) };
    let mut controller = NativeViewController::boxed(c_provider);

    let bound = slot(1, 0);
    // Binding calls `create`, which refuses.
    let error = controller
        .bind(
            bound,
            NativeViewKind::Custom("com.example.three-d"),
            &[NativeCompositionMode::Overlay],
        )
        .expect_err("the provider refuses");

    // The provider's own message came back, not a wrapper around it.
    match error {
        NativeError::Provider(detail) => assert_eq!(
            detail.as_ref(),
            "the provider refuses on purpose",
            "the provider's own message crossed the boundary"
        ),
        other => panic!("expected the provider's refusal, got {other:?}"),
    }

    // The custom kind's name crossed as well.
    assert_eq!(provider.recorded.created.len(), 1);
    assert_eq!(provider.recorded.created[0].2, KIND_CUSTOM);
    assert_eq!(provider.recorded.created[0].3, "com.example.three-d");
}

#[test]
fn a_provider_with_the_wrong_abi_version_is_refused() {
    let mut provider = Provider {
        recorded: Recorded::default(),
        refuse: false,
    };
    let mut table = vtable(&mut provider);
    table.abi_version = ABI_VERSION + 1;

    // SAFETY: `table` is a live local and `check` only reads it.
    let error = unsafe { check(&table as *const FloomNativeProviderV1) }
        .expect_err("a mismatched ABI is refused");
    assert!(
        error.to_string().contains("ABI version"),
        "the refusal says what is wrong: {error}"
    );
}

#[test]
fn a_provider_with_a_missing_entry_is_refused() {
    let mut provider = Provider {
        recorded: Recorded::default(),
        refuse: false,
    };
    let mut table = vtable(&mut provider);
    table.apply = None;

    // SAFETY: `table` is a live local and `check` only reads it.
    let error = unsafe { check(&table as *const FloomNativeProviderV1) }
        .expect_err("a missing entry point is refused");
    assert!(
        error.to_string().contains("apply"),
        "the refusal names the entry point: {error}"
    );
}

#[test]
fn the_input_modes_cross_as_a_pair() {
    let mut provider = Provider {
        recorded: Recorded::default(),
        refuse: false,
    };
    let table = vtable(&mut provider);
    // SAFETY: the table and the provider both outlive the controller below.
    let c_provider = unsafe { CProvider::from_raw(&table as *const FloomNativeProviderV1) };
    let controller = NativeViewController::boxed(c_provider);

    let caps = controller.capabilities();
    assert_eq!(
        caps.input,
        InputModes {
            passthrough: true,
            capture: true
        }
    );
    assert_eq!(INPUT_PASSTHROUGH, 0);
}

/// The conversion the ABI relies on, checked against the framework's own.
///
/// The adapter calls `to_device`; this asserts that the answer is the one the
/// framework would give, so the two cannot drift apart silently.
#[test]
fn the_device_conversion_is_the_frameworks_own() {
    let rect = Rect {
        x0: 10.0,
        y0: 20.0,
        x1: 110.0,
        y1: 70.0,
    };
    let device = to_device(rect, 2.0);
    assert_eq!(device.x, 20);
    assert_eq!(device.y, 40);
    assert_eq!(device.width, 200);
    assert_eq!(device.height, 100);
}
