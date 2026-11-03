
use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget};
use floom_types::{Color, PointerPhase};
use floom_ui_composites::Menu;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder, TestEventContext,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::with_alpha;

use floom_widget_md_menu::{MaterialMenu, MaterialMenuStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 300.0,
    y1: 200.0,
};

/// A point over the first row.
const OVER_FIRST: Point = Point { x: 10.0, y: 20.0 };

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialMenu {
    MaterialMenu::new().with_theme(theme())
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

/// The update pass, on a context that refuses overlays -- so the panel is in
/// the tree and a test can paint it.
fn tick(menu: &mut Menu) {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = menu.create(&mut create);
    let mut cx = RecordingUpdateContext::without_overlay_support();
    menu.update(&mut cx, id);
}

/// The menu, open, with its panel in the tree.
fn open_menu(kit: MaterialMenu) -> Menu {
    let mut menu = kit.view();
    floom_widgets::write_flag(&menu.open_flag(), true);
    tick(&mut menu);
    menu
}


/// The tokens and the colours reach the engine's style.
#[test]
fn the_tokens_and_colours_reach_the_engine_style() {
    let style: MaterialMenuStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Menu) {
        floom_core_theme::component_tokens::ComponentTokens::Menu(tokens) => tokens,
        _ => unreachable!(),
    };

    assert_eq!(
        style.min_width, tokens.min_width,
        "the token's minimum width"
    );
    assert_eq!(
        style.max_width, tokens.max_width,
        "the token's maximum width"
    );
    assert_eq!(
        style.item_height, tokens.item_height,
        "the token's row height"
    );
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.container_color,
        theme().colors().surface,
        "the panel's fill"
    );
    assert_eq!(
        style.text_color,
        theme().colors().on_surface,
        "the rows' ink"
    );
    assert_eq!(
        style.item_padding_horizontal, 12.0,
        "v1's row: `padding_horiz(12.0)`"
    );
    assert_eq!(
        style.item_padding_vertical, 8.0,
        "v1's row: `padding_vert(8.0)`"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_hovered_row_is_v1_s_fill() {
    assert_eq!(
        kit().resolved_style().hover_color,
        with_alpha(theme().colors().on_surface, 8),
        "v1's hovered row: `with_alpha(on_surface, 8)`"
    );
}

#[test]
fn the_panel_radius_is_v1_s() {
    assert_eq!(
        kit().resolved_style().radius,
        4.0,
        "v1's panel: `border_radius(4.0)`, a literal"
    );
}


#[test]
fn the_menu_paints_v1_s_appearance() {
    let resolved = kit().resolved_style();
    let menu = open_menu(kit().item("One", || {}));
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
        Some(paint(resolved.container_color)),
        "the panel's fill"
    );
    let ink = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(ink, Some(paint(resolved.text_color)), "and the rows' ink");
}

#[test]
fn the_hovered_row_is_filled_when_the_pointer_arrives() {
    let resolved = kit().resolved_style();
    let menu = open_menu(kit().item("One", || {}));
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
        Some(paint(resolved.hover_color)),
        "the hovered row's fill"
    );
}


/// Choosing a row runs its callback.
#[test]
fn choosing_a_row_runs_its_callback() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s = seen.clone();
    let kit = kit().item("One", move || s.lock().unwrap().push(0));
    kit.select(0);
    assert_eq!(seen.lock().unwrap().as_slice(), &[0]);
}

/// The flag the menu shares with its caller is the one that opens it.
#[test]
fn the_open_flag_is_shared_with_the_caller() {
    let flag = floom_widgets::shared_flag(false);
    let mut menu = kit().with_open_flag(flag.clone()).view();
    assert!(!menu.is_open(), "closed to begin with");

    floom_widgets::write_flag(&flag, true);
    tick(&mut menu);
    assert!(menu.is_open(), "the caller's flag opened it");
    assert_eq!(
        menu.children().len(),
        1,
        "and the panel is in the tree while there is no host to ask"
    );
}

/// The props report the items and the open state.
#[test]
fn the_props_report_the_items() {
    let kit = kit().item("One", || {}).item("Two", || {});
    assert_eq!(kit.props().items.len(), 2);
    assert_eq!(kit.props().items[0].label, "One");
    assert!(!kit.props().open, "and the menu starts closed");
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialMenu::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material menu read the app-wide signal, not a snapshot"
    );
}
