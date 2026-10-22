//! The pointer cursor, from the widget that asks for it to the shape the window
//! shows.
//!
//! These drive a real [`AppRunner`] over [`FakePlatform`] rather than a
//! `TestApp`, because the cursor is a *window*-level answer: nothing in the
//! widget tree observes it, and the runner is what turns a pointer move into a
//! `Window::set_cursor` call. A test that only asked the widgets what they wanted
//! would pass with the whole chain missing.
//!
//! The positions below are found by sweeping rather than hard-coded. A test that
//! guessed where a button lands would break the day layout changes, and worse,
//! would fail for a reason that has nothing to do with the cursor.

use std::cell::Cell;
use std::rc::Rc;

use floom_app::{AppRunner, WidgetHost};
use floom_core::WidgetAny;
use floom_layout::LayoutStyle;
use floom_platform::{FakePlatform, Modifiers, Platform, PlatformEvent, WindowConfig};
use floom_renderer_software::SoftwarePresenter;
use floom_types::{CursorStyle, Point, Size};
use floom_widgets::{Button, Text};

const W: f64 = 300.0;
const H: f64 = 100.0;

fn window() -> WindowConfig {
    WindowConfig {
        title: "cursor-test".into(),
        size: Size::new(W, H),
        resizable: true,
    }
}

/// Mounts `widget` as the root and settles the frame loop.
fn runner_with<W2: WidgetAny + 'static>(
    widget: W2,
) -> (
    FakePlatform,
    AppRunner<SoftwarePresenter>,
    floom_types::WindowId,
) {
    let mut platform = FakePlatform::new();
    let id = platform.create_window(window());
    let mut host = WidgetHost::new();
    host.mount_root(LayoutStyle::default(), widget);
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());
    for _ in 0..4 {
        runner.pump(&mut platform);
    }
    (platform, runner, id)
}

/// Moves the pointer to `at` and settles, then reads what the window shows.
fn hover(
    platform: &mut FakePlatform,
    runner: &mut AppRunner<SoftwarePresenter>,
    window: floom_types::WindowId,
    at: Point,
) -> Option<CursorStyle> {
    runner.handle(
        platform,
        PlatformEvent::PointerMoved {
            window,
            position: at,
            modifiers: Modifiers::NONE,
        },
    );
    for _ in 0..4 {
        runner.pump(platform);
    }
    platform.cursor(window)
}

/// Sweeps the pointer across the window a row at a time and returns every
/// distinct shape the window showed together with one point that produced it.
///
/// Positions are *found*, not assumed: a centred button, a hugging button and a
/// full-width button all answer at different points, and a test that hard-coded
/// one of those would fail for a reason that has nothing to do with the cursor.
fn sweep(
    platform: &mut FakePlatform,
    runner: &mut AppRunner<SoftwarePresenter>,
    window: floom_types::WindowId,
) -> Vec<(CursorStyle, Point)> {
    let mut seen: Vec<(CursorStyle, Point)> = Vec::new();
    for row in 1..10 {
        let y = H * f64::from(row) / 10.0;
        for col in 1..30 {
            let x = W * f64::from(col) / 30.0;
            let at = Point::new(x, y);
            if let Some(cursor) = hover(platform, runner, window, at) {
                if !seen.iter().any(|(c, _)| *c == cursor) {
                    seen.push((cursor, at));
                }
            }
        }
    }
    seen
}

/// One point that produced `wanted`, if the sweep found one.
fn point_showing(found: &[(CursorStyle, Point)], wanted: CursorStyle) -> Option<Point> {
    found.iter().find(|(c, _)| *c == wanted).map(|(_, p)| *p)
}

/// The load-bearing half of the feature: a control under the pointer gets the
/// shape it asked for.
#[test]
fn hovering_a_button_offers_a_pointer() {
    let (mut platform, mut runner, win) = runner_with(Button::new("Save"));

    let seen = sweep(&mut platform, &mut runner, win);

    assert!(
        point_showing(&seen, CursorStyle::Pointer).is_some(),
        "the pointer appeared somewhere over the button; got {seen:?}"
    );
}

/// **The failure everyone notices.** A cursor that sticks is worse than one that
/// never appeared: the arrow keeps waving over dead background after the pointer
/// has left the control. The sweep finds a point that showed `Pointer` and a
/// point that showed `Default`, so this does not assume where the button sits.
#[test]
fn moving_off_the_control_puts_the_arrow_back() {
    let (mut platform, mut runner, win) = runner_with(Button::new("Save"));

    let found = sweep(&mut platform, &mut runner, win);
    let on = point_showing(&found, CursorStyle::Pointer).expect("the button offered a pointer");
    let off = point_showing(&found, CursorStyle::Default).expect("there is empty space");

    assert_eq!(
        hover(&mut platform, &mut runner, win, on),
        Some(CursorStyle::Pointer),
        "over the control"
    );
    assert_eq!(
        hover(&mut platform, &mut runner, win, off),
        Some(CursorStyle::Default),
        "empty space gets the platform arrow, not the control's shape"
    );
    // And back over the control, so the reset was not merely "whatever came
    // last" -- the cursor has to follow the pointer both ways.
    assert_eq!(
        hover(&mut platform, &mut runner, win, on),
        Some(CursorStyle::Pointer),
        "the cursor tracks the pointer back onto the control"
    );
    // Off again, which is the actual regression this test exists for: a cursor
    // that can go on but not off.
    assert_eq!(
        hover(&mut platform, &mut runner, win, off),
        Some(CursorStyle::Default),
        "and off again"
    );
}

/// A control that cannot be clicked must not wave at you.
#[test]
fn a_disabled_control_does_not_offer_a_pointer() {
    let (mut platform, mut runner, win) = runner_with(Button::new("Save").disabled(true));

    let found = sweep(&mut platform, &mut runner, win);

    assert!(
        point_showing(&found, CursorStyle::Pointer).is_none(),
        "a pointer over something unclickable is a promise it cannot keep; got {found:?}"
    );
    assert!(
        point_showing(&found, CursorStyle::Default).is_some(),
        "and the arrow is what it offered instead; got {found:?}"
    );
}

/// A widget that asks for nothing gets the arrow, and the runner tells the window
/// so rather than leaving the platform to guess.
#[test]
fn a_widget_that_asks_for_nothing_gets_the_arrow() {
    let (mut platform, mut runner, win) = runner_with(Text::new("just words"));

    let found = sweep(&mut platform, &mut runner, win);

    assert_eq!(
        found.len(),
        1,
        "one shape was ever asked for; got {found:?}"
    );
    assert_eq!(found[0].0, CursorStyle::Default);
}

/// The skip: a pointer move arrives per motion event and setting a shape is a
/// platform round trip, so `WindowRecord` remembers the last answer and the
/// runner only pushes a change.
///
/// Asserted on the *count*, not on the shape: a runner that pushed every move
/// would leave the visible cursor correct and still be paying a platform call
/// per motion event, and a shape assertion cannot see that. (This test caught
/// exactly that when the first version of it did assert the shape.)
#[test]
fn an_unchanged_answer_is_not_pushed_again() {
    let (mut platform, mut runner, win) = runner_with(Button::new("Save"));
    let found = sweep(&mut platform, &mut runner, win);
    let on = point_showing(&found, CursorStyle::Pointer).expect("the button offered a pointer");
    let off = point_showing(&found, CursorStyle::Default).expect("there is empty space");

    hover(&mut platform, &mut runner, win, on);
    let after_on = platform.cursor_calls(win);

    for _ in 0..8 {
        hover(&mut platform, &mut runner, win, on);
    }
    assert_eq!(
        platform.cursor_calls(win),
        after_on,
        "eight more moves over the same control pushed nothing new"
    );

    // A change does go through, and then stops again.
    hover(&mut platform, &mut runner, win, off);
    assert_eq!(
        platform.cursor_calls(win),
        after_on + 1,
        "leaving the control is a change, so it is pushed once"
    );
    for _ in 0..4 {
        hover(&mut platform, &mut runner, win, off);
    }
    assert_eq!(
        platform.cursor_calls(win),
        after_on + 1,
        "and then it settles again"
    );
}

/// Two controls with different answers, so the runner is reading the widget under
/// the pointer rather than remembering the first one it ever saw.
#[test]
fn each_control_under_the_pointer_answers_for_itself() {
    // A label anywhere in the window: the arrow, everywhere.
    let (mut platform, mut runner, win) = runner_with(Text::new("plain text"));
    for row in 1..10 {
        for col in 1..30 {
            let at = Point::new(W * f64::from(col) / 30.0, H * f64::from(row) / 10.0);
            assert_eq!(
                hover(&mut platform, &mut runner, win, at),
                Some(CursorStyle::Default),
                "a label asks for nothing, at {at:?} included"
            );
        }
    }

    // A button: the pointer, somewhere. Same window size, same runner, same
    // platform -- so the difference came from the widget and nothing else.
    let (mut platform2, mut runner2, win2) = runner_with(Button::new("Save"));
    assert!(
        point_showing(
            &sweep(&mut platform2, &mut runner2, win2),
            CursorStyle::Pointer
        )
        .is_some(),
        "the button asked for the pointer"
    );
}

/// The widget-level half, on its own: what each control claims, asked directly.
/// Cheap, and it localises a failure to "the control" rather than "the chain".
#[test]
fn each_control_claims_the_shape_its_kind_implies() {
    fn cursor_of<W2: WidgetAny + 'static>(widget: W2) -> CursorStyle {
        widget.cursor()
    }
    use floom_core::Widget;

    assert_eq!(
        cursor_of(Button::new("b")),
        CursorStyle::Pointer,
        "a button acts on a click"
    );
    assert_eq!(
        cursor_of(Button::new("b").disabled(true)),
        CursorStyle::Default,
        "unless it is disabled"
    );
}

/// The callback plumbing is untouched by any of this: a button still fires.
#[test]
fn a_button_still_activates_while_its_cursor_is_being_tracked() {
    let clicks = Rc::new(Cell::new(0));
    let sink = clicks.clone();
    let (mut platform, mut runner, win) = runner_with(Button::new("Save").on_click(move || {
        sink.set(sink.get() + 1);
    }));

    // Click wherever the sweep says the button actually is.
    let found = sweep(&mut platform, &mut runner, win);
    let on = point_showing(&found, CursorStyle::Pointer).expect("the button offered a pointer");
    // The sweep ended over empty space, so go back onto the button before
    // reading the window -- otherwise this asserts the wrong moment.
    hover(&mut platform, &mut runner, win, on);
    assert_eq!(platform.cursor(win), Some(CursorStyle::Pointer));

    runner.handle(
        &mut platform,
        PlatformEvent::PointerButton {
            window: win,
            position: on,
            button: floom_platform::PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    );
    runner.handle(
        &mut platform,
        PlatformEvent::PointerButton {
            window: win,
            position: on,
            button: floom_platform::PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        },
    );
    for _ in 0..4 {
        runner.pump(&mut platform);
    }

    assert_eq!(clicks.get(), 1, "the click still reached the widget");
}
