// SPDX-License-Identifier: MIT
//! Native composition, host side (Phase 16, task 16.2).
//!
//! A participant is a native view the framework does not paint — an `NSView`,
//! an `HWND`, a real compositor surface. The host owns its lifetime, computes
//! its geometry every frame, and *subtracts* it from the framework's own
//! attention: a capturing participant takes the pointer, and a participant that
//! can be focused takes IME ownership away from the framework's text fields.
//!
//! None of that needs a device. Every assertion here is about *which commands a
//! provider received*, which is why [`FakeNativeProvider`] records them and why
//! the whole task is testable on a laptop. The device cases (D1–D6 in the phase
//! plan) check the one thing this cannot: that a real OS view lands where the
//! host said it would.

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use floom_animation::{AnimKind, CompositorProp, Tween};
use floom_core::{
    CreateContext, ElementId, Length, NativeGrant, NativeRequest, UpdateContext, Widget, WidgetAny,
};
use floom_layout::{AbsoluteInset, LayoutStyle, Position};
use floom_native::{
    ClipSupport, FakeNativeProvider, InputMode, InputModes, NativeCapabilities, NativeCommandLog,
    NativeCompositionMode, NativeSlot, NativeViewKind,
};
use floom_testkit::TestApp;
use floom_types::{Point, Rect, ResourceId, Size};
use floom_widgets::NativeSurface;
use floom_widgets::controls::Button;
use floom_widgets::overlay::OverlayKind;
use floom_widgets::primitives::Container;

// --- scaffolding -----------------------------------------------------------

/// A 400x300 window whose root is an empty box, so nodes can be placed at exact
/// coordinates. Participants live and die by their rects, and a flow layout
/// would make every assertion here an assertion about the layout engine.
fn empty_app() -> TestApp {
    TestApp::new(Size::new(400.0, 300.0), Container::new())
}

/// A provider plus the handle a test keeps to ask what it was told. The host
/// takes the provider by value, so the handle is the only way back.
fn provider() -> (FakeNativeProvider, NativeCommandLog) {
    let mut provider = FakeNativeProvider::new();
    let log = provider.watch();
    (provider, log)
}

/// The same, for a backend whose views cannot be cropped by anyone — a KMS
/// plane, or a view placed straight into the window's compositor tree.
fn cropless_provider() -> (FakeNativeProvider, NativeCommandLog) {
    let mut provider = FakeNativeProvider::cannot_crop();
    let log = provider.watch();
    (provider, log)
}

/// A backend that offers one mode and cannot crop it. Nothing about this
/// participant can be made correct, which is the case the host has to be honest
/// about rather than compositing anyway.
fn surface_only_cropless() -> FakeNativeProvider {
    FakeNativeProvider::with_capabilities(NativeCapabilities {
        modes: std::borrow::Cow::Borrowed(&[NativeCompositionMode::Surface]),
        snapshot: false,
        input: InputModes::ALL,
        focusable: false,
        clip: ClipSupport::Unsupported,
    })
}

/// A box at `rect`, taken out of the parent's flow so two nodes can overlap.
fn at(x: f32, y: f32, width: f32, height: f32) -> LayoutStyle {
    LayoutStyle {
        position: Position::Absolute,
        inset: AbsoluteInset {
            left: Some(x),
            top: Some(y),
            right: None,
            bottom: None,
        },
        width: Some(Length::Px(width)),
        height: Some(Length::Px(height)),
        ..LayoutStyle::default()
    }
}

/// The single participant's last pushed state.
fn live_state(app: &TestApp) -> floom_native::NativeState {
    let states = app.host().participant_states();
    assert_eq!(states.len(), 1, "expected exactly one participant");
    states[0].1
}

// --- a widget that re-keys itself ------------------------------------------

/// A participant whose identity changes on demand — a row in a virtualized
/// list, without the list.
///
/// The interesting behaviour is not the widget but what the host does when the
/// key changes: the old slot is destroyed, released to the pool, and the new
/// key acquires a recycled one. A row that scrolled out of view must not leave
/// a native view behind, and a row scrolling in must not mint one.
struct RowSurface {
    key: Rc<Cell<u64>>,
    grant: NativeGrant,
}

impl RowSurface {
    fn new(key: Rc<Cell<u64>>) -> Self {
        Self {
            key,
            grant: NativeGrant::Refused,
        }
    }
}

impl Widget for RowSurface {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // Stated every frame, exactly as `NativeSurface` does: the host makes it
        // idempotent, and this is how a changed key becomes a new slot with no
        // explicit teardown.
        let request = NativeRequest::new(NativeViewKind::Video).key(self.key.get());
        let grant = cx.request_native(&request);
        if grant != self.grant {
            self.grant = grant;
            cx.mark_dirty(id);
        }
    }
}

impl WidgetAny for RowSurface {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

// --- registry + per-frame diff ---------------------------------------------

/// With no provider there is nothing to composite, and the host says so rather
/// than handing out a participant that draws nothing.
#[test]
fn without_a_provider_the_request_is_refused() {
    let mut app = empty_app();
    let root = app.root();
    let node = app.host_mut().mount(
        root,
        at(0.0, 0.0, 100.0, 60.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    app.frame();

    assert!(!app.host().has_native_provider());
    assert_eq!(app.host().participant_count(), 0);
    assert_eq!(
        app.widget::<NativeSurface>(node).grant(),
        NativeGrant::Refused,
        "the widget must be told, so it can draw a fallback"
    );
}

/// The lifecycle: one `create` at bind, then one `apply` carrying the geometry
/// the layout pass produced.
#[test]
fn a_grant_binds_the_participant_and_applies_its_geometry() {
    let mut app = empty_app();
    let root = app.root();
    let node = app.host_mut().mount(
        root,
        at(10.0, 20.0, 120.0, 80.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    assert_eq!(app.host().participant_count(), 1);
    assert_eq!(log.creates(), 1);
    assert_eq!(log.destroys(), 0);
    assert_eq!(
        app.widget::<NativeSurface>(node).grant(),
        NativeGrant::Granted(NativeCompositionMode::Overlay),
        "the strongest mode the caller accepts and the provider offers"
    );

    let state = live_state(&app);
    assert!(state.visible);
    assert_eq!(state.mode, NativeCompositionMode::Overlay);
    assert_eq!(
        state.rect,
        Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 130.0,
            y1: 100.0
        },
        "the participant's rect is its node's box"
    );
    // The box the provider is told to draw in must round to whole device
    // pixels; the logical→device crossing happens exactly once, host-side.
    assert_eq!(state.device_rect().width, 120);
}

/// A display change doubles the device rect and leaves the participant's rect
/// exactly where it was.
///
/// This is the line between a participant and a painted widget. The framework's
/// own content is drawn into a surface the presenter already scales, so a denser
/// display is the presenter's problem alone; a participant is placed *in device
/// pixels* by the platform layer, which means the host has to be told the density
/// or it places a real OS view at half size — in a window that is itself drawn
/// correctly, which is what makes it hard to spot. The logical rect is a
/// statement about layout and must not move (device case D1).
#[test]
fn a_display_change_doubles_the_device_rect_and_leaves_the_rect_alone() {
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(10.0, 20.0, 120.0, 80.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    let before = live_state(&app);
    assert_eq!(
        before.rect,
        Rect {
            x0: 10.0,
            y0: 20.0,
            x1: 130.0,
            y1: 100.0
        }
    );
    assert_eq!(before.device_rect().width, 120, "1x to start");
    let commands = log.count();

    let window = app.window();
    app.platform().set_scale_factor(window, 2.0);
    app.settle();

    let after = live_state(&app);
    assert_eq!(
        after.rect, before.rect,
        "layout is in logical units: moving a window to a denser display does not move anything"
    );
    assert_eq!(after.device_rect().width, 240, "the OS view doubles");
    assert_eq!(after.device_rect().height, 160);
    assert_eq!(
        log.count(),
        commands + 1,
        "the participant is re-placed at the new density, not left behind at the old one"
    );
}

/// A frame in which nothing moved must not talk to the provider at all. This is
/// the whole point of keeping the diff host-side: an `apply` per frame per
/// participant is an OS call per frame per participant.
#[test]
fn an_idle_frame_tells_the_provider_nothing() {
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 100.0, 60.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();
    assert_eq!(log.count(), 2, "create + apply");

    for _ in 0..3 {
        app.frame();
    }
    assert_eq!(log.count(), 2, "idle frames issue no commands");
}

/// Moving the node issues exactly one new `apply`, and the state it carries is
/// absolute — a provider never has to integrate a delta.
#[test]
fn moving_a_participant_applies_exactly_one_new_state() {
    let mut app = empty_app();
    let root = app.root();
    let node = app.host_mut().mount(
        root,
        at(0.0, 0.0, 100.0, 60.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();
    let before = log.count();

    app.host_mut().set_style(node, at(40.0, 0.0, 100.0, 60.0));
    app.frame();

    assert_eq!(log.count(), before + 1, "one apply, not one per frame");
    let state = live_state(&app);
    assert_eq!(state.rect.x0, 40.0);
}

/// Unmounting destroys the participant exactly once — a leak here is a native
/// view the framework can no longer name, which is invisible until the OS runs
/// out of them.
#[test]
fn unmounting_destroys_the_participant_exactly_once() {
    let mut app = empty_app();
    let root = app.root();
    let node = app.host_mut().mount(
        root,
        at(0.0, 0.0, 100.0, 60.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();
    assert_eq!(app.host().participant_count(), 1);

    app.host_mut().unmount(node);
    app.frame();
    app.frame();

    assert_eq!(log.creates(), 1);
    assert_eq!(log.destroys(), 1, "destroyed once, not once per frame");
    assert_eq!(app.host().participant_count(), 0);
    assert!(
        app.host().unreleased_participants().is_empty(),
        "nothing is left bound after teardown"
    );
}

// --- clipping and visibility ----------------------------------------------

/// A participant clipped away by an ancestor flips invisible, and is *not*
/// moved to a rect outside it. A native view does not clip itself to anything,
/// so without this a participant in a scrolled container floats over content it
/// should be behind (device case D6).
#[test]
fn an_ancestor_clipping_the_participant_hides_it() {
    let mut app = empty_app();
    let root = app.root();
    // A 200x100 box, and a surface positioned below its bottom edge.
    let clip = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 100.0), Container::new());
    app.host_mut().mount(
        clip,
        at(0.0, 150.0, 100.0, 50.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    let state = live_state(&app);
    assert!(
        !state.visible,
        "clipped away by its ancestor, so not visible"
    );
    assert_eq!(
        state.rect,
        Rect {
            x0: 0.0,
            y0: 150.0,
            x1: 100.0,
            y1: 200.0
        },
        "still told where the view is — the placement does not move because \
         the view happens not to be on screen"
    );
    assert_eq!(
        state.clip,
        Some(Rect::ZERO),
        "what is left of it is nothing, which is a different statement"
    );
    assert!(state.visible_rect().is_empty());
    // Hidden is not gone: the view stays bound, because it is one scroll away.
    assert_eq!(log.creates(), 1);
    assert_eq!(log.destroys(), 0);
}

/// A *partial* clip is carried as a crop, never folded into the placement.
///
/// This is the case that decides whether a native view comes out squashed. A
/// provider handed a 100x50 box for a view that is really 100x100 will fit its
/// content into it — which looks plausible on a map and obviously wrong on a
/// video, and is exactly the bug that survives a release.
#[test]
fn a_partial_clip_is_carried_as_a_crop_not_folded_into_the_placement() {
    let mut app = empty_app();
    let root = app.root();
    let clip = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 100.0), Container::new());
    // Half of it hangs out of the bottom of the container.
    app.host_mut().mount(
        clip,
        at(0.0, 50.0, 100.0, 100.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    let state = live_state(&app);
    assert!(state.visible, "half of it is on screen");
    assert_eq!(
        state.rect,
        Rect {
            x0: 0.0,
            y0: 50.0,
            x1: 100.0,
            y1: 150.0
        },
        "the placement is the node's box, at full size"
    );
    assert_eq!(
        state.clip,
        Some(Rect {
            x0: 0.0,
            y0: 50.0,
            x1: 100.0,
            y1: 100.0
        }),
        "and the crop is the part the ancestor leaves showing"
    );
    assert_eq!(state.visible_rect(), state.clip.unwrap());
    assert_ne!(
        state.visible_rect(),
        state.rect,
        "if these were the same rect the view would have been resized"
    );
    assert!(
        state.needs_crop(),
        "and the provider is told it has work to do"
    );
}

/// A provider that cannot crop has one more way out, and the host takes it
/// automatically: move the participant to a mode whose pixels the *framework*
/// owns, where cropping is free.
///
/// The mode is what gives, never the geometry — the participant keeps its box
/// and is told the crop, and a `Texture` participant can be cropped by the
/// framework that draws it.
#[test]
fn a_provider_that_cannot_crop_is_moved_to_a_mode_the_framework_composites() {
    let mut app = empty_app();
    let root = app.root();
    let clip = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 100.0), Container::new());
    let surface = app.host_mut().mount(
        clip,
        at(0.0, 50.0, 100.0, 100.0),
        NativeSurface::new(NativeViewKind::Video).modes(&[
            NativeCompositionMode::Overlay,
            NativeCompositionMode::Texture,
        ]),
    );
    let (fake, log) = cropless_provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    let stats = app.host().last_stats();
    assert_eq!(
        stats.native_degraded, 1,
        "one participant was recomposited to survive the clip"
    );
    assert_eq!(stats.native_clip_hidden, 0, "and nothing had to be hidden");

    let state = live_state(&app);
    assert!(state.visible);
    assert_eq!(
        state.mode,
        NativeCompositionMode::Texture,
        "Overlay is not croppable by us, Texture is: the caller's order decides \
         which of the two wins"
    );
    assert_eq!(
        state.rect,
        Rect {
            x0: 0.0,
            y0: 50.0,
            x1: 100.0,
            y1: 150.0
        },
        "the box is unchanged — the view was not resized, it was recomposited"
    );
    assert_eq!(log.creates(), 2, "a new view in the new mode");
    assert_eq!(log.destroys(), 1, "and the old one gone");

    // The widget learns on its *next* update: the host decides the mode after
    // this frame's `update()` has already run. One frame of lag is fine — the
    // grant only tells a widget whether to draw a fallback — and the
    // alternative is asking every widget to re-read its mode twice per frame.
    app.frame();
    assert_eq!(
        app.widget::<NativeSurface>(surface).grant().mode(),
        Some(NativeCompositionMode::Texture),
        "the widget is told what it actually got"
    );

    // The decision is one-way and costs one frame: a per-frame re-negotiation
    // would destroy and re-create a native view on every scrolled frame at the
    // boundary. `native_degraded` counts per frame, like `native_applied`, so
    // "not re-decided" reads as zero here.
    for _ in 0..4 {
        app.frame();
    }
    assert_eq!(
        app.host().last_stats().native_degraded,
        0,
        "the decision was taken once"
    );
    assert_eq!(log.creates(), 2, "and the view was not re-created");
    assert_eq!(
        live_state(&app).mode,
        NativeCompositionMode::Texture,
        "still composited the way it had to be"
    );
}

/// When no mode the framework composites is on offer either, the participant is
/// hidden — and the placement it is told is still the truth.
///
/// A gap is wrong and *visible*; drawing it unclipped (device case D6) and
/// scaling its content to fit are both wrong and *silent*. That is the whole
/// reason this case exists rather than being composited anyway.
#[test]
fn a_provider_that_can_neither_crop_nor_be_recomposited_hides_the_participant() {
    let mut app = empty_app();
    let root = app.root();
    let clip = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 100.0), Container::new());
    app.host_mut().mount(
        clip,
        at(0.0, 50.0, 100.0, 100.0),
        NativeSurface::new(NativeViewKind::Video).modes(&[NativeCompositionMode::Surface]),
    );
    app.host_mut()
        .register_native_provider(surface_only_cropless(), &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    let stats = app.host().last_stats();
    assert_eq!(
        stats.native_clip_hidden, 1,
        "the host admits it cannot composite this correctly"
    );
    assert_eq!(stats.native_degraded, 0);

    let state = live_state(&app);
    assert!(!state.visible);
    assert_eq!(
        state.rect,
        Rect {
            x0: 0.0,
            y0: 50.0,
            x1: 100.0,
            y1: 150.0
        },
        "and it is told where the view really is, not where it would have to \
         shrink to"
    );

    for _ in 0..4 {
        app.frame();
    }
    assert_eq!(
        app.host().last_stats().native_clip_hidden,
        0,
        "the decision was taken once, not once per frame"
    );
    assert!(
        !live_state(&app).visible,
        "and the participant stays off the screen it cannot be composited on"
    );
}

/// The viewport is the outermost clip. A participant scrolled off the window
/// flips invisible for the same reason.
#[test]
fn a_participant_off_the_viewport_hides_it() {
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(500.0, 0.0, 100.0, 50.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    assert!(
        !live_state(&app).visible,
        "x0=500 is outside a 400px window"
    );
}

/// A modal owns the screen. The participant is hidden rather than destroyed:
/// it is straight back when the modal closes, and re-creating an OS view is the
/// expensive part.
#[test]
fn a_modal_hides_the_participant_without_destroying_it() {
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 100.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();
    assert!(live_state(&app).visible);

    let overlay = app.host_mut().open_overlay(
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(100.0, 80.0), Size::new(200.0, 120.0)),
        Button::new("ok"),
    );
    app.frame();

    assert_eq!(app.overlay_count(), 1);
    assert!(
        !live_state(&app).visible,
        "a modal is composited above the participant, so it must go away"
    );
    assert_eq!(log.destroys(), 0, "hidden, not torn down");

    app.host_mut().close_overlay(overlay);
    app.frame();
    assert!(live_state(&app).visible, "and back when the modal closes");
}

/// A participant an ancestor has clipped away must stop claiming the pointer.
///
/// Native regions outrank every framework one, because in Overlay mode the
/// participant is stacked above the whole scene. That is right while it is on
/// screen and wrong the moment it is not: an invisible participant that keeps
/// its region swallows clicks on whatever *is* painted there, with nothing on
/// screen to explain it. A row scrolled out of its container is exactly this —
/// its node box still overlaps the window while the container shows nothing.
#[test]
fn a_participant_clipped_away_stops_claiming_the_pointer() {
    let clicked = Rc::new(Cell::new(0));
    let sink = clicked.clone();
    let mut app = empty_app();
    let root = app.root();
    // A 200x100 container holding a surface that has slipped below its bottom
    // edge — scrolled out of view, but still where the layout put it.
    let container = app
        .host_mut()
        .mount(root, at(0.0, 0.0, 200.0, 100.0), Container::new());
    app.host_mut().mount(
        container,
        at(0.0, 150.0, 100.0, 50.0),
        NativeSurface::new(NativeViewKind::Video).input(InputMode::Capture),
    );
    // And a button painted exactly where the participant's node box is, which
    // is what the user actually sees at that spot.
    app.host_mut().mount(
        root,
        at(0.0, 150.0, 100.0, 50.0),
        Button::new("visible").on_click(move || sink.set(sink.get() + 1)),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    assert!(
        !live_state(&app).visible,
        "the container clipped it away, so it is not on screen"
    );

    app.click(Point::new(50.0, 170.0));

    assert_eq!(
        clicked.get(),
        1,
        "the button that is painted there got the click, not the participant \
         that is not"
    );
}

// --- z-order ---------------------------------------------------------------

/// Participants stack in mount order: what the tree paints later, the provider
/// stacks higher.
#[test]
fn paint_order_decides_z() {
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 100.0),
        NativeSurface::new(NativeViewKind::Map),
    );
    app.host_mut().mount(
        root,
        at(20.0, 20.0, 200.0, 100.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Map, NativeViewKind::Video])
        .unwrap();
    app.frame();

    let states = app.host().participant_states();
    assert_eq!(states.len(), 2);
    assert!(
        states[0].1.z < states[1].1.z,
        "the later-mounted participant outranks the earlier one: {:?}",
        states.iter().map(|(_, s)| s.z).collect::<Vec<_>>()
    );
}

// --- hit-test subtraction --------------------------------------------------

/// A capturing participant takes the pointer. The framework *subtracts* its
/// rect rather than adding to it — otherwise a click on a map also activates
/// the button the map is drawn over.
#[test]
fn a_capturing_participant_swallows_the_click() {
    let clicked = Rc::new(Cell::new(0));
    let sink = clicked.clone();
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 120.0),
        NativeSurface::new(NativeViewKind::Video).input(InputMode::Capture),
    );
    // Mounted *after* the surface, so plain paint order would put the button on
    // top. The participant still wins: it is composited above the whole scene.
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 120.0),
        Button::new("underneath").on_click(move || sink.set(sink.get() + 1)),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    app.click(Point::new(100.0, 60.0));

    assert_eq!(
        clicked.get(),
        0,
        "the framework widget under a capturing participant never sees the click"
    );
}

/// ...and a passthrough participant does not. The uniform is composited in the
/// same place but does not want input, so the framework routes hits as if it
/// were not there.
#[test]
fn a_passthrough_participant_lets_the_widget_underneath_have_it() {
    let clicked = Rc::new(Cell::new(0));
    let sink = clicked.clone();
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 120.0),
        NativeSurface::new(NativeViewKind::Video).input(InputMode::Passthrough),
    );
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 120.0),
        Button::new("underneath").on_click(move || sink.set(sink.get() + 1)),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    app.click(Point::new(100.0, 60.0));

    assert_eq!(clicked.get(), 1, "a passthrough participant is transparent");
}

// --- virtualized rows ------------------------------------------------------

/// A list scrolling through a thousand rows keeps ten native views alive.
///
/// The keys are what make that true: a row scrolling out releases its slot and
/// the row scrolling in acquires it, so the pool hands back the *same*
/// `NativeViewId` at a bumped generation — which is how a platform backend
/// knows it is holding a dead handle and must re-bind.
#[test]
fn scrolling_a_thousand_rows_keeps_the_slot_count_bounded() {
    const ROWS: u64 = 10;
    const STEPS: u64 = 100;

    let mut app = empty_app();
    let root = app.root();
    let keys: Vec<Rc<Cell<u64>>> = (0..ROWS).map(|_| Rc::new(Cell::new(0))).collect();
    for (i, key) in keys.iter().enumerate() {
        key.set(i as u64);
        app.host_mut().mount(
            root,
            at(0.0, i as f32 * 20.0, 100.0, 18.0),
            RowSurface::new(key.clone()),
        );
    }
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    assert_eq!(app.host().participant_count(), ROWS as usize);
    assert_eq!(log.creates(), ROWS as usize);
    assert_eq!(log.destroys(), 0);

    let mut destroyed = 0;
    for step in 1..=STEPS {
        // Scroll by one row: every visible row now shows a different item.
        for (i, key) in keys.iter().enumerate() {
            key.set(step * ROWS + i as u64);
        }
        app.frame();

        assert_eq!(
            app.host().participant_count(),
            ROWS as usize,
            "the live participant count never grows with the scroll"
        );
        assert_eq!(
            log.destroys(),
            destroyed + ROWS as usize,
            "each re-keyed row destroyed its previous view exactly once"
        );
        assert_eq!(
            log.creates(),
            ROWS as usize * (step + 1) as usize,
            "and created exactly one replacement"
        );
        destroyed = log.destroys();
    }

    // A thousand rows went past a ten-row viewport, and the framework is
    // holding ten native views — not a thousand.
    let slots = app.host().participant_slots();
    assert_eq!(slots.len(), ROWS as usize);
    let mut ids: Vec<_> = slots.iter().map(|slot| slot.id).collect();
    ids.sort_by_key(|id| format!("{id:?}"));
    ids.dedup();
    assert_eq!(ids.len(), ROWS as usize, "never more than ten live views");
    assert!(
        slots.iter().all(|slot| slot.generation > 1),
        "every slot was recycled at least once, so every one carries a bumped \
         generation a backend can detect: {:?}",
        slots.iter().map(|s| s.generation).collect::<Vec<_>>()
    );
    assert!(
        app.host().unreleased_participants().len() == ROWS as usize,
        "the ten live slots are exactly what the provider was told to create"
    );
}

/// Mounting a participant and then tearing the provider away leaves nothing
/// bound behind.
#[test]
fn clearing_the_provider_tears_every_participant_down() {
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 100.0, 60.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    app.host_mut().mount(
        root,
        at(0.0, 80.0, 100.0, 60.0),
        NativeSurface::new(NativeViewKind::Map),
    );
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Map, NativeViewKind::Video])
        .unwrap();
    app.frame();
    assert_eq!(app.host().participant_count(), 2);

    app.host_mut().clear_native_provider();
    app.frame();

    assert_eq!(log.destroys(), 2, "both views were destroyed");
    assert!(app.host().unreleased_participants().is_empty());
    assert_eq!(app.host().participant_count(), 0);
    assert!(!app.host().has_native_provider());
}

// --- 16.4 step 1: freeze and thaw ------------------------------------------

/// A participant with a provider installed and geometry applied, plus its slot.
fn bound_surface(app: &mut TestApp) -> NativeSlot {
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 80.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    app.frame();
    app.host().participant_slots()[0]
}

/// A frame whose *last* pushed state for the single participant is the thing
/// under test — a freeze that never reached the provider would leave the older,
/// visible one in place and look fine in every other respect.
fn last_applied(log: &NativeCommandLog) -> floom_native::NativeState {
    log.applied().last().expect("nothing was ever applied").1
}

/// Freezing withdraws the live view.
///
/// This is the swap itself: the OS keeps showing whatever it was showing unless
/// somebody tells it to stop, so leaving the view visible would draw the native
/// view *and* the texture — two copies of one participant, a frame apart, which
/// is visible the moment anything moves.
#[test]
fn freezing_a_participant_takes_its_live_view_off_screen() {
    let mut app = empty_app();
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);
    assert!(live_state(&app).visible, "precondition: compositing live");

    app.host_mut().freeze_native(slot).unwrap().unwrap();
    app.frame();

    assert!(
        !last_applied(&log).visible,
        "the live view must be hidden while the texture stands in for it"
    );
    assert_eq!(app.host().frozen_participants(), vec![slot]);
    // Still bound: a transition's end hands the participant back, and
    // re-creating an OS view is the expensive thing.
    assert_eq!(app.host().participant_count(), 1);
    assert_eq!(log.destroys(), 0);
}

/// The texture has to actually reach the screen, through the widget whose box
/// the participant occupies.
///
/// The participant's own `paint` is empty by design — the OS view paints
/// itself — so a freeze that forgot the drawing half leaves a hole exactly
/// where the video was, for the whole transition. That absence is what device
/// case D11 names as "the participant is missing during the animation".
#[test]
fn a_frozen_participant_is_painted_as_an_image_in_its_own_box() {
    let mut app = empty_app();
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);

    let snapshot = app.host_mut().freeze_native(slot).unwrap().unwrap();
    app.frame();

    let drawn = app
        .scene_nodes()
        .into_iter()
        .filter(|node| matches!(node, floom_scene::SceneNode::Image { image, .. } if *image == snapshot.resource))
        .count();
    assert_eq!(drawn, 1, "the frozen texture must be drawn exactly once");
}

/// Thawing hands ownership back: the view goes live and the provider is told
/// it can have its pixels again.
#[test]
fn thawing_restores_the_live_view() {
    let mut app = empty_app();
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);

    app.host_mut().freeze_native(slot).unwrap().unwrap();
    app.frame();
    log.clear();
    app.host_mut().thaw_native(slot).unwrap();
    app.frame();

    assert!(last_applied(&log).visible, "the live view must come back");
    assert!(
        app.host().frozen_participants().is_empty(),
        "a thawed participant is no longer frozen"
    );
    assert_eq!(
        log.commands()
            .iter()
            .filter(|c| matches!(c, floom_native::NativeCommand::Restore { .. }))
            .count(),
        1,
        "ownership moves exactly once"
    );
}

/// Ownership moves once, so the illegal states have to be *reported*. A second
/// freeze would strand the first texture and a stray thaw would hand back a
/// texture nobody owns — both indistinguishable from working in the moment.
#[test]
fn the_freeze_lifecycle_rejects_double_and_stray_calls() {
    let mut app = empty_app();
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);

    app.host_mut().freeze_native(slot).unwrap().unwrap();
    assert_eq!(
        app.host_mut().freeze_native(slot),
        Err(floom_native::NativeError::AlreadyFrozen(slot)),
        "freezing twice strands the first snapshot"
    );
    assert_eq!(
        log.commands()
            .iter()
            .filter(|c| matches!(c, floom_native::NativeCommand::Snapshot { .. }))
            .count(),
        1,
        "the rejected second freeze must not reach the provider"
    );

    app.host_mut().thaw_native(slot).unwrap();
    assert_eq!(
        app.host_mut().thaw_native(slot),
        Err(floom_native::NativeError::NotFrozen(slot)),
        "thawing twice frees a texture nobody owns"
    );
}

/// `Ok(None)` is not a failure: a provider that cannot freeze into a texture is
/// answered honestly, and the participant simply carries on being composited
/// live. Getting this wrong is how a platform without snapshot support would
/// silently lose its native views mid-transition.
#[test]
fn a_provider_that_cannot_snapshot_keeps_the_live_view() {
    let mut app = empty_app();
    let cannot = FakeNativeProvider::with_capabilities(NativeCapabilities {
        modes: std::borrow::Cow::Borrowed(&[NativeCompositionMode::Overlay]),
        snapshot: false,
        input: InputModes::ALL,
        focusable: false,
        clip: ClipSupport::Platform,
    });
    app.host_mut()
        .register_native_provider(cannot, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);

    assert_eq!(
        app.host_mut().freeze_native(slot),
        Ok(None),
        "cannot-freeze is a capability answer, not an error"
    );
    app.frame();

    assert!(
        live_state(&app).visible,
        "nothing was frozen, so the live view must stay live"
    );
    assert!(app.host().frozen_participants().is_empty());
}

/// Unmounting while frozen must not strand the texture.
///
/// The node goes away, so the widget that would have drawn the texture goes
/// with it — and the provider is still holding a view it thinks is frozen.
#[test]
fn an_unmounting_frozen_participant_hands_ownership_back() {
    let mut app = empty_app();
    let root = app.root();
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    let node = app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 80.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    app.frame();
    let slot = app.host().participant_slots()[0];

    app.host_mut().freeze_native(slot).unwrap().unwrap();
    log.clear();
    app.host_mut().unmount(node);
    app.frame();

    assert!(
        app.host().frozen_participants().is_empty(),
        "a retired participant cannot still be frozen"
    );
    assert_eq!(app.host().participant_count(), 0);
    assert_eq!(
        app.host().unreleased_participants(),
        Vec::new(),
        "nothing may survive that the framework can no longer name"
    );
    let _ = log;
}

// --- 16.4 step 2: animating the frozen texture -----------------------------

/// A 0.15 s slide from `from` to `to`, the shape a page transition asks for.
fn slide(from: f64, to: f64) -> AnimKind {
    AnimKind::Tween(Tween::new(from, to, 0.15).with_compositor(CompositorProp::TranslateX(0.0)))
}

/// The frozen image's drawn box, which is where a slide has to show up: the
/// live view cannot move — that is the entire reason it was frozen — so the
/// texture's bounds are the only thing on screen that can.
fn frozen_bounds(app: &TestApp, resource: ResourceId) -> Option<Rect> {
    app.scene_nodes().into_iter().find_map(|node| match node {
        floom_scene::SceneNode::Image { bounds, image, .. } if image == resource => Some(bounds),
        _ => None,
    })
}

/// The texture follows the tween, and the participant goes live when it ends.
///
/// D11 in two halves: *the pushed screen animates as a frozen texture*, and
/// *then goes live at the end*. Missing either half is a real failure — a
/// texture that never thaws is a participant that will never animate again,
/// and one that thaws on the first frame is not an animation.
#[test]
fn a_frozen_participant_is_animated_then_goes_live() {
    let mut app = empty_app();
    let (fake, log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);

    let snapshot = app
        .host_mut()
        .begin_native_transition(slot, slide(-40.0, 0.0))
        .unwrap()
        .unwrap();

    // Mid-transition: the texture has been drawn off its resting place.
    app.frame();
    let start = frozen_bounds(&app, snapshot.resource).expect("frozen texture is drawn");
    let mut moved = start;
    for _ in 0..4 {
        app.frame();
        if let Some(bounds) = frozen_bounds(&app, snapshot.resource) {
            moved = bounds;
        }
    }
    assert_ne!(
        moved.x0, start.x0,
        "the frozen texture must travel with the tween: {start:?} -> {moved:?}"
    );

    // Then it ends, by itself.
    for _ in 0..12 {
        app.frame();
    }
    assert!(
        app.host().frozen_participants().is_empty(),
        "a finished transition thaws the participant"
    );
    assert!(
        last_applied(&log).visible,
        "and the live view is what is on screen afterwards"
    );
    assert!(
        !app.host().overlay_animating(),
        "a settled transition stops owing frames"
    );
}

/// A provider that cannot snapshot must not be left half-transitioned: no
/// texture exists, so there is nothing to animate and nothing to thaw.
#[test]
fn a_transition_without_a_snapshot_animates_nothing() {
    let mut app = empty_app();
    let mut cannot = FakeNativeProvider::with_capabilities(NativeCapabilities {
        modes: std::borrow::Cow::Borrowed(&[NativeCompositionMode::Overlay]),
        snapshot: false,
        input: InputModes::ALL,
        focusable: false,
        clip: ClipSupport::Platform,
    });
    let log = cannot.watch();
    app.host_mut()
        .register_native_provider(cannot, &[NativeViewKind::Video])
        .unwrap();
    let slot = bound_surface(&mut app);

    assert_eq!(
        app.host_mut()
            .begin_native_transition(slot, slide(-40.0, 0.0)),
        Ok(None)
    );
    for _ in 0..6 {
        app.frame();
    }

    assert!(app.host().frozen_participants().is_empty());
    assert!(
        last_applied(&log).visible,
        "the live view never went away, so it never comes back"
    );
}

// A recycled slot does *not* need protecting from its predecessor's freeze, and
// it is worth saying why: the pool hands the slot back with a bumped
// generation, so `NativeSlot` equality — which the freeze record is keyed on —
// already tells the two occupants apart. What does need cleaning up is the
// record itself, which the test above covers; a version of this test keyed on
// the record survived deleting the cleanup, so it was asserting nothing.

/// A node that is not a participant keeps its own hit region, so the subtraction
/// above is a property of the participant and not of the host.
#[test]
fn a_plain_widget_still_receives_clicks_with_a_provider_installed() {
    let clicked = Rc::new(Cell::new(0));
    let sink = clicked.clone();
    let mut app = empty_app();
    let root = app.root();
    app.host_mut().mount(
        root,
        at(0.0, 200.0, 200.0, 80.0),
        Button::new("plain").on_click(move || sink.set(sink.get() + 1)),
    );
    app.host_mut().mount(
        root,
        at(0.0, 0.0, 200.0, 120.0),
        NativeSurface::new(NativeViewKind::Video),
    );
    let (fake, _log) = provider();
    app.host_mut()
        .register_native_provider(fake, &[NativeViewKind::Video])
        .unwrap();
    app.frame();

    app.click(Point::new(100.0, 240.0));

    assert_eq!(clicked.get(), 1);
}
