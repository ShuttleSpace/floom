//! The engine's `Menu`: a panel of rows that opens when a flag says so.
//!
//! Three things make it a menu rather than a list: the panel is up only while
//! the flag says so, the row the pointer is over is filled, and choosing a row
//! runs its callback and closes the menu.

use std::sync::Arc;
use std::sync::Mutex;

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget};
use floom_types::PointerPhase;
use floom_ui_composites::{Menu, MenuItem, MenuStyle};
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 300.0,
    y1: 200.0,
};

/// A point over the first row.
const OVER_FIRST: Point = Point { x: 10.0, y: 20.0 };

fn style() -> MenuStyle {
    MenuStyle {
        background: floom_core::Color::rgb(0xFF, 0xFB, 0xFE),
        text_color: floom_core::Color::rgb(0x1C, 0x1B, 0x1F),
        hover_color: floom_core::Color::rgba(0x1C, 0x1B, 0x1F, 8),
        min_width: 112.0,
        max_width: 280.0,
        item_height: 48.0,
        item_padding_horizontal: 12.0,
        item_padding_vertical: 8.0,
        item_font_size: 14.0,
        padding: 8.0,
        radius: 4.0,
        shadow: floom_types::Shadow {
            color: floom_core::Color::rgba(0, 0, 0, 24),
            blur: 12.0,
            offset_x: 0.0,
            offset_y: 0.0,
            spread: 1.0,
        },
    }
}

fn menu(chosen: Arc<Mutex<Vec<usize>>>) -> Menu {
    let first = chosen.clone();
    let second = chosen.clone();
    Menu::new()
        .style(style())
        .item(MenuItem::new("One").on_select(move || first.lock().unwrap().push(0)))
        .item(MenuItem::new("Two").on_select(move || second.lock().unwrap().push(1)))
}

/// The update pass, which is where the flag reaches the menu.
///
/// The context refuses every overlay, the way a headless embedder does, so the
/// menu takes its in-tree fallback and the panel is a child of it. The
/// host's own path is a separate test, below.
fn tick(menu: &mut Menu) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = menu.create(&mut create);
    let mut cx = RecordingUpdateContext::without_overlay_support();
    menu.update(&mut cx, id);
}

/// The menu is up only while the flag says so.
#[test]
fn the_menu_is_up_only_while_the_flag_says_so() {
    let chosen = Arc::new(Mutex::new(Vec::new()));
    let mut menu = menu(chosen.clone());
    assert!(!menu.is_open(), "closed to begin with");

    let flag = menu.open_flag();
    floom_widgets::write_flag(&flag, true);
    tick(&mut menu);
    assert!(menu.is_open(), "the flag opened it");
    assert_eq!(
        menu.children().len(),
        1,
        "and the panel is in the tree while there is no host to ask"
    );

    floom_widgets::write_flag(&flag, false);
    tick(&mut menu);
    assert!(!menu.is_open(), "the flag closed it");
    assert!(menu.children().is_empty(), "and the panel is gone");
}

/// With a host that can do overlays, the panel is asked of the host, with the
/// menu's own size.
#[test]
fn the_menu_asks_the_host_for_a_panel_of_its_own_size() {
    let chosen = Arc::new(Mutex::new(Vec::new()));
    let mut menu = menu(chosen);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = menu.create(&mut create);
    let mut cx = RecordingUpdateContext::with_bounds(BOX);
    floom_widgets::write_flag(&menu.open_flag(), true);
    menu.update(&mut cx, id);

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one panel was asked for");
    let bounds = match opened[0] {
        floom_widgets::testkit::OverlayCall::Open { bounds, .. } => *bounds,
        _ => unreachable!(),
    };
    assert_eq!(bounds.x0, BOX.x0, "at the box it was mounted in");
    assert_eq!(bounds.y0, BOX.y0);
    assert_eq!(
        bounds.width(),
        style().min_width,
        "and at least the minimum width"
    );
    assert_eq!(
        bounds.height(),
        2.0 * 48.0 + 8.0 * 2.0,
        "and as tall as the rows"
    );
}

/// The panel is as wide as the widest row, and as tall as the rows stacked.
#[test]
fn the_panel_is_the_widest_row_and_the_rows_stacked() {
    let chosen = Arc::new(Mutex::new(Vec::new()));
    let mut menu = menu(chosen);
    floom_widgets::write_flag(&menu.open_flag(), true);
    tick(&mut menu);

    let panel = menu.children().remove(0);
    let request = panel.layout;
    let text = floom_text::measure_text("One", 14.0, f64::INFINITY);
    let widest = text.width as f64 + 12.0 * 2.0;
    assert!(
        widest < style().min_width,
        "these labels are narrower than the minimum, so the clamp decides"
    );
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(style().min_width as f32)),
        "the panel is at least the minimum width"
    );
    assert_eq!(
        request.height,
        Some(floom_core::Length::Px((2.0 * 48.0 + 8.0 * 2.0) as f32)),
        "the rows stacked, plus the panel's padding"
    );
}

/// A row wider than the minimum widens the panel, up to the maximum.
#[test]
fn a_wide_row_widens_the_panel() {
    let mut menu = Menu::new()
        .style(style())
        .item(MenuItem::new("A very long menu item indeed"));
    let flag = menu.open_flag();
    floom_widgets::write_flag(&flag, true);
    tick(&mut menu);

    let panel = menu.children().remove(0);
    let text = floom_text::measure_text("A very long menu item indeed", 14.0, f64::INFINITY);
    let widest = text.width as f64 + 12.0 * 2.0;
    assert!(
        widest > style().min_width,
        "the label is wider than the minimum"
    );
    assert!(
        widest < style().max_width,
        "and narrower than the maximum, so the clamp leaves it alone"
    );
    assert_eq!(
        panel.layout.width,
        Some(floom_core::Length::Px(widest as f32)),
        "the panel is as wide as its widest row"
    );
}

/// The panel paints its fill, its rows, and the row the pointer is over.
#[test]
fn the_panel_paints_its_fill_and_its_rows() {
    let chosen = Arc::new(Mutex::new(Vec::new()));
    let mut menu = menu(chosen);
    floom_widgets::write_flag(&menu.open_flag(), true);
    tick(&mut menu);

    let panel = menu.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = panel.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    panel.paint(&mut recorder, id);
    recorder.finish();

    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(floom_core::Color::rgb(0xFF, 0xFB, 0xFE)),
        "the panel's fill"
    );
    let labels: Vec<String> = recorder
        .ops()
        .iter()
        .filter_map(|op| match op {
            PaintOp::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        labels,
        vec!["One".to_string(), "Two".to_string()],
        "the rows"
    );
}

/// The row the pointer is over is filled, and the others are not.
#[test]
fn the_hovered_row_is_filled() {
    let chosen = Arc::new(Mutex::new(Vec::new()));
    let mut menu = menu(chosen);
    floom_widgets::write_flag(&menu.open_flag(), true);
    tick(&mut menu);

    let mut panel = menu.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = panel.create(&mut create);
    let mut enter = PointerEvent::primary_down(OVER_FIRST);
    enter.phase = PointerPhase::Enter;
    let mut cx = TestEventContext::new(id, BOX, enter.into());
    panel.event(&mut cx, id);

    let mut recorder = SceneRecorder::new(BOX);
    panel.paint(&mut recorder, id);
    recorder.finish();
    let hovered = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        hovered,
        Some(floom_core::Color::rgba(0x1C, 0x1B, 0x1F, 8)),
        "the hovered row's fill"
    );
}

/// Choosing a row runs its callback and closes the menu.
#[test]
fn choosing_a_row_runs_its_callback_and_closes_the_menu() {
    let chosen = Arc::new(Mutex::new(Vec::new()));
    let mut menu = menu(chosen.clone());
    let flag = menu.open_flag();
    floom_widgets::write_flag(&flag, true);
    tick(&mut menu);

    let mut panel = menu.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = panel.create(&mut create);
    let mut up = PointerEvent::primary_down(OVER_FIRST);
    up.phase = PointerPhase::Up;
    let mut cx = TestEventContext::new(id, BOX, up.into());
    panel.event(&mut cx, id);

    assert_eq!(
        chosen.lock().unwrap().as_slice(),
        &[0],
        "the row's callback ran"
    );
    assert!(
        !floom_widgets::read_flag(&flag),
        "and the menu closed itself"
    );
}
