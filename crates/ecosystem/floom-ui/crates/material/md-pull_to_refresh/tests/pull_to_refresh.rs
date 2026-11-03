
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, Length, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{
    AllocatingCreateContext, PaintOp, RecordingUpdateContext, SceneRecorder,
};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_md_pull_to_refresh::{MaterialPullToRefresh, MaterialPullToRefreshStyle};

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

fn kit() -> MaterialPullToRefresh {
    MaterialPullToRefresh::new().with_theme(theme())
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

/// The column the kit's box holds.
fn column(kit: MaterialPullToRefresh) -> Box<dyn floom_core::WidgetAny> {
    let view = kit.view();
    view.children().remove(0).build()
}

/// The indicator the column holds.
fn indicator(kit: MaterialPullToRefresh) -> Box<dyn floom_core::WidgetAny> {
    let column = column(kit);
    column.children().remove(0).build()
}


/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialPullToRefreshStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::PullToRefresh) {
            floom_core_theme::component_tokens::ComponentTokens::PullToRefresh(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.max_pull, tokens.max_pull, "the token's max pull");
    assert_eq!(
        style.indicator_size, tokens.indicator_size,
        "the token's indicator size"
    );
    assert_eq!(
        style.background,
        theme().colors().background,
        "the box's fill"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the label's ink"
    );
    assert_eq!(
        style.indicator_font_size, 13.0,
        "v1's label size, a literal"
    );
    assert_eq!(
        style.indicator_max_height, 28.0,
        "v1's tallest indicator, a literal"
    );
}

#[test]
fn the_threshold_is_the_props_not_the_token() {
    let style = kit().resolved_style();
    assert_eq!(
        style.threshold,
        kit().props().threshold,
        "the prop's threshold"
    );
    let kit = kit().threshold(120.0);
    assert_eq!(
        kit.resolved_style().threshold,
        120.0,
        "and it follows the prop"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}


#[test]
fn the_box_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    let view = kit().view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = view.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    view.paint(&mut recorder, id);
    recorder.finish();
    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(fill, Some(paint(resolved.background)), "the box's fill");
}

#[test]
fn the_indicator_is_painted_in_v1_s_ink_at_v1_s_size() {
    let resolved = kit().resolved_style();
    let kit = kit();
    *kit.pull_distance().lock().unwrap() = 20.0;
    let indicator = indicator(kit);
    let label = indicator.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    let (text, size, ink) = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the label is painted");
    assert_eq!(text, "↓ Pull to refresh", "v1's label while pulling");
    assert_eq!(size, resolved.indicator_font_size, "v1's label size");
    assert_eq!(ink, paint(resolved.on_surface_variant), "v1's label ink");
}

#[test]
fn the_refreshing_state_shows_v1_s_other_label() {
    let kit = kit().loading(true);
    let indicator = indicator(kit);
    let label = indicator.children().remove(0).build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = label.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    label.paint(&mut recorder, id);
    recorder.finish();
    let text = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .expect("the label is painted");
    assert_eq!(text, "↻ Refreshing…", "v1's label while refreshing");
}


/// The indicator is absent until there is something to say.
#[test]
fn the_indicator_is_absent_until_there_is_something_to_say() {
    let indicator = indicator(kit());
    assert!(
        indicator.children().is_empty(),
        "v1 drew nothing until the pull passed 0.5"
    );
}

/// The pull is the indicator's height, and the content is pushed down under it.
#[test]
fn the_pull_is_the_indicator_s_height() {
    let kit = kit();
    *kit.pull_distance().lock().unwrap() = 12.0;
    let indicator = indicator(kit);
    let request = indicator
        .layout_request()
        .expect("the indicator asks for its own height");
    assert_eq!(
        request.height,
        Some(Length::Px(12.0)),
        "v1 grew the indicator with the pull"
    );
}

/// The content is the column's other child.
#[test]
fn the_content_is_the_column_s_other_child() {
    let child = floom_core::Child::new(
        floom_core::ChildKey::index(0),
        floom_core::LayoutRequest::new(),
        || Box::new(floom_widgets::Text::new("Content")),
    );
    let column = column(kit().child(child));
    assert_eq!(column.children().len(), 2, "the indicator and the content");
}

/// The refresh fires when the pull passes the threshold.
#[test]
fn the_refresh_fires_when_the_pull_passes_the_threshold() {
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = fired.clone();
    let kit = kit().on_refresh(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    let refreshing = kit.refreshing_flag();
    *kit.pull_distance().lock().unwrap() = 100.0;
    let mut column = column(kit);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = column.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    column.update(&mut cx, id);
    assert_eq!(
        fired.load(Ordering::SeqCst),
        1,
        "v1 fired the refresh past the threshold"
    );
    assert!(
        floom_widgets::read_flag(&refreshing),
        "and showed the refreshing state"
    );
}

/// A pull that has not reached the threshold fires nothing.
#[test]
fn a_pull_short_of_the_threshold_fires_nothing() {
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = fired.clone();
    let kit = kit().on_refresh(move || {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    *kit.pull_distance().lock().unwrap() = 10.0;
    let mut column = column(kit);
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = column.create(&mut create);
    let mut cx = RecordingUpdateContext::new();
    column.update(&mut cx, id);
    assert_eq!(
        fired.load(Ordering::SeqCst),
        0,
        "the threshold was not passed"
    );
}

/// The props report the threshold.
#[test]
fn the_props_report_the_threshold() {
    let kit = kit().threshold(64.0);
    assert_eq!(kit.props().threshold, 64.0);
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialPullToRefresh::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material pull-to-refresh read the app-wide signal, not a snapshot"
    );
}
