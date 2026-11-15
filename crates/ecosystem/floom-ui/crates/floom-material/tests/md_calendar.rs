#![cfg(feature = "calendar")]
use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, Point, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::calendar::CalendarDate as ThemeDate;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::with_alpha;

use floom_material::widgets::calendar::{CalendarDate, MaterialCalendar, MaterialCalendarStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialCalendar {
    MaterialCalendar::new().with_theme(theme())
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

/// One widget's paint ops.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The text a widget paints, if it paints one.
fn text_of(widget: &dyn Widget) -> Option<(String, f32, Color)> {
    ops_of(widget).into_iter().find_map(|op| match op {
        PaintOp::Text {
            text, size, color, ..
        } => Some((text, size, color)),
        _ => None,
    })
}

/// The calendar's body: the header, the weekday row and the grid.
fn body_of(kit: MaterialCalendar) -> Box<dyn WidgetAny> {
    kit.view().children().remove(0).build()
}

/// The header's title: the month and the year.
fn title_of(kit: MaterialCalendar) -> Option<(String, f32, Color)> {
    let body = body_of(kit);
    let header = body.children().remove(0).build();
    let title = header.children().remove(1).build();
    text_of(&*title)
}

/// The cell whose number is `label`.
fn cell_widget(kit: MaterialCalendar, label: &str) -> Box<dyn WidgetAny> {
    let body = body_of(kit);
    let grid = body.children().remove(2).build();
    for week in 0..6 {
        let row = grid.children().remove(week).build();
        for column in 0..row.children().len() {
            let cell = row.children().remove(column).build();
            let surface = cell.children().remove(0).build();
            let text = surface.children().remove(0).build();
            let painted = text_of(&*text);
            if painted.as_ref().map(|(text, _, _)| text.as_str()) == Some(label) {
                return cell;
            }
        }
    }
    panic!("the month has no cell labelled {label}");
}

/// The fill and the number a cell paints.
fn cell_paint(cell: &dyn WidgetAny) -> (Option<Color>, Option<(String, f32, Color)>) {
    let surface = cell.children().remove(0).build();
    let text = surface.children().remove(0).build();
    let fill = ops_of(&*surface).into_iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(color),
        PaintOp::RoundedRect { color, .. } => Some(color),
        _ => None,
    });
    (fill, text_of(&*text))
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialCalendarStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Calendar)
    {
        floom_core_theme::component_tokens::ComponentTokens::Calendar(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.cell_size, tokens.cell_size, "the token's cell size");
    assert_eq!(
        style.panel_width, tokens.panel_width,
        "the token's panel width"
    );
    assert_eq!(
        style.header_font_size, tokens.header_font_size,
        "the token's header size"
    );
    assert_eq!(
        style.day_font_size, tokens.day_font_size,
        "the token's day size"
    );
    assert_eq!(
        style.weekday_font_size, tokens.weekday_font_size,
        "the token's weekday size"
    );
    assert_eq!(
        style.weeknumber_width, tokens.weeknumber_width,
        "the token's week-number width"
    );
    assert_eq!(
        style.border_radius, tokens.border_radius,
        "the token's radius"
    );
    assert_eq!(style.surface, theme().colors().surface, "the panel's fill");
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the day numbers' ink"
    );
    assert_eq!(
        style.primary,
        theme().colors().primary,
        "the selected day's fill"
    );
    assert_eq!(
        style.on_primary,
        theme().colors().on_primary,
        "the selected day's number"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_panel_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let panel = kit().view();
    let ops = ops_of(&panel);

    let fill = ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.surface)), "the panel's fill");

    let border = ops.iter().find_map(|op| match op {
        PaintOp::Path { stroke, .. } => stroke.map(|(width, color)| (width, color)),
        _ => None,
    });
    let (width, color) = border.expect("the panel has an outline");
    assert_eq!(width, 1.0, "v1's border was one pixel wide");
    assert_eq!(
        color,
        paint(with_alpha(resolved.outline, 120)),
        "v1's outline was the outline colour at 120"
    );
}

/// The header names the month and the year.
#[test]
fn the_header_names_the_month() {
    let resolved = kit().resolved_style();
    let (text, size, ink) = title_of(kit()).expect("the title is painted");
    assert_eq!(text, "August 2026", "v1's default date is August 2026");
    assert_eq!(size, resolved.header_font_size, "the token's header size");
    assert_eq!(ink, paint(resolved.on_surface), "v1's title ink");
}

/// The selected day is filled with `primary` and its number is `on_primary`;
/// every other day is not filled, and its number is `on_surface`.
#[test]
fn the_selected_day_is_filled_and_the_others_are_not() {
    let resolved = kit().resolved_style();
    let (fill, painted) = cell_paint(&*cell_widget(kit(), "1"));
    assert_eq!(
        fill,
        Some(paint(resolved.primary)),
        "the selected day is filled with the primary colour"
    );
    let (_, size, ink) = painted.expect("the selected day is painted");
    assert_eq!(size, resolved.day_font_size, "the token's day size");
    assert_eq!(
        ink,
        paint(resolved.on_primary),
        "the selected day's number is the on-primary colour"
    );

    let (fill, painted) = cell_paint(&*cell_widget(kit(), "2"));
    assert_eq!(
        fill,
        Some(paint(floom_core_theme::tokens::color::Color::rgba(
            0, 0, 0, 0
        ))),
        "an unselected day is not filled"
    );
    let (_, _, ink) = painted.expect("the day is painted");
    assert_eq!(ink, paint(resolved.on_surface), "v1's day ink");
}

/// Clicking a day picks it: the callback is called with the date, and the
/// date is the one the cell shows.
#[test]
fn clicking_a_day_picks_it() {
    let picked: Arc<Mutex<Option<CalendarDate>>> = Arc::new(Mutex::new(None));
    let recorder = picked.clone();
    let kit = kit().on_change(move |date| {
        if let Ok(mut guard) = recorder.lock() {
            *guard = Some(date);
        }
    });

    let mut cell = cell_widget(kit, "15");

    let id = ElementId::new(0, 1);
    let mut down = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(floom_types::PointerEvent::primary_down(Point::new(
            5.0, 5.0,
        ))),
    );
    cell.event(&mut down, id);
    let mut up = TestEventContext::new(
        id,
        BOX,
        Event::Pointer(floom_types::PointerEvent::primary_up(Point::new(5.0, 5.0))),
    );
    cell.event(&mut up, id);

    let picked = picked.lock().map(|guard| *guard).unwrap_or(None);
    assert_eq!(
        picked,
        Some(CalendarDate {
            year: 2026,
            month: 8,
            day: 15
        }),
        "the day that was clicked is the one that was picked"
    );
}

/// The week numbers are drawn only when the caller asks for them, and they
/// take a column of their own.
#[test]
fn the_week_numbers_are_drawn_only_when_asked_for() {
    let body = body_of(kit());
    let grid = body.children().remove(2).build();
    let row = grid.children().remove(1).build();
    assert_eq!(
        row.children().len(),
        7,
        "seven days, and no week-number column"
    );

    let body = body_of(kit().show_week_numbers(true));
    let grid = body.children().remove(2).build();
    let row = grid.children().remove(1).build();
    assert_eq!(
        row.children().len(),
        8,
        "the week-number column, and the seven days"
    );
}

#[test]
fn the_props_report_the_date() {
    let kit = kit()
        .value(CalendarDate {
            year: 2027,
            month: 3,
            day: 9,
        })
        .show_week_numbers(true);
    assert_eq!(kit.props().value.year, 2027);
    assert_eq!(kit.props().value.month, 3);
    assert_eq!(kit.props().value.day, 9);
    assert!(kit.props().show_week_numbers);
    assert!(
        kit.props().on_change.is_none(),
        "the props' callback slot stays empty; the kit holds its own handle"
    );
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialCalendar::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material calendar read the app-wide signal, not a snapshot"
    );
}
