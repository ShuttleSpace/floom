#![cfg(feature = "sheet")]
//! The ported Fluent sheet -- a panel on the edge of the window, over a

use floom_core::{ElementId, Length, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, OverlayCall, PaintOp, RecordingUpdateContext, SceneRecorder,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::sheet::SheetSide;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::sheet::{FluentSheet, FluentSheetStyle};

const VIEWPORT: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 800.0,
    y1: 600.0,
};

const ID: ElementId = ElementId::new(1, 1);

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentSheet {
    FluentSheet::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn panel_of(kit: FluentSheet) -> Box<dyn floom_core::WidgetAny> {
    let flag = kit.open_flag();
    floom_widgets::write_flag(&flag, true);
    let mut dialog = kit.view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    dialog.update(&mut cx, ID);
    let opened = cx.opened_overlays();
    assert!(!opened.is_empty(), "the sheet asked for an overlay");
    match opened[0] {
        OverlayCall::Open { content, .. } => {
            let stack = content.children().remove(0).build();
            stack.children().remove(0).build()
        }
        _ => panic!("the sheet asked for an overlay"),
    }
}

fn panel_ops(kit: FluentSheet) -> Vec<PaintOp> {
    let panel = panel_of(kit);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = panel.create(&mut create);
    let mut recorder = SceneRecorder::new(VIEWPORT);
    panel.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn scrim_ops(kit: FluentSheet) -> Vec<PaintOp> {
    let flag = kit.open_flag();
    floom_widgets::write_flag(&flag, true);
    let mut dialog = kit.view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    dialog.update(&mut cx, ID);
    let opened = cx.opened_overlays();
    assert!(!opened.is_empty(), "the sheet asked for an overlay");
    let content = match opened[0] {
        OverlayCall::Open { content, .. } => content,
        _ => panic!("the sheet asked for an overlay"),
    };
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = content.create(&mut create);
    let mut recorder = SceneRecorder::new(VIEWPORT);
    content.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentSheetStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Sheet) {
        floom_core_theme::component_tokens::ComponentTokens::Sheet(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.side_width, tokens.side_width,
        "the token's side width"
    );
    assert_eq!(style.shadow_blur, tokens.shadow_blur, "the shadow's blur");
    assert_eq!(
        style.shadow_spread, tokens.shadow_spread,
        "the shadow's spread"
    );
    assert_eq!(style.scrim_color, tokens.scrim_color, "the scrim's colour");
    assert_eq!(
        style.corner_radius,
        theme().radius().sm,
        "the panel's corner radius -- v1's paint used the theme's radius"
    );
    assert_eq!(
        style.token_corner_radius, tokens.corner_radius,
        "and the token v1's style reported"
    );
    assert_ne!(
        style.corner_radius, style.token_corner_radius,
        "which for Fluent DISAGREE: v1's paint rounded the panel to 2.0 while \
         v1's style said 8.0. The port reports both, and paints the theme's."
    );
    assert_eq!(
        style.panel_color,
        theme().colors().surface,
        "the panel's fill"
    );
    assert_eq!(
        style.content_color,
        theme().colors().on_surface,
        "the content's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_scrim_is_black_at_160() {
    let resolved = kit().resolved_style();
    let fill = scrim_ops(kit()).iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 160
        }),
        "the scrim is black at 160, whatever the scrim token's own alpha was"
    );
    assert_ne!(
        resolved.scrim_color.a, 160,
        "CONTROL: the token's alpha is not 160, so the assertion above is about \
         the literal and not about the token"
    );
}

#[test]
fn the_panel_is_painted_the_surface_colour() {
    let resolved = kit().resolved_style();
    let fill = panel_ops(kit()).iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.panel_color)), "the panel's fill");
}

#[test]
fn a_bottom_sheet_spans_the_width_and_is_rounded() {
    let resolved = kit().resolved_style();
    let panel = panel_of(kit().side(SheetSide::Bottom));
    let request = panel
        .layout_request()
        .expect("the panel asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Pct(100.0)),
        "v1's `.width_pct(100.0)` for the top and bottom sheets"
    );
    let surface = panel
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .expect("the panel is a surface");
    assert_eq!(
        surface.decoration().corner_radius(),
        resolved.corner_radius,
        "v1 rounded the top and bottom sheets"
    );
}

#[test]
fn a_side_sheet_is_the_tokens_width_and_full_height() {
    let resolved = kit().resolved_style();
    let panel = panel_of(kit().side(SheetSide::End));
    let request = panel
        .layout_request()
        .expect("the panel asks for its own space");
    assert_eq!(
        request.width,
        Some(Length::Px(resolved.side_width)),
        "v1's `.width(side_w)` for the side sheets"
    );
    assert_eq!(
        request.height,
        Some(Length::Pct(100.0)),
        "v1's `.height_pct(100.0)` for the side sheets"
    );
    let surface = panel
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .expect("the panel is a surface");
    assert_eq!(
        surface.decoration().corner_radius(),
        0.0,
        "v1 rounded the top and bottom sheets only"
    );
}

#[test]
fn the_panel_is_padded_with_the_token() {
    let resolved = kit().resolved_style();
    let panel = panel_of(kit());
    let request = panel
        .layout_request()
        .expect("the panel asks for its own space");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets::uniform(f64::from(resolved.padding))),
        "v1's `.padding(pad)`"
    );
}

#[test]
fn an_elevated_sheet_casts_the_shadow() {
    let resolved = kit().resolved_style();
    let shadow = panel_ops(kit().elevated(true))
        .iter()
        .find_map(|op| match op {
            PaintOp::Shadow { blur, spread, .. } => Some((*blur, *spread)),
            _ => None,
        })
        .expect("an elevated sheet casts a shadow");
    assert_eq!(
        shadow.0,
        f64::from(resolved.shadow_blur),
        "the shadow's blur"
    );
    assert_eq!(
        shadow.1,
        f64::from(resolved.shadow_spread),
        "the shadow's spread"
    );

    assert!(
        !panel_ops(kit().elevated(false))
            .iter()
            .any(|op| matches!(op, PaintOp::Shadow { .. })),
        "and a flat sheet casts none"
    );
}

#[test]
fn the_open_flag_opens_and_closes_the_sheet() {
    let kit = kit();
    let flag = kit.open_flag();
    assert!(!kit.is_open(), "closed to begin with");

    let mut dialog = kit.view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    dialog.update(&mut cx, ID);
    assert!(!cx.opened_any_overlay(), "a closed sheet asks for nothing");

    floom_widgets::write_flag(&flag, true);
    cx.clear();
    dialog.update(&mut cx, ID);
    assert_eq!(
        cx.opened_overlays().len(),
        1,
        "and the flag opens it, on a later frame -- which is the whole reason \
         the flag exists"
    );
}

#[test]
fn the_flag_is_one_allocation() {
    let kit = kit();
    let handed_back = kit.open_flag();
    assert!(
        std::sync::Arc::ptr_eq(&kit.open_flag(), &handed_back),
        "one flag, not two"
    );
}

#[test]
fn the_props_report_the_side_and_the_elevation() {
    let kit = kit().side(SheetSide::Start).elevated(false).open(true);
    assert_eq!(kit.props().side, SheetSide::Start);
    assert!(!kit.props().elevated);
    assert!(kit.props().open);
    assert!(kit.is_open(), "and the flag agrees with the prop");
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentSheet::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent sheet took a snapshot, not the app-wide signal"
    );
}

#[test]
fn the_painted_radius_is_the_themes_and_the_token_is_reported_too() {
    let resolved = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Sheet) {
        floom_core_theme::component_tokens::ComponentTokens::Sheet(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(resolved.corner_radius, theme().radius().sm, "as painted");
    assert_eq!(
        resolved.token_corner_radius, tokens.corner_radius,
        "and the token v1's style reported"
    );

    let panel = panel_of(kit().side(SheetSide::Bottom));
    let surface = panel
        .as_any()
        .downcast_ref::<floom_widgets::Surface>()
        .expect("the panel is a surface");
    assert_eq!(
        surface.decoration().corner_radius(),
        theme().radius().sm,
        "so the panel really is rounded to the theme's radius"
    );
}
