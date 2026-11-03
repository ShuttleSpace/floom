//! The ported Fluent rating -- a row of stars, the first `value` of them

use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};
use floom_widgets::Stack;

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_rating::{FluentRating, FluentRatingStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentRating {
    FluentRating::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn star_inks(row: &Stack) -> Vec<Option<Color>> {
    let mut out = Vec::new();
    for star in row.children() {
        let pressable = star.build();
        let slot = pressable.children().remove(0).build();
        let mut ink = None;
        for conditional in slot.children() {
            let mut conditional = conditional.build();
            let mut create = AllocatingCreateContext::new();
            let id: ElementId = conditional.create(&mut create);
            let mut cx = RecordingUpdateContext::new();
            conditional.update(&mut cx, id);
            for glyph in conditional.children() {
                let widget = glyph.build();
                let mut create = AllocatingCreateContext::new();
                let id: ElementId = widget.create(&mut create);
                let mut recorder = SceneRecorder::new(BOX);
                widget.paint(&mut recorder, id);
                recorder.finish();
                for op in recorder.ops() {
                    if let PaintOp::Path { fill, .. } = op {
                        ink = *fill;
                    }
                }
            }
        }
        out.push(ink);
    }
    out
}

fn tap(row: &Stack, index: usize) {
    let mut star = row.children().remove(index).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = star.create(&mut create);
    let at = Point::new(10.0, 10.0);
    let mut down = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    star.event(&mut down, id);
    let mut up = TestEventContext::new(id, BOX, PointerEvent::primary_up(at).into());
    star.event(&mut up, id);
}


#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentRatingStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Rating) {
        floom_core_theme::component_tokens::ComponentTokens::Rating(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.star_size, tokens.star_size, "the token's star size");
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(
        style.active_color,
        theme().colors().primary,
        "the filled star's ink"
    );
    assert_eq!(
        style.inactive_color,
        theme().colors().outline_variant,
        "the empty star's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}


#[test]
fn the_filled_stars_are_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let row = kit().value(3.0).view();
    let inks = star_inks(&row);

    assert_eq!(inks.len(), 5, "v1's default maximum is five stars");
    for (index, ink) in inks.iter().enumerate() {
        let wanted = if index < 3 {
            paint(resolved.active_color)
        } else {
            paint(resolved.inactive_color)
        };
        assert_eq!(*ink, Some(wanted), "star {index}");
    }
}

#[test]
fn the_star_is_the_size_the_token_says() {
    let resolved = kit().resolved_style();
    let row = kit().value(1.0).view();
    let star = row.children().remove(0).build();
    let slot = star.children().remove(0).build();
    let mut conditional = slot.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = conditional.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    conditional.update(&mut cx, id);
    let glyph = conditional.children().remove(0).build();
    let request = glyph
        .layout_request()
        .expect("the glyph asks for its own space");
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(resolved.star_size)),
        "the token's star size"
    );
}

#[test]
fn a_fractional_value_fills_the_stars_below_it() {
    let resolved = kit().resolved_style();
    let row = kit().value(3.5).view();
    let inks = star_inks(&row);
    assert_eq!(
        inks[3],
        Some(paint(resolved.active_color)),
        "3.5 fills the fourth star"
    );
    assert_eq!(
        inks[4],
        Some(paint(resolved.inactive_color)),
        "and not the fifth"
    );
}


#[test]
fn a_tap_fills_the_stars_below_it() {
    let resolved = kit().resolved_style();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let row = kit()
        .value(1.0)
        .on_change(move |value| recorder.lock().unwrap().push(value))
        .view();

    tap(&row, 2);

    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[3.0],
        "v1's value is one more"
    );
    let inks = star_inks(&row);
    assert_eq!(inks[0], Some(paint(resolved.active_color)), "star 0");
    assert_eq!(inks[1], Some(paint(resolved.active_color)), "star 1");
    assert_eq!(inks[2], Some(paint(resolved.active_color)), "star 2");
    assert_eq!(inks[3], Some(paint(resolved.inactive_color)), "star 3");
    assert_eq!(inks[4], Some(paint(resolved.inactive_color)), "star 4");
}

#[test]
fn a_read_only_rating_does_not_answer() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let recorder = seen.clone();
    let row = kit()
        .value(1.0)
        .readonly(true)
        .on_change(move |value| recorder.lock().unwrap().push(value))
        .view();

    tap(&row, 3);

    assert!(
        seen.lock().unwrap().is_empty(),
        "v1's read-only rating reported nothing"
    );
    let resolved = kit().resolved_style();
    let inks = star_inks(&row);
    assert_eq!(
        inks[3],
        Some(paint(resolved.inactive_color)),
        "and the star is still empty"
    );
}

#[test]
fn the_props_report_the_value() {
    let kit = kit().value(3.0).max(7);
    assert_eq!(kit.props().value, 3.0);
    assert_eq!(kit.props().max, 7);
    assert!(!kit.props().readonly);
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentRating::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent rating took a snapshot, not the app-wide signal"
    );
}
