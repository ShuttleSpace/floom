//! The ported Cupertino chip group -- a row of chips whose **ink** follows the
//! choice.

use floom_core::{Child, ChildKey, LayoutRequest, Point, PointerEvent, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_ui_composites::{ContentPlacement, SelectGroup};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};
use floom_widgets::Pressable;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_chip_group::{CupertinoChipGroup, CupertinoChipGroupStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 80.0,
};

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 300.0,
    y1: 48.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn group() -> CupertinoChipGroup {
    CupertinoChipGroup::new().with_theme(theme())
}

/// Theme colour -> engine colour.
fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// The chip's own box: the row holds a pressable, the pressable holds the
/// surface, and the surface holds the label.
fn chip_of(group: &SelectGroup, index: usize) -> Box<dyn WidgetAny> {
    let pressable = group.children()[index].build().children().remove(0).build();
    pressable.children().remove(0).build()
}

/// The fill and the ink one chip actually painted.
fn chip_painted(group: &SelectGroup, index: usize) -> (Option<Color>, Option<Color>) {
    let surface = chip_of(group, index);

    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();
    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });

    let label = surface.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    let ink = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });

    (fill, ink)
}

/// The text and the size one chip's label painted.
fn chip_label(group: &SelectGroup, index: usize) -> (String, f32) {
    let surface = chip_of(group, index);
    let label = surface.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, size, .. } => Some((text.clone(), *size)),
            _ => None,
        })
        .expect("a chip paints its label")
}

/// Taps a chip, the way the host would.
fn tap(group: &SelectGroup, index: usize) {
    let mut mounted = group.children()[index].build().children().remove(0).build();
    let pressable = mounted
        .as_any_mut()
        .downcast_mut::<Pressable>()
        .expect("a chip's slot mounts a pressable");
    let mut pressable = std::mem::replace(
        pressable,
        Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(floom_widgets::Text::new(""))
        })),
    );
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, ROW, phase.into());
        pressable.event(&mut cx, id);
    }
}

// --- the translation ------------------------------------------------------

/// The whole of this port: the tokens and the colours in the engine's
/// vocabulary, with the shape of a chip and v1's rules attached.
#[test]
fn the_tokens_and_colours_reach_the_engine_style() {
    let style = group().select_group_style();
    let resolved = group().resolved_style();

    assert_eq!(style.gap, resolved.gap, "the token's gap");
    assert_eq!(style.pad, resolved.padding_horizontal, "v1's 12");
    assert_eq!(style.radius, resolved.border_radius, "v1's 16");
    assert_eq!(style.height, Some(32.0), "v1's 32");
    assert_eq!(style.border_width, 0.0, "v1's chip had no border");
    assert_eq!(
        style.axis,
        floom_core::Axis::Horizontal,
        "a chip group is a row of chips"
    );
    assert_eq!(
        style.content_placement,
        ContentPlacement::Centred,
        "v1 centred the label in the chip"
    );
    assert_eq!(
        style.background,
        paint(resolved.background),
        "the unchosen fill"
    );
    assert_eq!(
        style.foreground,
        paint(resolved.foreground),
        "the unchosen ink"
    );
    assert_eq!(
        style.selected_background,
        paint(resolved.selected_background),
        "the chosen fill"
    );
    assert_eq!(
        style.selected_foreground,
        paint(resolved.selected_foreground),
        "the chosen ink"
    );
}

/// The numbers v1 wrote as literals, rather than as tokens.
#[test]
fn the_numbers_v1_wrote_as_literals_are_literals_here() {
    let resolved = group().resolved_style();
    assert_eq!(resolved.height, 32.0, "v1's `.height(32.0)`");
    assert_eq!(
        resolved.padding_horizontal, 12.0,
        "v1's `.padding_horiz(12.0)`"
    );
    assert_eq!(resolved.border_radius, 16.0, "v1's `.border_radius(16.0)`");
    assert_eq!(resolved.font_size, 13.0, "v1's `.font_size(13.0)`");
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(group().resolved_style().design_system, "cupertino");
}

/// The style struct is the port's whole output, so a caller can build the
/// widget by hand from it -- which is what `view` does internally.
#[test]
fn the_resolved_style_is_enough_to_build_the_widget() {
    let resolved: CupertinoChipGroupStyle = group().resolved_style();
    assert!(resolved.gap > 0.0);

    let by_hand =
        floom_ui_composites::SelectGroup::new().style(floom_ui_composites::SelectGroupStyle {
            gap: resolved.gap,
            axis: floom_core::Axis::Horizontal,
            height: Some(resolved.height),
            pad: resolved.padding_horizontal,
            content_placement: ContentPlacement::Centred,
            toggles_off_in_single_mode: true,
            mandatory_in_multiple_mode: false,
            radius: resolved.border_radius,
            border_width: 0.0,
            background: paint(resolved.background),
            foreground: paint(resolved.foreground),
            border_color: paint(resolved.foreground),
            selected_background: paint(resolved.selected_background),
            selected_foreground: paint(resolved.selected_foreground),
            selected_border_color: paint(resolved.selected_foreground),
        });
    let by_port = group().select_group_style();
    assert_eq!(by_hand.declared_style(), by_port);
}

// --- the pixels -----------------------------------------------------------

/// The chip that is not chosen: `surface_variant` with `on_surface` ink.
#[test]
fn the_unchosen_chip_paints_the_unchosen_appearance() {
    let resolved = group().resolved_style();
    let group = group().chip("One").view();
    let (fill, ink) = chip_painted(&group, 0);
    assert_eq!(fill, Some(paint(resolved.background)), "the fill");
    assert_eq!(ink, Some(paint(resolved.foreground)), "and the ink");
}

/// The chosen chip: `secondary_container` with `on_secondary_container` ink.
///
/// The ink is the point. v1 wrote `.color(fg)` on the chip's *box*, so the
/// label changed with the choice and not only the fill. 2.0's `Surface` paints
/// a fill and only the child knows it is text, so the content builder has to
/// be able to see the choice -- which is what the engine's `row_with_state` is
/// for.
#[test]
fn the_chosen_chip_paints_the_chosen_appearance() {
    let resolved = group().resolved_style();
    let group = group().chip("One").chip("Two").view();
    tap(&group, 0);

    let (fill, ink) = chip_painted(&group, 0);
    assert_eq!(fill, Some(paint(resolved.selected_background)), "the fill");
    assert_eq!(
        ink,
        Some(paint(resolved.selected_foreground)),
        "and the ink follows the choice"
    );

    let (fill, ink) = chip_painted(&group, 1);
    assert_eq!(fill, Some(paint(resolved.background)), "the other chip");
    assert_eq!(ink, Some(paint(resolved.foreground)), "is untouched");
}

/// The chip's own text, at v1's size.
#[test]
fn the_chip_paints_its_own_label_at_v1_s_size() {
    let group = group().chip("One").view();
    let (text, size) = chip_label(&group, 0);
    assert_eq!(text, "One");
    assert_eq!(size, 13.0, "v1 wrote `.font_size(13.0)`");
}

// --- the rules ------------------------------------------------------------

/// A chip can be tapped off; a radio cannot. v1's chip group did.
#[test]
fn the_chosen_chip_comes_off_when_it_is_tapped_again() {
    let group = group().chip("One").chip("Two").view();
    tap(&group, 0);
    assert_eq!(group.selection(), vec![0], "the first tap chooses it");
    tap(&group, 0);
    assert!(
        group.selection().is_empty(),
        "and the second tap takes it off, the way v1's chip did"
    );
}

/// Unless it is mandatory.
#[test]
fn a_mandatory_chip_stays_chosen() {
    let group = group().chip("One").chip("Two").mandatory(true).view();
    tap(&group, 0);
    tap(&group, 0);
    assert_eq!(group.selection(), vec![0], "v1's `cur && mandatory` guard");
}

/// Several at once, when `multiple`.
#[test]
fn several_chips_may_be_chosen_at_once() {
    let group = group()
        .chip("One")
        .chip("Two")
        .chip("Three")
        .multiple(true)
        .view();
    tap(&group, 0);
    tap(&group, 2);
    assert_eq!(group.selection(), vec![0, 2]);
}

/// v1 read `mandatory` only in the single-mode branch, so in multiple mode it
/// did nothing: the last chosen chip still came off. The engine is told the
/// same thing, so the rule survives the port.
#[test]
fn mandatory_does_nothing_in_multiple_mode_as_it_did_in_v1() {
    let group = group()
        .chip("One")
        .chip("Two")
        .multiple(true)
        .mandatory(true)
        .view();
    tap(&group, 0);
    tap(&group, 0);
    assert!(
        group.selection().is_empty(),
        "v1's chip group let the last one go"
    );
}

/// One at a time, when not `multiple`.
#[test]
fn one_chip_at_a_time_by_default() {
    let group = group().chip("One").chip("Two").view();
    tap(&group, 0);
    tap(&group, 1);
    assert_eq!(group.selection(), vec![1], "not [0, 1]");
}

// --- the callback ---------------------------------------------------------

/// The callback reports the chosen chips.
#[test]
fn the_change_callback_reports_the_chosen_chips() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let group = group()
        .chip("One")
        .chip("Two")
        .on_change(move |indices| s.lock().unwrap().push(indices))
        .view();

    tap(&group, 1);
    assert_eq!(seen.lock().unwrap().as_slice(), &[vec![1]]);

    // One at a time, so the second tap *replaces* the choice rather than adding
    // to it -- v1 cleared every other chip before setting this one.
    tap(&group, 0);
    assert_eq!(seen.lock().unwrap().as_slice(), &[vec![1], vec![0]]);
}

/// The one difference that is left, pinned: v1 reported the selection after
/// *every* click, including a click that changed nothing, so tapping the
/// chosen chip of a `mandatory` group reported the same indices again. The
/// engine reports a *change*, so that click reports nothing.
#[test]
fn a_mandatory_chip_that_changes_nothing_reports_nothing() {
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let s = seen.clone();
    let group = group()
        .chip("One")
        .chip("Two")
        .mandatory(true)
        .on_change(move |indices| s.lock().unwrap().push(indices))
        .view();

    tap(&group, 0);
    assert_eq!(seen.lock().unwrap().as_slice(), &[vec![0]]);

    tap(&group, 0);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[vec![0]],
        "v1 reported the same indices again; the engine reports a change"
    );
}

/// The props report the shape, and not the callback: the kit holds that as an
/// `Arc` of its own, because 2.0's slots are `Send + Sync` and the props'
/// `Box<dyn Fn(..)>` is not.
#[test]
fn the_props_report_the_shape_and_not_the_callback() {
    let group = group().multiple(true).mandatory(true);
    assert!(group.props().multiple);
    assert!(group.props().mandatory);
    assert!(group.props().on_change.is_none());
}
