//! End-to-end interaction tests: real platform events in, real control state
//! out.
//!
//! `widgets/floem-widgets/tests/controls.rs` drives the widgets directly, so
//! it proves each control's own logic. What it cannot prove is the wiring
//! *around* them: that a pointer button the platform reports reaches the
//! widget under the cursor, that a key reaches whatever holds focus at that
//! moment, and that the resulting state change actually reaches the screen
//! instead of stopping inside the widget.
//!
//! These tests therefore go through the whole chain — FakePlatform →
//! AppRunner → WidgetHost → control — driving it with the same
//! `PlatformEvent`s a real windowing backend would deliver. Every interaction
//! asserts on two things: the control's state moved, and the frame that
//! followed had damage to paint.

use std::cell::Cell;
use std::rc::Rc;

use floem_app::{AppRunner, NodeId, WidgetHost};
use floem_core::WidgetAny;
use floem_layout::LayoutStyle;
use floem_platform::{FakePlatform, KeyCode, Platform, PlatformEvent, PointerButton, WindowConfig};
use floem_renderer_software::SoftwarePresenter;
use floem_types::{Modifiers, Point, Size, WindowId};
use floem_widgets::{Checkbox, Select, Slider, Switch, TextField};

/// A window holding one control, pinned to the whole viewport.
///
/// Pinning keeps the arithmetic honest: a click at `x` on a `200px` wide
/// control is unambiguously at fraction `x / 200`, with no layout in between
/// to reason about. Layout is covered by its own tests.
struct Harness {
    platform: FakePlatform,
    runner: AppRunner<SoftwarePresenter>,
    window: WindowId,
}

impl Harness {
    fn with(size: Size, widget: impl WidgetAny + 'static) -> (Self, NodeId) {
        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        let node = host.mount_root(
            LayoutStyle {
                width: Some(size.width as f32),
                height: Some(size.height as f32),
                ..LayoutStyle::default()
            },
            widget,
        );
        let runner = AppRunner::new(host, SoftwarePresenter::new());
        let window = platform.create_window(WindowConfig {
            title: "interaction".into(),
            size,
            resizable: true,
        });
        let mut harness = Self {
            platform,
            runner,
            window,
        };
        // Let the window attach, lay out and paint its first frame.
        for _ in 0..6 {
            harness.runner.pump(&mut harness.platform);
        }
        (harness, node)
    }

    /// Drains the queue — rendering every frame the last interaction asked
    /// for — and reports whether any of them had something to repaint.
    ///
    /// "Any" rather than "the last": a change can ask for a cascade of frames
    /// (new value → repaint → re-measure → re-layout → repaint), and only the
    /// first of those carries damage. What the test wants to know is whether
    /// the interaction reached the screen at all.
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

    fn moved(&mut self, to: Point) -> bool {
        let window = self.window;
        self.runner.handle(
            &mut self.platform,
            PlatformEvent::PointerMoved {
                window,
                position: to,
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

    /// Committed IME text — the path a real keyboard's composed input takes.
    fn commit(&mut self, text: &str) -> bool {
        let window = self.window;
        self.runner.handle(
            &mut self.platform,
            PlatformEvent::ImeCommit {
                window,
                text: text.into(),
            },
        );
        self.frame()
    }

    fn widget<T: WidgetAny + 'static>(&self, node: NodeId) -> &T {
        self.runner
            .host()
            .widget::<T>(node)
            .expect("the control is still mounted")
    }

    fn is_focused(&self, node: NodeId) -> bool {
        let host = self.runner.host();
        host.focused() == Some(host.node_id(node))
    }

    /// A tree that stopped changing stops repainting: after the queue drains,
    /// the last frame had nothing left to do.
    fn settles(&mut self) {
        for _ in 0..8 {
            self.runner.pump(&mut self.platform);
        }
        assert!(
            self.runner.host().scene().damage().is_empty(),
            "a settled tree stops asking for repaints"
        );
    }
}

#[test]
fn dragging_a_slider_moves_its_value_and_repaints() {
    let value = Rc::new(Cell::new(0.0_f64));
    let sink = value.clone();
    let (mut harness, slider) = Harness::with(
        Size::new(200.0, 32.0),
        Slider::new(0.0, 0.0, 100.0).on_change(move |next| sink.set(next)),
    );
    assert_eq!(harness.widget::<Slider>(slider).value(), 0.0);

    // Press three quarters along a 200px track: 75 on a 0..100 range.
    assert!(
        harness.press(Point::new(150.0, 16.0)),
        "the press repainted"
    );
    assert_eq!(harness.widget::<Slider>(slider).value(), 75.0);
    assert_eq!(value.get(), 75.0, "the app's callback saw the drag");

    // Still held: moving keeps following the pointer, no second press needed.
    assert!(harness.moved(Point::new(50.0, 16.0)), "the drag repainted");
    assert_eq!(harness.widget::<Slider>(slider).value(), 25.0);
    assert_eq!(value.get(), 25.0);

    assert!(harness.release(Point::new(50.0, 16.0)));
    assert!(
        harness.is_focused(slider),
        "pressing a slider focuses it, which is what lets the keyboard take over"
    );

    harness.settles();
}

#[test]
fn the_keyboard_moves_a_focused_slider() {
    let (mut harness, slider) = Harness::with(
        Size::new(200.0, 32.0),
        Slider::new(50.0, 0.0, 100.0).step(5.0),
    );

    // A key with nothing focused is nobody's: it must not move the slider.
    assert!(!harness.key(KeyCode::ARROW_RIGHT));
    assert_eq!(harness.widget::<Slider>(slider).value(), 50.0);

    // Press near the left end: the pointer itself sets 0 (quantized to the
    // 5.0 step) and hands over focus.
    harness.press(Point::new(4.0, 16.0));
    harness.release(Point::new(4.0, 16.0));
    assert_eq!(harness.widget::<Slider>(slider).value(), 0.0);

    assert!(harness.key(KeyCode::ARROW_RIGHT));
    assert_eq!(harness.widget::<Slider>(slider).value(), 5.0);

    // A page is ten steps.
    assert!(harness.key(KeyCode::PAGE_UP));
    assert_eq!(harness.widget::<Slider>(slider).value(), 55.0);

    assert!(harness.key(KeyCode::END));
    assert_eq!(harness.widget::<Slider>(slider).value(), 100.0);
}

#[test]
fn typing_reaches_the_focused_text_field() {
    let (mut harness, field) =
        Harness::with(Size::new(200.0, 32.0), TextField::new().placeholder("name"));
    assert_eq!(harness.widget::<TextField>(field).text(), "");

    // Nothing holds focus yet, so the character has nowhere to go.
    assert!(!harness.key(KeyCode::from_char('a')));
    assert_eq!(harness.widget::<TextField>(field).text(), "");

    // Clicking focuses it…
    harness.press(Point::new(100.0, 16.0));
    harness.release(Point::new(100.0, 16.0));
    assert!(
        harness.is_focused(field),
        "pressing a text field focuses it"
    );

    // …and now committed text lands in the value, caret advancing with it.
    assert!(harness.commit("hi"), "typing repainted the field");
    assert_eq!(harness.widget::<TextField>(field).text(), "hi");
    assert_eq!(harness.widget::<TextField>(field).caret(), 2);

    // Backspace removes the character before the caret.
    assert!(harness.key(KeyCode::BACKSPACE));
    assert_eq!(harness.widget::<TextField>(field).text(), "h");
    assert_eq!(harness.widget::<TextField>(field).caret(), 1);

    // A plain character key arrives as text input too, not as a shortcut.
    assert!(harness.key(KeyCode::from_char('i')));
    assert_eq!(harness.widget::<TextField>(field).text(), "hi");

    harness.settles();
}

#[test]
fn a_click_opens_a_select_and_the_keyboard_chooses() {
    let chosen = Rc::new(Cell::new(None::<usize>));
    let sink = chosen.clone();
    let (mut harness, select) = Harness::with(
        Size::new(200.0, 32.0),
        Select::new(["Red", "Green", "Blue"]).on_change(move |index| sink.set(Some(index))),
    );
    assert!(!harness.widget::<Select>(select).is_open());

    // Click the header (above the first row): it opens, and took focus doing so.
    harness.press(Point::new(100.0, 8.0));
    harness.release(Point::new(100.0, 8.0));
    assert!(
        harness.widget::<Select>(select).is_open(),
        "the click opened the list"
    );
    assert!(harness.is_focused(select), "the click focused the select");

    // Arrows walk the options without dismissing the popup.
    assert!(harness.key(KeyCode::ARROW_DOWN));
    assert_eq!(harness.widget::<Select>(select).selected_index(), Some(0));
    assert_eq!(chosen.get(), Some(0), "the app's callback saw the choice");
    assert!(harness.widget::<Select>(select).is_open());

    assert!(harness.key(KeyCode::ARROW_DOWN));
    assert_eq!(harness.widget::<Select>(select).selected_index(), Some(1));
    assert_eq!(chosen.get(), Some(1));

    // Escape closes without disturbing the choice.
    assert!(harness.key(KeyCode::ESCAPE));
    assert!(!harness.widget::<Select>(select).is_open());
    assert_eq!(harness.widget::<Select>(select).selected_index(), Some(1));

    harness.settles();
}

#[test]
fn a_click_toggles_a_checkbox() {
    let toggles = Rc::new(Cell::new(0));
    let reported = Rc::new(Cell::new(None::<bool>));
    let count = toggles.clone();
    let state = reported.clone();
    let (mut harness, checkbox) = Harness::with(
        Size::new(200.0, 32.0),
        Checkbox::new("I agree").on_toggle(move |on| {
            count.set(count.get() + 1);
            state.set(Some(on));
        }),
    );
    assert!(!harness.widget::<Checkbox>(checkbox).is_checked());

    harness.press(Point::new(100.0, 16.0));
    assert!(
        harness.release(Point::new(100.0, 16.0)),
        "the click repainted"
    );
    assert!(
        harness.widget::<Checkbox>(checkbox).is_checked(),
        "the click checked it"
    );
    assert_eq!(toggles.get(), 1, "on_toggle fired exactly once");
    assert_eq!(
        reported.get(),
        Some(true),
        "the callback reports the state it moved to"
    );

    // And back again: the callback sees the way back too.
    harness.press(Point::new(100.0, 16.0));
    harness.release(Point::new(100.0, 16.0));
    assert!(!harness.widget::<Checkbox>(checkbox).is_checked());
    assert_eq!(toggles.get(), 2);
    assert_eq!(reported.get(), Some(false));

    harness.settles();
}

#[test]
fn a_disabled_control_ignores_the_pointer_end_to_end() {
    let toggles = Rc::new(Cell::new(0));
    let count = toggles.clone();
    let (mut harness, checkbox) = Harness::with(
        Size::new(200.0, 32.0),
        Checkbox::new("I agree")
            .disabled(true)
            .on_toggle(move |_| count.set(count.get() + 1)),
    );

    // The whole chain agrees it is disabled: the event is delivered, refused,
    // and nothing changes — so there is nothing to repaint either.
    assert!(!harness.press(Point::new(100.0, 16.0)));
    assert!(!harness.release(Point::new(100.0, 16.0)));
    assert!(!harness.widget::<Checkbox>(checkbox).is_checked());
    assert_eq!(toggles.get(), 0);
}

#[test]
fn a_click_flips_a_switch() {
    let (mut harness, switch) = Harness::with(Size::new(200.0, 32.0), Switch::new("Wi-Fi"));
    assert!(!harness.widget::<Switch>(switch).is_on());

    harness.press(Point::new(100.0, 16.0));
    harness.release(Point::new(100.0, 16.0));
    assert!(
        harness.widget::<Switch>(switch).is_on(),
        "the click flipped it on"
    );

    harness.press(Point::new(100.0, 16.0));
    harness.release(Point::new(100.0, 16.0));
    assert!(!harness.widget::<Switch>(switch).is_on());
}
