#![cfg(feature = "sparkline")]
use floom_core::{ElementId, LayoutRequest, Rect, Widget, WidgetAny};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::sparkline::{MaterialSparkline, MaterialSparklineStyle};

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

fn kit() -> MaterialSparkline {
    MaterialSparkline::new().with_theme(theme())
}

fn tokens() -> floom_core_theme::component_tokens::SparklineTokens {
    match theme().component(floom_core_theme::component_tokens::ComponentId::Sparkline) {
        floom_core_theme::component_tokens::ComponentTokens::Sparkline(t) => t,
        _ => unreachable!("the sparkline component must resolve to SparklineTokens"),
    }
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

/// The bars of a sparkline, as the widgets they are.
fn bars(sparkline: &impl Widget) -> Vec<Box<dyn WidgetAny>> {
    sparkline
        .children()
        .into_iter()
        .map(|child| child.build())
        .collect()
}

/// The size a bar asked for.
fn bar_size(bar: &dyn WidgetAny) -> (Option<f32>, Option<f32>) {
    bar.layout_request()
        .map(|r: LayoutRequest| (r.width.and_then(|w| w.px()), r.height.and_then(|h| h.px())))
        .unwrap_or((None, None))
}

/// The fill a bar painted.
fn bar_fill(bar: &dyn WidgetAny) -> Option<Color> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = bar.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    bar.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    })
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialSparklineStyle = kit().resolved_style();
    assert_eq!(style.width, tokens().width, "the token's width");
    assert_eq!(style.height, tokens().height, "the token's height");
    assert_eq!(
        style.fill_opacity,
        tokens().fill_opacity,
        "the token's alpha"
    );
    assert_eq!(
        style.bar_color,
        floom_core_theme::tokens::color_ext::with_alpha(
            theme().colors().primary,
            tokens().fill_opacity
        ),
        "v1's default is a translucent primary"
    );
    assert!(
        style.fill,
        "v1's `SparklineProps::default()` has fill: true"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

/// The props win over the tokens.
#[test]
fn the_props_win_over_the_tokens() {
    assert_eq!(
        kit().resolved_style().width,
        tokens().width,
        "the props defer"
    );
    assert_eq!(kit().width(120.0).resolved_style().width, 120.0);
    assert_eq!(kit().height(30.0).resolved_style().height, 30.0);
}

#[test]
fn the_gap_is_the_props_line_width_and_not_the_tokens() {
    let style = kit().line_width(6.0).resolved_style();
    assert_eq!(style.gap, 6.0, "v1's gap is the props' `line_width`");
    assert_eq!(
        style.token_line_width,
        tokens().line_width,
        "and the token's own bar width is reported but never used"
    );
    assert_eq!(
        kit().resolved_style().gap,
        kit().props().line_width,
        "the props' default is the gap"
    );
}

/// The bar's width is computed from the rendered width and the gap.
#[test]
fn the_bar_width_is_computed_from_the_width_and_the_gap() {
    let style = kit()
        .values(vec![1.0, 2.0, 3.0])
        .width(100.0)
        .height(50.0)
        .resolved_style();
    assert!(
        (style.bar_width - 92.0 / 3.0).abs() < 1e-4,
        "the bar's width: {}",
        style.bar_width
    );
}

/// A series whose values are all equal is drawn against a range of one.
#[test]
fn a_flat_series_is_drawn_against_a_range_of_one() {
    let style = kit()
        .values(vec![5.0, 5.0, 5.0])
        .height(50.0)
        .resolved_style();
    assert_eq!(style.min, 5.0);
    assert_eq!(style.max, 5.0);
    assert_eq!(style.range, 1.0, "v1's rule for an empty range");
}

/// An empty series is drawn against (0, 1) and has no bars at all.
#[test]
fn an_empty_series_draws_no_bars() {
    let style = kit().resolved_style();
    assert_eq!(style.min, 0.0);
    assert_eq!(style.max, 1.0);
    assert_eq!(style.bar_width, 0.0);
    let sparkline = kit().view();
    assert_eq!(sparkline.children().len(), 0, "no bars");
}

/// The props report the series.
#[test]
fn the_props_report_the_series() {
    let kit = kit()
        .values(vec![1.0, 2.0])
        .width(120.0)
        .height(30.0)
        .fill(false)
        .line_width(6.0);
    assert_eq!(kit.props().values, vec![1.0, 2.0]);
    assert_eq!(kit.props().width, Some(120.0));
    assert_eq!(kit.props().height, Some(30.0));
    assert!(!kit.props().fill);
    assert_eq!(kit.props().line_width, 6.0);
}

#[test]
fn the_bars_are_painted_in_v1_s_colour() {
    let sparkline = kit().values(vec![1.0, 2.0]).view();
    let bar_widgets = bars(&sparkline);
    assert_eq!(bar_widgets.len(), 2, "one bar per value");
    for bar in &bar_widgets {
        assert_eq!(
            bar_fill(&**bar),
            Some(paint(floom_core_theme::tokens::color_ext::with_alpha(
                theme().colors().primary,
                tokens().fill_opacity
            ))),
            "v1's default fill is a translucent primary"
        );
    }

    let solid = kit().values(vec![1.0, 2.0]).fill(false).view();
    for bar in bars(&solid) {
        assert_eq!(
            bar_fill(&*bar),
            Some(paint(theme().colors().primary)),
            "and a solid primary when `fill` is off"
        );
    }
}

#[test]
fn the_bars_have_v1_s_heights() {
    let sparkline = kit()
        .values(vec![1.0, 2.0, 3.0])
        .width(100.0)
        .height(50.0)
        .view();
    let bars = bars(&sparkline);
    assert_eq!(bars.len(), 3);
    let heights: Vec<Option<f32>> = bars.iter().map(|b| bar_size(&**b).1).collect();
    assert_eq!(heights[0], Some(2.0), "the smallest value is floored at 2");
    assert_eq!(
        heights[1],
        Some(25.0),
        "the middle value is half the height"
    );
    assert_eq!(
        heights[2],
        Some(50.0),
        "the largest value is the full height"
    );
}

#[test]
fn the_bars_have_v1_s_width() {
    let sparkline = kit()
        .values(vec![1.0, 2.0, 3.0])
        .width(100.0)
        .height(50.0)
        .view();
    for bar in bars(&sparkline) {
        let (width, _) = bar_size(&*bar);
        let width = width.expect("a bar asks for its own width");
        assert!(
            (width - 92.0 / 3.0).abs() < 1e-4,
            "the bar's width: {width}"
        );
    }
}

#[test]
fn the_sparkline_declares_its_rendered_size() {
    let request = kit().width(120.0).height(30.0).view();
    let request = request.layout_request().expect("a request");
    assert_eq!(request.width.and_then(|w| w.px()), Some(120.0));
    assert_eq!(request.height.and_then(|h| h.px()), Some(30.0));
}

#[test]
fn the_gap_between_the_bars_is_v1_s() {
    let request = kit().line_width(6.0).view();
    let request = request.layout_request().expect("a request");
    assert_eq!(request.gap, Some(6.0), "the props' `line_width`");
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialSparkline::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material sparkline read the app-wide signal, not a snapshot"
    );
}
