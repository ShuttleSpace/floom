//! `min_width` / `min_height` end to end, through a real host.
//!
//! A widget-level test can only prove that `min_height(..)` leaves as a
//! `LayoutRequest` field. That is the half that has already gone wrong twice in
//! this port — a `padding` that left as nothing (`Container`), and a `border` whose
//! colour change was reported as "unchanged" because the lossy half was compared.
//! These tests close the loop: the host lays out through taffy, and the box comes
//! out the height it asked for.
//!
//! The claim has two halves and both are load-bearing:
//!
//! * a **floor**: content shorter than the floor still gets the floor, which is
//!   what a list row needs (a touch target is never smaller than 44pt, even when
//!   its text is one line);
//! * and **not a ceiling**: content taller than the floor grows past it, which is
//!   what separates `min_height` from `height`. A test that only checked the first
//!   would pass against `height`, which is exactly the substitution this capability
//!   exists to prevent.

use floom_app::WidgetHost;
use floom_core::{Align, Axis, LayoutRequest};
use floom_layout::LayoutStyle;
use floom_types::{Color, Rect, Size};
use floom_widgets::{Stack, Surface, Text};

/// A surface with `min_height` laid out inside a non-stretching stack, so its own
/// size is the thing under test.
///
/// A root is force-filled to the viewport and cannot grow or shrink (`WidgetHost::
/// run_layout`), so a minimum is only observable on a box that shrink-wraps.
fn row_with_floor(floor: f32, lines: usize) -> Rect {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 400.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(move || {
                let mut box_ = Surface::new()
                    .background(Color::WHITE)
                    .padding_horiz(16.0)
                    .request(floom_core::LayoutRequest::new().min_height(floor));
                // A column, so the lines stack. Pushing them straight onto the
                // surface put them side by side — the surface's own axis is Row —
                // and five lines then fitted in one line's height, which made the
                // "floor, not ceiling" assertion fail for a reason that had nothing
                // to do with the floor.
                let mut column = Stack::new(Axis::Vertical);
                for n in 0..lines {
                    column = column.push(move || Box::new(Text::new(format!("line {n}"))));
                }
                let column = column.clone();
                box_ = box_.push(move || Box::new(column.clone()));
                Box::new(box_)
            }),
    );
    host.frame();
    painted_rect(&host)
}

/// The first filled rect in the scene: the box.
fn painted_rect(host: &WidgetHost) -> Rect {
    use floom_scene::SceneNode;
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { .. } => node.bounds_hint(),
            _ => None,
        })
        .expect("the box painted itself")
}

// --- the floor ------------------------------------------------------------

/// The claim itself: one line of text in a box with no floor is short; the same
/// box with a 56pt floor is 56pt tall.
#[test]
fn a_min_height_raises_a_short_box_to_the_floor() {
    let no_floor = row_with_floor(0.0, 1);
    let floored = row_with_floor(56.0, 1);

    assert!(
        no_floor.height() < 56.0,
        "CONTROL: one line of text with no floor is under 56pt ({}), so the \\
     assertion below is about the floor and not about the text",
        no_floor.height()
    );
    assert!(
        floored.height() >= 56.0,
        "the floor holds the box at 56pt even though its content is shorter: {}",
        floored.height()
    );
}

/// Three floors, three heights. A `min_height` that were secretly a `height` would
/// pass the single-floor test and fail this one, because the heights would not
/// track.
#[test]
fn the_box_height_tracks_the_floor() {
    let small = row_with_floor(40.0, 1);
    let middle = row_with_floor(72.0, 1);
    let large = row_with_floor(120.0, 1);

    assert!(
        small.height() < middle.height() && middle.height() < large.height(),
        "40 -> {} , 72 -> {} , 120 -> {}",
        small.height(),
        middle.height(),
        large.height()
    );
}

// --- and not a ceiling ---------------------------------------------------

/// The half that separates `min_height` from `height`.
///
/// Enough lines to exceed the floor on their own. If `min_height` were lowering to
/// `size.height`, this box would be pinned at the floor and the content would spill
/// — so the test asserts the box grew *past* the floor, and that it grew by more
/// than the floor's worth of extra content.
#[test]
fn a_min_height_is_a_floor_and_not_a_ceiling() {
    let one_line = row_with_floor(40.0, 1);
    let five_lines = row_with_floor(40.0, 5);

    assert!(
        five_lines.height() > one_line.height(),
        "five lines under a 40pt floor are taller than one: {} vs {}",
        five_lines.height(),
        one_line.height()
    );
    assert!(
        five_lines.height() > 40.0,
        "and they exceed the floor rather than being clipped to it: {}",
        five_lines.height()
    );
}

/// The same content under a floor it already exceeds must not change: a floor that
/// added height whenever it was smaller would be a bug wearing a floor's clothes.
#[test]
fn a_floor_below_the_content_adds_nothing() {
    let with_floor = row_with_floor(8.0, 5);
    let with_lower_floor = row_with_floor(0.0, 5);

    assert_eq!(
        with_floor.height(),
        with_lower_floor.height(),
        "a floor under the content's own height is not observable"
    );
}

// --- width ---------------------------------------------------------------

/// The same capability on the other axis, because `min_width` was added in the
/// same place and would otherwise be untested.
#[test]
fn a_min_width_holds_a_narrow_box_open() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .request(floom_core::LayoutRequest::new().min_width(180.0))
                        .push(|| Box::new(Text::new("x"))),
                )
            }),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() >= 180.0,
        "a box with one character in it is held open to 180pt wide: {}",
        bounds.width()
    );
}

/// The control for the width test: without a floor, the same box hugs its one
/// character.
///
/// Mounted as a **child** of a non-stretching stack, because a root is force-filled
/// to the viewport and cannot shrink — so as a root the box came out 400pt wide
/// whatever it asked for, and this assertion was measuring the host rather than the
/// floor. This is the same two-shape trick `surface_padding.rs` uses for the same
/// reason, and the first version of this test got it wrong in exactly the way that
/// trick exists to prevent.
#[test]
fn without_a_min_width_the_box_hugs_its_content() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .push(|| Box::new(Text::new("x"))),
                )
            }),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() < 180.0,
        "CONTROL: no floor, so the box is as narrow as its one character: {}",
        bounds.width()
    );
}

/// The width floor on the same child shape, so the pair differs only in the floor.
#[test]
fn a_min_width_holds_a_narrow_box_open_as_a_child() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .request(LayoutRequest::new().min_width(180.0))
                        .push(|| Box::new(Text::new("x"))),
                )
            }),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() >= 180.0,
        "a box with one character in it is held open to 180pt wide: {}",
        bounds.width()
    );
}

// --- the mount style still wins ------------------------------------------

/// A host that pins a dimension must not be widened by a widget's floor.
///
/// This is the `apply_self_layout` rule, and it is why the floor is carried *inside*
/// the `if base.width.is_none()` branch rather than beside it: a mount style saying
/// "30pt wide" is the caller overriding the widget, and a floor underneath it would
/// silently re-apply the widget's preference.
///
/// The surface is mounted **as the root**, because that is the only arrangement in
/// which the two compete. The first version of this test pinned the style on a
/// wrapping `Stack` instead — so the style applied to the stack, the surface was its
/// child with a request of its own, and nothing was ever in conflict. The box came
/// out 180pt and the test reported it as "the mount style's 30pt stands: 180 wide",
/// which is a sentence that says the opposite of what it means.
#[test]
fn a_mount_style_that_pins_the_dimension_wins_over_the_floor() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    let mut pinned = LayoutStyle::default();
    pinned.width = Some(floom_types::Length::Px(30.0));
    host.mount_root(
        pinned,
        Surface::new()
            .background(Color::WHITE)
            .request(LayoutRequest::new().min_width(180.0))
            .push(|| Box::new(Text::new("x"))),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() <= 30.0,
        "the mount style's 30pt stands, so the widget's 180pt floor does not \
         re-apply underneath it: {} wide",
        bounds.width()
    );
}

/// The control for the assertion above: the same surface with the same floor, and
/// no pinned style, does reach 180. Without this the previous test would also pass
/// on a host that ignored floors entirely.
#[test]
fn without_a_pinned_style_the_same_floor_applies() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(400.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Surface::new()
            .background(Color::WHITE)
            .request(LayoutRequest::new().min_width(180.0))
            .push(|| Box::new(Text::new("x"))),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() >= 180.0,
        "CONTROL: unpinned, the 180pt floor applies: {}",
        bounds.width()
    );
}

// --- the ceiling ----------------------------------------------------------

/// `max_width` is the other half of `min_width`, and it exists because a dialog's
/// card is `width_pct(100.0)` *and* capped — with only `width`, one of the two
/// would have to be dropped. floem-ui asks for it 22 times across 8 crates.
///
/// The content is **wider than the cap**, which is the only shape in which a ceiling
/// is observable at all. The first version of this test used a single character, and
/// a mutation that sized the box *to* its ceiling instead of capping it passed: a
/// one-character box is already narrower than 180pt, so pinning it to 180 still
/// satisfied `<= 180`. A ceiling that cannot be violated cannot be tested.
#[test]
fn a_max_width_caps_content_that_wants_to_be_wider() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(600.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .request(LayoutRequest::new().max_width(180.0))
                        // Comfortably wider than 180pt at this font size.
                        .push(|| Box::new(Text::new("a line of text that would rather not wrap"))),
                )
            }),
    );
    host.frame();
    let capped = painted_rect(&host);
    assert!(
        capped.width() <= 180.0,
        "the cap holds even though the content wants more: {}",
        capped.width()
    );

    // The control: the same content with no ceiling is wider than the cap, so the
    // assertion above is about the ceiling and not about the text.
    let mut host = WidgetHost::new();
    host.set_size(Size::new(600.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .push(|| Box::new(Text::new("a line of text that would rather not wrap"))),
                )
            }),
    );
    host.frame();
    let uncapped = painted_rect(&host);
    assert!(
        uncapped.width() > 180.0,
        "CONTROL: uncapped, the same content is {} wide — so the ceiling above \
         really did something",
        uncapped.width()
    );
}

/// The other half of the pair above, and the half that distinguishes a ceiling from
/// a size.
///
/// With content **wider** than the cap, a ceiling and "size me to the cap" produce
/// the same box — 180pt either way — so the wide-content test cannot tell them
/// apart. With content **narrower** than the cap they differ: a ceiling lets the box
/// shrink to its content, while a size pins it to 180. This is the test that catches
/// that mutation, and it is why both are needed.
#[test]
fn content_narrower_than_the_cap_still_shrinks_the_box() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(600.0, 200.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .request(LayoutRequest::new().max_width(180.0))
                        .push(|| Box::new(Text::new("x"))),
                )
            }),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() < 180.0,
        "a ceiling is not a size: one character in a 180pt-capped box is still \
         {} wide",
        bounds.width()
    );
}

/// The pair that makes it a *cap* rather than a *size*: a floor and a ceiling on the
/// same box, with content between them. This is the dialog's card exactly —
/// `width_pct(100.0)`, a `max_width`, and a `min_height` — and it is the only shape
/// in the engine that needs all four numbers at once.
#[test]
fn a_floor_and_a_ceiling_together_pin_a_band() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(600.0, 400.0));
    host.mount_root(
        LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .align(Align::Start, Align::Start)
            .push(|| {
                let mut column = Stack::new(Axis::Vertical);
                column = column.push(|| Box::new(Text::new("line")));
                let column = column.clone();
                Box::new(
                    Surface::new()
                        .background(Color::WHITE)
                        .request(
                            LayoutRequest::new()
                                .min_width(120.0)
                                .max_width(180.0)
                                .min_height(60.0)
                                .max_height(90.0),
                        )
                        .push(move || Box::new(column.clone())),
                )
            }),
    );
    host.frame();
    let bounds = painted_rect(&host);

    assert!(
        bounds.width() >= 120.0 && bounds.width() <= 180.0,
        "the width sits inside its band: {}",
        bounds.width()
    );
    assert!(
        bounds.height() >= 60.0 && bounds.height() <= 90.0,
        "and so does the height: {}",
        bounds.height()
    );
}
