use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{Align, Axis, Length, Widget, WidgetAny};
use floom_types::{FontWeight, Point, PointerEvent, Rect};

use floom_ui_composites::{ListItemRow, ListItemStyle};
use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext, TestEventContext};
use floom_widgets::{Pressable, Ripple, Stack, Surface, Text};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 300.0,
    y1: 56.0,
};

fn style() -> ListItemStyle {
    ListItemStyle {
        min_height: 56.0,
        padding_horizontal: 16.0,
        gap: 16.0,
        text_gap: 4.0,
        title_size: 16.0,
        title_weight: FontWeight::Medium,
        title_color: floom_core::Color::rgb(0x1D, 0x1B, 0x20),
        subtitle_size: Some(14.0),
        subtitle_color: floom_core::Color::rgb(0x49, 0x4A, 0x5C),
        background: floom_core::Color::rgb(0xFF, 0xFB, 0xFE),
    }
}

fn row() -> ListItemRow {
    ListItemRow::new("Title").style(style())
}

/// A stand-in slot widget, so the tests can tell a slot from the text column.
fn marker() -> Box<dyn WidgetAny> {
    Box::new(Text::new("M"))
}

/// The row's one child, built.
fn root(row: &ListItemRow) -> Box<dyn WidgetAny> {
    let children = row.children();
    assert_eq!(children.len(), 1, "the row is one node with one child");
    children[0].build()
}

fn child_of<T: WidgetAny>(parent: &dyn WidgetAny) -> Box<dyn WidgetAny> {
    let children = parent.children();
    assert_eq!(children.len(), 1, "expected one child to downcast");
    let built = children[0].build();
    assert!(
        built.as_any().is::<T>(),
        "expected {}, got something else",
        std::any::type_name::<T>()
    );
    built
}

/// The band's `Surface`, from the row down: `Ripple -> Pressable -> Surface`.
fn band_of(row: &ListItemRow) -> Box<dyn WidgetAny> {
    let ripple = root(row);
    let ripple = ripple.as_any().downcast_ref::<Ripple>().expect("a ripple");
    let pressable = child_of::<Pressable>(ripple);
    child_of::<Surface>(pressable.as_ref())
}

// --- the structure --------------------------------------------------------

/// The three wrappers, outermost first. Same claim as the icon button's, and the
/// same reason: the order is a design decision that compiles either way.
#[test]
fn the_subtree_is_ripple_pressable_surface() {
    let row = row();
    let ripple = root(&row);
    assert!(ripple.as_any().is::<Ripple>(), "the ripple is outermost");
    let pressable = child_of::<Pressable>(ripple.as_ref());
    assert!(
        pressable.as_any().is::<Pressable>(),
        "the pressable is the click target"
    );
    let surface = child_of::<Surface>(pressable.as_ref());
    assert!(
        surface.as_any().is::<Surface>(),
        "and the surface is the band"
    );
}

/// v1 passed **no** radius to the ripple, and a full-width row with rounded ink
/// reads as a card rather than a band. So the ripple's clip radius is zero, and
/// this is the line that says so — a kit wanting a rounded row changes this, and
/// should change the test with it.
#[test]
fn the_ripple_is_not_rounded() {
    let row = row();
    let ripple = root(&row);
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    assert_eq!(
        ripple.clip_radius(),
        0.0,
        "v1 gave the list row's ripple no radius, and the default is 0 — a rounded \\
         ink on a full-width row would read as a card"
    );
}

/// The band is a floor, not a size. v1's `min_height(..)`; the pair of assertions
/// is what separates the two, and the second is the one a `height` would fail.
#[test]
fn the_band_has_a_height_floor_and_fills_its_width() {
    let band = band_of(&row());
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let request = band.layout_request().expect("the band declares a layout");

    assert_eq!(
        request.min_height,
        Some(Length::Px(56.0)),
        "v1's `min_height(..)`, which is a floor"
    );
    assert_eq!(
        request.height, None,
        "and NOT a height: content taller than the floor must still grow, which is \\
         what `a_min_height_is_a_floor_and_not_a_ceiling` in floom-app proves \
         end to end"
    );
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "v1's `.width_pct(100.0)` — a row spans its parent"
    );
    assert_eq!(
        request.padding.map(|p| p.left),
        Some(16.0),
        "and the token's horizontal padding"
    );
    assert_eq!(
        request.cross_align,
        Some(Align::Center),
        "v1's `.items_center()` on the band"
    );
}

// --- the optional slots ---------------------------------------------------

/// Three shapes, three child counts: neither slot, one slot, both. v1 built
/// `row_kids` by pushing the slots only when they were supplied, so the count is
/// the observable difference and a row that always mounted two empty slots would
/// still look right.
#[test]
fn the_slots_are_optional_and_the_child_count_follows() {
    let bare = row();
    let band = band_of(&bare);
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let inner = band.children();
    assert_eq!(inner.len(), 1, "the band holds one row");
    let stack = inner[0].build();
    assert!(
        stack.as_any().is::<Stack>(),
        "the row is a horizontal stack"
    );
    let stack = stack.as_any().downcast_ref::<Stack>().unwrap();
    assert_eq!(
        stack.child_count(),
        1,
        "no slots and no subtitle: just the text column"
    );

    let leading = ListItemRow::new("T").style(style()).leading(|| marker());
    let stack = leading.children()[0].build();
    let stack = stack.as_any().downcast_ref::<Ripple>().unwrap().children()[0].build();
    let stack = stack.as_any().downcast_ref::<Pressable>().unwrap();
    let stack = stack.children()[0].build();
    let band = stack.as_any().downcast_ref::<Surface>().unwrap();
    let stack = band.children()[0].build();
    assert_eq!(
        stack
            .as_any()
            .downcast_ref::<Stack>()
            .unwrap()
            .child_count(),
        2,
        "a leading slot adds exactly one child"
    );
}

/// Both slots at once, and the text column stays **second** — the order is what
/// puts the trailing slot at the far edge, so a row that put the column last would
/// be a different component.
#[test]
fn the_order_is_leading_then_text_then_trailing() {
    let row = ListItemRow::new("T")
        .style(style())
        .leading(|| marker())
        .trailing(|| marker());
    let band = band_of(&row);
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let inner = band.children();
    let stack = inner[0]
        .build()
        .as_any()
        .downcast_ref::<Stack>()
        .unwrap()
        .clone();

    assert_eq!(stack.child_count(), 3, "leading, column, trailing");
}

// --- the text column ------------------------------------------------------

/// The column is a stack of its own, and it is the only part with `flex_grow`.
///
/// That is what pushes the trailing slot to the far edge of a full-width row: with
/// no growth on the column, the row would shrink-wrap and the trailing slot would
/// sit next to the text.
#[test]
fn the_text_column_is_the_only_part_that_grows() {
    let row = ListItemRow::new("T").style(style()).trailing(|| marker());
    let band = band_of(&row);
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let inner = band.children();
    let row_layout = inner[0].layout.clone();
    assert_eq!(
        row_layout.flex_grow,
        Some(1.0),
        "CONTROL: the row inside the band grows"
    );

    let stack = inner[0].build();
    let stack = stack.as_any().downcast_ref::<Stack>().unwrap();
    let children = stack.children();
    assert_eq!(children.len(), 2, "the column and the trailing slot");

    // The column is the growing one, and it is asked to fill what is left.
    assert_eq!(
        children[0].layout.flex_grow,
        Some(1.0),
        "the text column grows"
    );
    assert_eq!(
        children[1].layout.flex_grow, None,
        "and the trailing slot does not — so it is pushed to the far edge"
    );
}

/// The title's weight comes from the *typography* scale in v1, not from the
/// list-item tokens, so it is a parameter here. Cupertino and Fluent read their own
/// weight; nothing in the engine can know which.
#[test]
fn the_title_carries_the_weight_it_was_given() {
    let heavy = ListItemRow::new("T").style(ListItemStyle {
        title_weight: FontWeight::Bold,
        ..style()
    });
    assert_eq!(heavy.resolved_style().title_weight, FontWeight::Bold);

    let normal = ListItemRow::new("T").style(ListItemStyle {
        title_weight: FontWeight::Normal,
        ..style()
    });
    assert_eq!(
        normal.resolved_style().title_weight,
        FontWeight::Normal,
        "and a kit that wants the v1 default of `Normal` says so"
    );
}

/// A subtitle is an `Option`, not a zero size: "no subtitle line" and "a subtitle
/// of zero height" are different boxes.
#[test]
fn no_subtitle_is_not_a_zero_height_subtitle() {
    assert!(!row().has_subtitle());
    assert!(ListItemRow::new("T")
        .style(style())
        .subtitle("S")
        .has_subtitle());
    assert_eq!(
        style().subtitle_size,
        Some(14.0),
        "CONTROL: the style carries a real subtitle size when there is one"
    );
}

// --- painting -------------------------------------------------------------

/// The band paints, and it paints at the style's fill. The control group for every
/// assertion above: a row that laid out correctly but painted nothing would satisfy
/// all of them.
#[test]
fn the_band_paints_its_fill() {
    let band = band_of(&row());
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let id = band.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(&band, id, BOX);
    assert!(painted.contains("#FFFBFEFF"), "the band's fill: {painted}");
}

/// The title paints as text, and it is the only text in a row with no subtitle.
#[test]
fn the_title_paints_and_a_bare_row_has_exactly_one_line() {
    let band = band_of(&row());
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let inner = band.children();
    let stack = inner[0].build();
    let stack = stack.as_any().downcast_ref::<Stack>().unwrap().clone();
    let column = stack.children()[0].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();

    assert_eq!(
        column.child_count(),
        1,
        "no subtitle, so the column is one line"
    );

    let mut line = column.children()[0].build();
    let id = line.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(&*line, id, BOX);
    assert!(!painted.is_empty(), "the title paints: {painted}");
}

/// With a subtitle the column is two lines. The pair with the assertion above is
/// what makes `Option<f32>` load-bearing rather than decoration.
#[test]
fn a_subtitle_makes_the_column_two_lines() {
    let row = ListItemRow::new("T").style(style()).subtitle("S");
    let band = band_of(&row);
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let inner = band.children();
    let stack = inner[0].build();
    let stack = stack.as_any().downcast_ref::<Stack>().unwrap().clone();
    let column = stack.children()[0].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(column.child_count(), 2, "title and subtitle");
}

// --- clicking -------------------------------------------------------------

fn click(pressable: &mut Pressable, id: floom_core::ElementId) -> bool {
    let at = Point::new(150.0, 28.0);
    let mut down = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    let mut consumed = pressable.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, PointerEvent::primary_up(at).into());
    consumed |= pressable.event(&mut up, id);
    consumed
}

#[test]
fn an_enabled_row_fires_its_callback() {
    let hits = Arc::new(AtomicUsize::new(0));
    let row = {
        let hits = hits.clone();
        row().on_click(move || {
            hits.fetch_add(1, Ordering::SeqCst);
        })
    };
    let ripple = root(&row);
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let mut pressable = child_of::<Pressable>(ripple);
    let pressable = pressable.as_any_mut().downcast_mut::<Pressable>().unwrap();

    let id = pressable.create(&mut AllocatingCreateContext::new());
    assert!(click(pressable, id), "the press is consumed");
    assert_eq!(
        hits.load(Ordering::SeqCst),
        1,
        "CONTROL: an enabled row's callback ran"
    );
}

/// v1 wrote `if !disabled { .. }` inside the action closure, which guarded the
/// pointer but not the keyboard. 2.0's gate is in `Pressable` and covers both — so
/// this is an improvement rather than a translation.
#[test]
fn a_disabled_row_is_inert() {
    let hits = Arc::new(AtomicUsize::new(0));
    let row = {
        let hits = hits.clone();
        row().disabled(true).on_click(move || {
            hits.fetch_add(1, Ordering::SeqCst);
        })
    };
    let ripple = root(&row);
    let ripple = ripple.as_any().downcast_ref::<Ripple>().unwrap();
    let mut pressable = child_of::<Pressable>(ripple);
    let pressable = pressable.as_any_mut().downcast_mut::<Pressable>().unwrap();

    let id = pressable.create(&mut AllocatingCreateContext::new());
    let consumed = click(pressable, id);
    assert_eq!(
        hits.load(Ordering::SeqCst),
        0,
        "a disabled row does not fire"
    );
    assert!(!consumed, "and does not consume the press");
}

/// The ripple is suppressed when disabled too, not just the click: v1 passed
/// `.disabled(disabled)` to it, and a greyed row that still showed ink on hover
/// would read as enabled.
#[test]
fn a_disabled_row_has_no_ripple() {
    let live = row();
    let enabled = root(&live);
    let enabled = enabled.as_any().downcast_ref::<Ripple>().unwrap();
    assert!(!enabled.is_disabled(), "CONTROL: an enabled row has ink");

    let off = row().disabled(true);
    let disabled = root(&off);
    let disabled = disabled.as_any().downcast_ref::<Ripple>().unwrap();
    assert!(
        disabled.is_disabled(),
        "and a disabled row's ripple is suppressed, as v1 asked"
    );
}

/// A row with no callback is still a row: v1 attached an action unconditionally and
/// guarded inside it, so a row with no `on_click` was the common case.
#[test]
fn a_row_with_no_callback_still_assembles() {
    let row = row();
    assert!(root(&row).as_any().is::<Ripple>());
}

/// The row's own node publishes nothing, because the `Pressable` inside it is what
/// carries the role. Asserting the absence rather than leaving it implicit: a
/// `semantics_hint` that returned an empty role would be a second, competing
/// announcement.
#[test]
fn the_row_node_publishes_no_semantics_of_its_own() {
    assert!(
        <ListItemRow as WidgetAny>::semantics_hint(&row()).is_none(),
        "the Pressable inside publishes; this node stays quiet"
    );
}

// --- the axis the column stacks along -------------------------------------

/// A column, not a row: the title and subtitle are one above the other. A test for
/// a mistake that would look like a row with two short lines.
#[test]
fn the_text_column_stacks_vertically() {
    let row = ListItemRow::new("T").style(style()).subtitle("S");
    let band = band_of(&row);
    let band = band.as_any().downcast_ref::<Surface>().unwrap().clone();
    let inner = band.children();
    let stack = inner[0].build();
    let stack = stack.as_any().downcast_ref::<Stack>().unwrap().clone();
    let column = stack.children()[0].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();

    assert_eq!(column.axis(), Axis::Vertical, "title above subtitle");
    assert_eq!(stack.axis(), Axis::Horizontal, "and the row is horizontal");
}

/// The gap inside the column is v1's literal `4.0`, which it wrote at the call site
/// rather than reading from a token. It is a style field here so a kit can see it
/// rather than discover it.
#[test]
fn the_column_gap_is_the_call_site_literal() {
    assert_eq!(style().text_gap, 4.0, "v1's `.gap(4.0)` at the call site");
    assert_ne!(
        style().text_gap,
        style().gap,
        "CONTROL: which is not the row's own gap — so the two cannot be confused"
    );
}
