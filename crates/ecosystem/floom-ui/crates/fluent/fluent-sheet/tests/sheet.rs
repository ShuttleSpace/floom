//! The ported Fluent sheet -- a panel on the edge of the window, over a
//! scrim, that the caller shows and hides.
//!
//! The sheet is the engine's `Dialog` composite with the scrim and the panel
//! as its card, so what is tested here is the translation and the pixels: the
//! tokens reach the style, the scrim and the panel are painted in v1's
//! colours, the panel is sized and rounded the way each side asks, and the
//! shared flag is what opens and closes it.
//!
//! ## The tripwires
//!
//! `the_scrim_is_black_at_160` and `the_open_flag_opens_and_closes_the_sheet`
//! would fail if this kit stopped being this kit.

use floom_core::{ElementId, Length, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, OverlayCall, PaintOp, RecordingUpdateContext, SceneRecorder,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::sheet::SheetSide;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_sheet::{FluentSheet, FluentSheetStyle};

/// A viewport, because a sheet covers the window rather than its own box.
const VIEWPORT: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 800.0,
    y1: 600.0,
};

const ID: ElementId = ElementId::new(1, 1);

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentSheet {
    FluentSheet::new().with_theme(theme())
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

/// The panel inside the card the host mounts.
///
/// Opened through the engine's own door, so the test sees what the host would
/// have built: the scrim, then the aligned stack, then the panel.
fn panel_of(kit: FluentSheet) -> Box<dyn floom_core::WidgetAny> {
    // v1 hid the sheet while it was closed, so the panel and the scrim
    // only exist while it is up: the flag opens it before the host would
    // have built anything.
    let flag = kit.open_flag();
    floom_widgets::write_flag(&flag, true);
    let mut dialog = kit.view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    dialog.update(&mut cx, ID);
    let opened = cx.opened_overlays();
    assert!(!opened.is_empty(), "the sheet asked for an overlay");
    match opened[0] {
        OverlayCall::Open { content, .. } => {
            // The card is the scrim; it holds the aligned stack, which holds
            // the panel.
            let stack = content.children().remove(0).build();
            stack.children().remove(0).build()
        }
        _ => panic!("the sheet asked for an overlay"),
    }
}

/// The panel's paint ops.
fn panel_ops(kit: FluentSheet) -> Vec<PaintOp> {
    let panel = panel_of(kit);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = panel.create(&mut create);
    let mut recorder = SceneRecorder::new(VIEWPORT);
    panel.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The scrim's paint ops: the card the host mounts.
fn scrim_ops(kit: FluentSheet) -> Vec<PaintOp> {
    // v1 hid the sheet while it was closed, so the panel and the scrim
    // only exist while it is up: the flag opens it before the host would
    // have built anything.
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

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
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

/// v1 wrote `with_alpha(scrim_color, 160)`, which *replaces* the scrim token's
/// own alpha rather than multiplying it. The token's alpha never showed.
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

// --- the pixels -----------------------------------------------------------

/// The panel is painted the surface colour.
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

/// A bottom sheet spans the width and is rounded.
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

/// A side sheet is the token's width, full height, and not rounded.
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

/// The panel's padding is the token's.
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

/// An elevated sheet casts the shadow v1 asked for; a flat one does not.
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

// --- the presence ---------------------------------------------------------

/// The shared flag is what opens and closes the sheet.
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

/// `open_flag()` hands back the same allocation. `Arc::ptr_eq`, because two
/// independent flags would pass every behavioural test and then diverge.
#[test]
fn the_flag_is_one_allocation() {
    let kit = kit();
    let handed_back = kit.open_flag();
    assert!(
        std::sync::Arc::ptr_eq(&kit.open_flag(), &handed_back),
        "one flag, not two"
    );
}

/// The props report the side, the elevation and the open state.
#[test]
fn the_props_report_the_side_and_the_elevation() {
    let kit = kit().side(SheetSide::Start).elevated(false).open(true);
    assert_eq!(kit.props().side, SheetSide::Start);
    assert!(!kit.props().elevated);
    assert!(kit.props().open);
    assert!(kit.is_open(), "and the flag agrees with the prop");
}

// --- the theme ------------------------------------------------------------

/// The theme: v1's Fluent sheet took a *snapshot* of its own design system,
/// so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        FluentSheet::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent sheet took a snapshot, not the app-wide signal"
    );
}

/// v1's Fluent sheet rounded its panel to the theme's small radius while its
/// own style reported the token -- and the two were not the same number. The
/// port reports both, and paints the theme's.
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
