
use floom_core::{ElementId, Point, Rect, Widget};
use floom_types::Color;
use floom_types::PointerEvent;
use floom_ui_composites::{popup_height, Select, SelectStyle};
use floom_widgets::controls::Button;
use floom_widgets::testkit::{
    paint_snapshot_any, AllocatingCreateContext, SceneRecorder, TestEventContext,
};

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn allocate(widget: &dyn Widget) -> ElementId {
    let mut create = AllocatingCreateContext::new();
    widget.create(&mut create)
}

/// Paints `widget` and hands back the ops it produced.
fn ops_of(
    widget: &dyn Widget,
    id: ElementId,
    bounds: Rect,
) -> Vec<floom_widgets::testkit::PaintOp> {
    let mut recorder = SceneRecorder::new(bounds);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The default style with `min_width` lifted, so a width assertion measures the
/// label rather than the floor.
fn no_min_width() -> SelectStyle {
    SelectStyle {
        min_width: 0.0,
        ..SelectStyle::default()
    }
}

/// The intrinsic size a Select asks for, as `(width, height)`.
///
/// Both are `Length::Px` here — a Select's own size is never a percentage — so
/// `px()` returning `None` would be the widget asking in a unit this test
/// cannot compare.
fn request_of(select: &Select) -> (f32, f32) {
    let request = select.layout_request().expect("a select asks for a size");
    (
        request.width.expect("a width").px().expect("in px"),
        request.height.expect("a height").px().expect("in px"),
    )
}

/// Opens `select` and returns the popup it built plus the rect it asked for.
fn popup_of(select: &mut Select, id: ElementId) -> (Box<dyn floom_core::WidgetAny>, Rect) {
    let mut cx =
        TestEventContext::pointer(id, ROW, PointerEvent::primary_up(Point::new(10.0, 5.0)));
    assert!(select.event(&mut cx, id), "the header click opens the list");
    let bounds = match cx.last_open() {
        Some(floom_widgets::testkit::OverlayCall::Open { bounds, .. }) => *bounds,
        _ => panic!("the select asked for no overlay"),
    };
    let content = cx.take_content().expect("the request carried content");
    (content, bounds)
}

// --- layout ---------------------------------------------------------------

#[test]
fn the_style_height_reaches_the_layout_request() {
    let base = Select::new(["Red"]);
    let tall = Select::new(["Red"]).style(SelectStyle {
        height: 96.0,
        ..SelectStyle::default()
    });

    let (_, base_height) = request_of(&base);
    let (_, tall_height) = request_of(&tall);
    assert_eq!(base_height, SelectStyle::default().height);
    assert_eq!(tall_height, 96.0);
}

#[test]
fn a_short_option_list_still_asks_for_the_style_min_width() {
    let base = Select::new(["R"]);
    let wide = Select::new(["R"]).style(SelectStyle {
        min_width: 400.0,
        ..SelectStyle::default()
    });

    let (base_width, _) = request_of(&base);
    let (wide_width, _) = request_of(&wide);
    assert_eq!(base_width, SelectStyle::default().min_width);
    assert_eq!(wide_width, 400.0);
}

#[test]
fn the_padding_and_the_text_size_reach_the_layout_request() {
    // Both feed the same measurement, so this is the pair that proves the
    // *width* is read from the style rather than from a constant: a bigger
    // font makes a wider control, and so does a bigger padding. `min_width`
    // has to be out of the way or it is a floor the measurement never reaches
    // (which the test next door asserts, in the other direction).
    let base = Select::new(["Red", "Green"]).style(no_min_width());
    let large = Select::new(["Red", "Green"]).style(SelectStyle {
        text_size: 40.0,
        ..no_min_width()
    });

    let (base_width, _) = request_of(&base);
    let (large_width, _) = request_of(&large);
    assert!(
        large_width > base_width + 40.0,
        "a larger font must ask for more room: {base_width} -> {large_width}"
    );
}

#[test]
fn a_chevron_widens_the_trigger_by_the_gap_and_the_icon() {
    // A chevron is the kit's to choose, but the *space* it takes is the
    // style's — and a trigger that laid itself out without it would clip it.
    let icon = floom_widgets::VectorIcon::from_path_data(
        "M0 0 L10 0 L5 6 Z",
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        },
    );

    let without = Select::new(["Red"]).style(no_min_width());
    let with = Select::new(["Red"]).chevron(icon).style(no_min_width());
    let style = no_min_width();

    let (without_width, _) = request_of(&without);
    let (with_width, _) = request_of(&with);
    assert_eq!(
        with_width,
        without_width + style.trigger_gap + style.icon_size,
        "the chevron's box is reserved, not overlapped by the label"
    );
}

// --- paint: the trigger ---------------------------------------------------

#[test]
fn the_trigger_paints_the_styles_border_then_its_background() {
    let style = SelectStyle {
        outline: Color::rgb(0x11, 0x22, 0x33),
        surface: Color::rgb(0xAA, 0xBB, 0xCC),
        border_width: 3.0,
        border_radius: 9.0,
        ..SelectStyle::default()
    };
    let select = Select::new(["Red"]).selected(0).style(style);
    let id = allocate(&select);
    let ops = ops_of(&select, id, ROW);

    // Border first, at the outer box; background second, inset by the width.
    assert!(matches!(
        ops.first(),
        Some(floom_widgets::testkit::PaintOp::RoundedRect { color, radius, .. })
            if *color == style.outline && *radius == 9.0
    ));
    assert!(matches!(
        ops.get(1),
        Some(floom_widgets::testkit::PaintOp::RoundedRect { color, bounds, .. })
            if *color == style.surface && bounds.x0 == 3.0
    ));
}

#[test]
fn a_zero_border_width_paints_no_border() {
    let select = Select::new(["Red"]).selected(0).style(SelectStyle {
        border_width: 0.0,
        ..SelectStyle::default()
    });
    let id = allocate(&select);
    let ops = ops_of(&select, id, ROW);

    let rounded: Vec<_> = ops
        .iter()
        .filter(|op| matches!(op, floom_widgets::testkit::PaintOp::RoundedRect { .. }))
        .collect();
    assert_eq!(
        rounded.len(),
        1,
        "without a border there is one box, not two:\n{}",
        ops.iter()
            .map(|op| op.snapshot_line())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn the_label_takes_the_placeholder_colour_until_something_is_chosen() {
    let style = SelectStyle {
        placeholder: Color::rgb(0x01, 0x02, 0x03),
        on_surface: Color::rgb(0xF1, 0xF2, 0xF3),
        ..SelectStyle::default()
    };
    let unchosen = Select::new(["Red", "Green"])
        .placeholder("Pick one")
        .style(style);
    let chosen = Select::new(["Red", "Green"])
        .placeholder("Pick one")
        .selected(0)
        .style(style);

    let unchosen_id = allocate(&unchosen);
    let chosen_id = allocate(&chosen);

    let placeholder_op = ops_of(&unchosen, unchosen_id, ROW)
        .into_iter()
        .find_map(|op| match op {
            floom_widgets::testkit::PaintOp::Text { text, color, .. } if text == "Pick one" => {
                Some(color)
            }
            _ => None,
        })
        .expect("the placeholder is painted");
    let chosen_op = ops_of(&chosen, chosen_id, ROW)
        .into_iter()
        .find_map(|op| match op {
            floom_widgets::testkit::PaintOp::Text { text, color, .. } if text == "Red" => {
                Some(color)
            }
            _ => None,
        })
        .expect("the chosen option is painted");

    assert_eq!(placeholder_op, style.placeholder);
    assert_eq!(chosen_op, style.on_surface);
}

#[test]
fn a_disabled_trigger_paints_the_disabled_triple() {
    let style = SelectStyle {
        disabled_text: Color::rgb(0x10, 0x00, 0x00),
        disabled_border: Color::rgb(0x20, 0x00, 0x00),
        disabled_background: Color::rgb(0x30, 0x00, 0x00),
        surface: Color::rgb(0xFF, 0xFF, 0xFF),
        outline: Color::rgb(0x00, 0x00, 0xFF),
        on_surface: Color::rgb(0x00, 0xFF, 0x00),
        ..SelectStyle::default()
    };
    let select = Select::new(["Red"]).selected(0).disabled(true).style(style);
    let id = allocate(&select);
    let snapshot = ops_of(&select, id, ROW)
        .iter()
        .map(|op| op.snapshot_line())
        .collect::<Vec<_>>()
        .join("\n");

    let colors: Vec<Color> = ops_of(&select, id, ROW)
        .iter()
        .filter_map(|op| match op {
            floom_widgets::testkit::PaintOp::RoundedRect { color, .. }
            | floom_widgets::testkit::PaintOp::Text { color, .. } => Some(*color),
            _ => None,
        })
        .collect();

    assert!(
        colors.contains(&style.disabled_border),
        "the border is the disabled one:\n{snapshot}"
    );
    assert!(
        colors.contains(&style.disabled_background),
        "the background is the disabled one:\n{snapshot}"
    );
    assert!(
        colors.contains(&style.disabled_text),
        "the label is the disabled one:\n{snapshot}"
    );
    assert!(
        !colors.contains(&style.surface) && !colors.contains(&style.outline),
        "an enabled colour leaking into a disabled control:\n{snapshot}"
    );
}

#[test]
fn the_chevron_is_tinted_from_the_style_and_disabled_from_its_own() {
    let icon = floom_widgets::VectorIcon::from_path_data(
        "M0 0 L10 0 L5 6 Z",
        Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        },
    );
    let style = SelectStyle {
        chevron: Color::rgb(0x00, 0x00, 0xAA),
        disabled_chevron: Color::rgb(0xAA, 0x00, 0x00),
        icon_size: 12.0,
        ..SelectStyle::default()
    };

    let enabled = Select::new(["Red"])
        .selected(0)
        .chevron(icon.clone())
        .style(style);
    let disabled = Select::new(["Red"])
        .selected(0)
        .disabled(true)
        .chevron(icon)
        .style(style);
    let enabled_id = allocate(&enabled);
    let disabled_id = allocate(&disabled);

    let enabled_fill = path_fill(&ops_of(&enabled, enabled_id, ROW));
    let disabled_fill = path_fill(&ops_of(&disabled, disabled_id, ROW));

    assert_eq!(enabled_fill, Some(style.chevron));
    assert_eq!(disabled_fill, Some(style.disabled_chevron));
}

fn path_fill(ops: &[floom_widgets::testkit::PaintOp]) -> Option<Color> {
    ops.iter().find_map(|op| match op {
        floom_widgets::testkit::PaintOp::Path { fill, .. } => *fill,
        _ => None,
    })
}

// --- the popup ------------------------------------------------------------

#[test]
fn the_popup_rect_follows_the_item_height_the_gap_and_the_max_height() {
    let style = SelectStyle {
        item_height: 24.0,
        panel_gap: 6.0,
        panel_padding: 5.0,
        panel_max_height: 60.0,
        ..SelectStyle::default()
    };
    let mut select = Select::new(["a", "b", "c", "d", "e"]).style(style);
    let id = allocate(&select);
    let (_, bounds) = popup_of(&mut select, id);

    assert_eq!(bounds.y0, ROW.y1 + 6.0, "the gap is read");
    assert_eq!(
        bounds.height(),
        popup_height(&style, 5),
        "five rows would be 130px; the cap is 60"
    );
    assert_eq!(bounds.height(), 60.0);
}

#[test]
fn the_popup_paints_the_styles_panel_shadow_and_rows() {
    let style = SelectStyle {
        panel_shadow: Color::rgba(0x00, 0x00, 0x00, 0x99),
        panel_shadow_blur: 21.0,
        panel_shadow_spread: 3.0,
        selected_row: Color::rgba(0x00, 0xFF, 0x00, 0x40),
        hover_row: Color::rgba(0xFF, 0x00, 0x00, 0x40),
        row_padding_horizontal: 30.0,
        text_size: 11.0,
        ..SelectStyle::default()
    };
    let mut select = Select::new(["Red", "Green"]).selected(1).style(style);
    let id = allocate(&select);
    let (content, bounds) = popup_of(&mut select, id);

    let snapshot = paint_snapshot_any(content.as_ref(), ElementId::new(99, 1), bounds);

    assert!(
        snapshot.contains("blur=21.0"),
        "the shadow blur is the style's:\n{snapshot}"
    );
    assert!(
        snapshot.contains("spread=3.0"),
        "the shadow spread is the style's:\n{snapshot}"
    );
    // Row 1 is both chosen and the initial highlight; a chosen row keeps the
    // chosen tint rather than the hover one, which is v1's precedence.
    assert!(
        snapshot.contains("#00FF0040"),
        "the chosen row is tinted `selected_row`:\n{snapshot}"
    );
    assert!(
        snapshot.contains("(30.0,"),
        "the row padding is the style's:\n{snapshot}"
    );
    assert!(
        snapshot.contains("11.0"),
        "the row text size is the style's:\n{snapshot}"
    );
}

#[test]
fn a_row_is_found_below_the_panel_padding_not_at_the_popup_edge() {
    // The panel's padding is inside the popup's rect, so the first row starts
    // below it. A click in the padding is not a click on a row.
    let style = SelectStyle {
        panel_padding: 10.0,
        item_height: 20.0,
        ..SelectStyle::default()
    };
    let mut select = Select::new(["Red", "Green"]).style(style);
    let id = allocate(&select);
    let (mut content, bounds) = popup_of(&mut select, id);
    let popup = content
        .as_any_mut()
        .downcast_mut::<floom_ui_composites::SelectPopup>()
        .expect("the content is a popup list");

    assert_eq!(popup.row_at(bounds, bounds.y0 + 2.0), None);
    assert_eq!(popup.row_at(bounds, bounds.y0 + 10.0), Some(0));
    assert_eq!(popup.row_at(bounds, bounds.y0 + 10.0 + 20.0), Some(1));
}

// --- a control that cannot be fooled ---------------------------------------

/// The failure mode this file exists for: a value the widget stores and never
/// reads. `Button` is the control the port has had longest, and it takes its
/// colours from the same `Color` type, so it is the cheapest witness that a
/// `SelectStyle` field reaching a paint op is not something the paint op does
/// by itself.
#[test]
fn another_control_painting_the_same_colour_is_not_the_style_being_read() {
    let color = Color::rgb(0x12, 0x34, 0x56);
    let button = Button::new("x");
    let id = allocate(&button);
    let snapshot = ops_of(&button, id, ROW)
        .iter()
        .map(|op| op.snapshot_line())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !snapshot.contains("#123456"),
        "the button never saw this colour, so finding it would mean the \
         assertion above is not testing what it claims:\n{snapshot}"
    );
    let _ = color;
}
