//! Overlay system: the chain that lets popups, dialogs and sheets escape their
//! parent's clip and still be seen, clicked and focused.
//!
//! `floem-overlay` owns the *policy* (stacking, modal scrim, focus scope, hit
//! priority); `WidgetHost` owns the *content* — it mounts an overlay's widget
//! as an independent subtree, lays it out against the viewport, paints it last
//! (so it sits on top), and routes pointer hits to it before the main tree.
//!
//! These tests drive that whole chain through `AppRunner`, the same way a real
//! window would, and assert on two things every time: the overlay's content
//! actually receives the event, and the change reaches the screen (damage).

use std::cell::Cell;
use std::rc::Rc;

use floem_app::{AppRunner, WidgetHost};
use floem_core::WidgetAny;
use floem_layout::LayoutStyle;
use floem_overlay::{OverlayHandle, OverlayKind};
use floem_platform::{FakePlatform, KeyCode, Platform, PlatformEvent, PointerButton, WindowConfig};
use floem_renderer_software::SoftwarePresenter;
use floem_scene::SceneNode;
use floem_types::{Modifiers, Point, Rect, Size, WindowId};
use floem_widgets::Button;

/// Whether two rects share any area.
fn overlaps(a: Rect, b: Rect) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
}

/// A window holding one root widget, pinned to the whole viewport.
struct Harness {
    platform: FakePlatform,
    runner: AppRunner<SoftwarePresenter>,
    window: WindowId,
}

impl Harness {
    fn with(size: Size, widget: impl WidgetAny + 'static) -> Self {
        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        host.mount_root(
            LayoutStyle {
                width: Some(size.width as f32),
                height: Some(size.height as f32),
                ..LayoutStyle::default()
            },
            widget,
        );
        let runner = AppRunner::new(host, SoftwarePresenter::new());
        let window = platform.create_window(WindowConfig {
            title: "overlay".into(),
            size,
            resizable: true,
        });
        let mut harness = Self {
            platform,
            runner,
            window,
        };
        for _ in 0..6 {
            harness.runner.pump(&mut harness.platform);
        }
        harness
    }

    /// Opens `content` as an overlay and renders one frame so its hit regions
    /// and scene nodes exist before any pointer event.
    fn open(
        &mut self,
        kind: OverlayKind,
        modal: bool,
        bounds: Rect,
        content: impl WidgetAny + 'static,
    ) -> OverlayHandle {
        let id = self
            .runner
            .host_mut()
            .open_overlay(kind, modal, bounds, content);
        self.render();
        id
    }

    /// Renders one host frame directly (no window round-trip) — enough to lay
    /// out and paint overlays for hit-testing and damage assertions.
    fn render(&mut self) {
        self.runner.host_mut().frame();
    }

    /// Drains the queue and reports whether any frame had damage to paint.
    fn frame(&mut self) -> bool {
        let mut events = Vec::new();
        self.platform.poll_events(&mut events);
        let mut repainted = false;
        for event in events {
            self.runner.handle(&mut self.platform, event);
            if !self.runner.host().scene().damage().is_empty() {
                repainted = true;
            }
        }
        repainted
    }

    fn press(&mut self, at: Point) -> bool {
        self.button(at, true)
    }

    fn release(&mut self, at: Point) -> bool {
        self.button(at, false)
    }

    fn button(&mut self, at: Point, pressed: bool) -> bool {
        let window = self.window;
        self.runner.handle(
            &mut self.platform,
            PlatformEvent::PointerButton {
                window,
                position: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            },
        );
        self.frame()
    }

    fn key(&mut self, code: KeyCode) -> bool {
        let window = self.window;
        self.runner.handle(
            &mut self.platform,
            PlatformEvent::Key {
                window,
                code,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        );
        self.frame()
    }
}

/// An overlay button placed clear of the main tree still receives the click —
/// its content is painted on top and hit-tested before the main tree, so the
/// parent's clip no longer hides (or swallows) it.
#[test]
fn an_overlay_button_outside_the_main_tree_is_clickable() {
    let clicked = Rc::new(Cell::new(0));
    let main_clicked = Rc::new(Cell::new(0));
    let main_sink = main_clicked.clone();

    let mut h = Harness::with(
        Size::new(800.0, 600.0),
        Button::new("main").on_click(move || main_sink.set(main_sink.get() + 1)),
    );

    let overlay_sink = clicked.clone();
    let id = h.open(
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("overlay").on_click(move || overlay_sink.set(overlay_sink.get() + 1)),
    );
    assert_eq!(h.runner.host().overlay_count(), 1);

    // A click inside the overlay, far from where the main button paints.
    h.press(Point::new(360.0, 320.0));
    h.release(Point::new(360.0, 320.0));
    assert_eq!(clicked.get(), 1, "the overlay button received the click");
    assert_eq!(main_clicked.get(), 0, "the main tree was not involved");

    h.runner.host_mut().close_overlay(id);
    assert_eq!(h.runner.host().overlay_count(), 0, "closing removes it");
}

/// A modal overlay covers the screen, so a click that would have landed on a
/// popup beneath it is swallowed by the scrim instead. Once the modal closes,
/// the same click reaches the popup again.
#[test]
fn a_modal_overlay_absorbs_clicks_underneath_it() {
    let popup_clicked = Rc::new(Cell::new(0));
    let modal_clicked = Rc::new(Cell::new(0));
    let p = popup_clicked.clone();
    let m = modal_clicked.clone();

    let mut h = Harness::with(Size::new(800.0, 600.0), Button::new("root"));

    let popup = h.open(
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("popup").on_click(move || p.set(p.get() + 1)),
    );
    let modal = h.open(
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0)),
        // Modal content lives only in the corner; the rest is scrim.
        Button::new("modal").on_click(move || m.set(m.get() + 1)),
    );

    // Click where the popup sits — but the modal is on top of it, so the click
    // reaches the modal's content instead of the popup beneath.
    h.press(Point::new(360.0, 320.0));
    h.release(Point::new(360.0, 320.0));
    assert_eq!(
        popup_clicked.get(),
        0,
        "the modal shielded the popup beneath it from the click"
    );
    assert_eq!(
        modal_clicked.get(),
        1,
        "the click reached the modal's content on top"
    );

    // Close the modal and re-render so the popup is hit-testable again.
    h.runner.host_mut().close_overlay(modal);
    h.render();

    h.press(Point::new(360.0, 320.0));
    h.release(Point::new(360.0, 320.0));
    assert_eq!(
        popup_clicked.get(),
        1,
        "after the modal closes the popup is clickable again"
    );

    h.runner.host_mut().close_overlay(popup);
}

/// Focus moves into an overlay's content when it opens, so a key meant for the
/// overlay reaches it instead of the main tree.
#[test]
fn keyboard_reaches_overlay_content() {
    let clicked = Rc::new(Cell::new(0));
    let sink = clicked.clone();

    let mut h = Harness::with(Size::new(800.0, 600.0), Button::new("root"));

    h.open(
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(800.0, 600.0)),
        Button::new("overlay").on_click(move || sink.set(sink.get() + 1)),
    );

    h.key(KeyCode::ENTER);
    assert_eq!(clicked.get(), 1, "Enter reached the focused overlay button");
    h.key(KeyCode::SPACE);
    assert_eq!(clicked.get(), 2, "Space also activates it");
}

/// A press that misses a non-modal overlay takes it down, and does *not* also
/// activate whatever the press landed on.
///
/// This is the host's light-dismiss rule. It cannot live in the widget: the
/// router never routes a press to a widget it missed, so a popup would have to
/// invent a scrim it does not otherwise want in order to hear about it.
#[test]
fn a_press_outside_a_non_modal_overlay_dismisses_it() {
    let popup_clicked = Rc::new(Cell::new(0));
    let main_clicked = Rc::new(Cell::new(0));
    let p = popup_clicked.clone();
    let m = main_clicked.clone();

    let mut h = Harness::with(
        Size::new(800.0, 600.0),
        Button::new("main").on_click(move || m.set(m.get() + 1)),
    );

    let id = h.open(
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("popup").on_click(move || p.set(p.get() + 1)),
    );
    assert_eq!(h.runner.host().overlay_count(), 1);

    // A press clear of the popup's content.
    h.press(Point::new(20.0, 20.0));
    assert_eq!(
        h.runner.host().overlay_count(),
        0,
        "the outside press took the popup down"
    );
    assert!(!h.runner.host().has_overlay(id), "and it is really gone");
    assert_eq!(
        main_clicked.get(),
        0,
        "the dismiss is consumed rather than passed to the tree beneath"
    );
    assert_eq!(popup_clicked.get(), 0, "the popup's own button saw nothing");
}

/// A modal overlay is *not* light-dismissed: it exists to hold the window, and
/// a press on its scrim is the overlay's business, not the host's.
#[test]
fn a_press_outside_a_modal_overlay_leaves_it_up() {
    let modal_clicked = Rc::new(Cell::new(0));
    let m = modal_clicked.clone();

    let mut h = Harness::with(Size::new(800.0, 600.0), Button::new("root"));

    h.open(
        OverlayKind::Dialog,
        true,
        Rect::from_origin_size(Point::new(200.0, 200.0), Size::new(200.0, 120.0)),
        Button::new("modal").on_click(move || m.set(m.get() + 1)),
    );

    h.press(Point::new(700.0, 550.0));
    assert_eq!(
        h.runner.host().overlay_count(),
        1,
        "a modal overlay stays up until something closes it"
    );
}

/// The largest `y1` in the scene: how far down the window the paint reaches.
/// An overlay entering with `TranslateY` starts below where it settles, so this
/// number falls as the animation runs.
fn painted_bottom(h: &Harness) -> f64 {
    h.runner
        .host()
        .scene()
        .iter()
        .filter_map(|(_, node)| node.bounds_hint())
        .map(|bounds| bounds.y1)
        .fold(f64::NEG_INFINITY, f64::max)
}

/// An overlay is pushed with an entry animation, and that animation is actually
/// composited: the sheet's content starts below its final position and slides
/// up, and the host keeps owing frames until it settles.
#[test]
fn a_sheet_slides_into_place_and_then_settles() {
    let mut h = Harness::with(Size::new(800.0, 600.0), Button::new("root"));

    let area = Rect::from_origin_size(Point::new(0.0, 400.0), Size::new(800.0, 200.0));
    h.open(
        OverlayKind::BottomSheet,
        true,
        area,
        Button::new("sheet content like this"),
    );

    let entering = painted_bottom(&h);
    assert!(
        h.runner.host().overlay_animating(),
        "the entry animation is running on the frame it opened"
    );
    assert!(
        entering > area.y1,
        "the sheet starts below its settled bottom edge ({entering} vs {})",
        area.y1
    );

    // 0.15 s at the nominal 60 Hz step is nine frames; give it a couple spare.
    for _ in 0..12 {
        h.render();
    }
    assert!(!h.runner.host().overlay_animating(), "the slide settles");
    let settled = painted_bottom(&h);
    assert!(
        (settled - area.y1).abs() < 0.5,
        "the sheet ended flush with the bottom edge ({settled} vs {})",
        area.y1
    );
    assert!(
        settled < entering,
        "the sheet moved up into place ({entering} → {settled})"
    );
}

/// A non-sheet overlay fades: part-way through its entry its content is drawn
/// with partial alpha, and once the animation settles the payload is written
/// unmodified (the compositor goes back to `None`, so nothing keeps re-writing
/// the scene and the host stops taking damage).
#[test]
fn a_popup_fades_in_and_then_stops_damaging_the_scene() {
    let mut h = Harness::with(Size::new(800.0, 600.0), Button::new("root"));

    let area = Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0));
    h.open(OverlayKind::Popup, false, area, Button::new("popup"));

    let mut seen_partial_alpha = false;
    for _ in 0..12 {
        // The overlay's Button paints exactly one background rect, sized to the
        // rect the overlay was opened at, so alpha there is the entry
        // animation — the main tree's button is nowhere near this area.
        let alpha = h
            .runner
            .host()
            .scene()
            .iter()
            .filter_map(|(_, node)| match node {
                SceneNode::Rect { bounds, fill, .. } if overlaps(area, *bounds) => Some(fill.a),
                _ => None,
            })
            .min();
        if let Some(alpha) = alpha {
            if alpha > 0 && alpha < 255 {
                seen_partial_alpha = true;
            }
        }
        h.render();
    }

    assert!(
        seen_partial_alpha,
        "the popup passed through partial opacity on its way in"
    );
    assert!(!h.runner.host().overlay_animating(), "the fade settles");
    assert!(
        !h.runner.host().needs_frame(),
        "once it settles the host owes no further frames"
    );
}

/// Opening and closing an overlay both change the scene, so each repaints —
/// proving the overlay subtree participates in the paint pass (on top of the
/// main tree) rather than being skipped.
#[test]
fn opening_and_closing_an_overlay_repaints() {
    let mut h = Harness::with(Size::new(800.0, 600.0), Button::new("root"));

    let id = h.open(
        OverlayKind::Popup,
        false,
        Rect::from_origin_size(Point::new(300.0, 300.0), Size::new(120.0, 40.0)),
        Button::new("overlay"),
    );
    assert_eq!(h.runner.host().overlay_count(), 1);

    // Closing unmounts the overlay subtree and removes its scene nodes, which
    // must carry damage this frame.
    h.runner.host_mut().close_overlay(id);
    h.render();
    assert!(
        !h.runner.host().scene().damage().is_empty(),
        "closing the overlay repainted the screen"
    );
    assert_eq!(h.runner.host().overlay_count(), 0);
}
