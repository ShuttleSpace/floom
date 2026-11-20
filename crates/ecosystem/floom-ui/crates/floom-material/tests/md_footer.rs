#![cfg(feature = "footer")]
use floom_core::{Edges, Widget};
use floom_types::{Insets, Rect};
use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::layout::{layout_signal, LayoutSlot};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_material::widgets::footer::MdFooter;
use floom_signal::SignalGet;

const BAR: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 56.0,
};

fn footer() -> MdFooter {
    MdFooter::new()
}

fn tokens() -> floom_core_theme::component_tokens::FooterTokens {
    let theme = MultiTheme::new_for(DesignSystem::Material);
    match theme.component(floom_core_theme::component_tokens::ComponentId::Footer) {
        floom_core_theme::component_tokens::ComponentTokens::Footer(t) => t,
        _ => panic!("the footer component must resolve to FooterTokens"),
    }
}

/// The line this whole port exists for: a rule along the **top** edge, at the
/// theme's thickness, in the theme's outline colour.
#[test]
fn the_separator_is_a_top_border_at_the_token_thickness() {
    let (width, colour) = footer()
        .border()
        .expect("v1's FooterProps::default() has bordered: true");
    assert_eq!(width, tokens().border, "the token's top-border thickness");
    assert_eq!(
        footer().border_edges(),
        Edges::TOP,
        "and only the top edge: a footer is not a box outlined all round"
    );
    let theme = MultiTheme::new_for(DesignSystem::Material);
    use floom_core_theme::theme::Theme;
    assert_eq!(colour.r, theme.colors().outline_variant.r);
    assert_eq!(colour.a, theme.colors().outline_variant.a);
}

/// And it reaches the widget as an edge set, not as a whole outline — the same
/// distinction `floom-widgets`' own suite draws, seen from a kit.
#[test]
fn the_separator_reaches_the_surface_as_a_top_edge() {
    let surface = footer().view();
    let border = surface
        .decoration()
        .border_spec()
        .expect("a bordered footer declares a border");
    assert_eq!(border.edges(), Edges::TOP);
    assert_eq!(border.width(), tokens().border);

    assert_ne!(border.edges(), Edges::ALL);
}

#[test]
fn an_unbordered_footer_has_no_separator_at_all() {
    let bar = footer().bordered(false);
    assert_eq!(bar.border(), None);
    assert_eq!(bar.border_edges(), Edges::NONE);
    assert!(
        bar.view().decoration().border_spec().is_none(),
        "and nothing is declared on the widget"
    );
}

#[test]
fn the_bar_declares_its_own_height_and_full_width() {
    let request = footer().request();
    assert_eq!(
        request.height.and_then(|h| h.px()),
        Some(tokens().height),
        "56dp for material, from the token"
    );
    assert!(
        request.width.map(|w| w.is_pct()).unwrap_or(false),
        "and the full width of its parent, v1's `width_pct(100.0)`: {request:?}"
    );
}

#[test]
fn the_content_is_centred_on_both_axes() {
    use floom_widgets::Align;
    let request = footer().request();
    assert_eq!(request.main_align, Some(Align::Center), "vertically");
    assert_eq!(request.cross_align, Some(Align::Center), "horizontally");

    let declared = footer().view().layout_request().expect("a request");
    assert_eq!(declared.main_align, Some(Align::Center));
    assert_eq!(declared.cross_align, Some(Align::Center));
}

#[test]
fn the_padding_is_inside_the_box_not_on_the_child() {
    assert_eq!(
        footer().padding(),
        Insets::uniform(f64::from(tokens().pad)),
        "from the token when the props defer"
    );
    let declared = footer().view().layout_request().expect("a request");
    assert_eq!(
        declared.padding,
        Some(Insets::uniform(f64::from(tokens().pad))),
        "and it arrives as the surface's own padding"
    );
}

#[test]
fn the_props_win_over_the_tokens_and_the_tokens_win_by_default() {
    assert_eq!(
        footer().resolved_style().height,
        tokens().height,
        "the props defer by default"
    );
    assert_eq!(footer().height(72.0).resolved_style().height, 72.0);
    assert_eq!(
        footer().pad(4.0).resolved_style().padding,
        4.0,
        "padding too"
    );
    assert_eq!(
        footer().height(72.0).request().height.and_then(|h| h.px()),
        Some(72.0),
        "and the request follows the props, not the token"
    );
}

#[test]
fn only_an_elevated_footer_casts_a_shadow() {
    assert_eq!(footer().shadow(), None, "v1: `if elevation > 0`");
    let shadow = footer().elevation(1).shadow().expect("elevated");
    assert_eq!(shadow.blur, 6.0, "v1: box_shadow_blur(6.0)");
    assert_eq!(shadow.spread, 0.0, "v1: box_shadow_spread(0.0)");
    assert_eq!(shadow.offset_x, 0.0, "v1 set no offset");
    assert_eq!(shadow.offset_y, 0.0, "v1 set no offset");
    assert_eq!(shadow.color.a, 30, "v1: with_alpha(BLACK, 30)");
    assert!(footer()
        .elevation(2)
        .view()
        .decoration()
        .drop_shadow()
        .is_some());
}

#[test]
fn the_content_goes_inside_the_box() {
    let empty = footer().view();
    assert_eq!(empty.child_count(), 0, "no content, no children");

    let filled = footer()
        .child(|| Box::new(floom_widgets::Text::new("Cancel  ·  Save")))
        .view();
    assert_eq!(filled.child_count(), 1, "the caller's content is inside");
}

#[test]
fn a_bordered_elevated_bar_with_content_still_declares_all_three() {
    let surface = footer()
        .bordered(true)
        .elevation(1)
        .child(|| Box::new(floom_widgets::Text::new("Save")))
        .view();
    let request = surface.layout_request().expect("a request");
    assert!(request.padding.is_some(), "padding");
    assert!(request.height.is_some(), "extent");
    assert!(surface.decoration().border_spec().is_some(), "separator");
    assert!(surface.decoration().drop_shadow().is_some(), "elevation");
    assert_eq!(surface.child_count(), 1, "content");
}

#[test]
fn building_the_footer_publishes_its_height() {
    let _ = layout_signal().get().footer;
    footer().height(72.0).view();
    assert_eq!(
        layout_signal().get().footer,
        72.0,
        "`md-main` reads this to inset the content region"
    );
}

/// The token's own height when the props defer — the value a caller who never
/// sets a height publishes.
#[test]
fn the_published_height_defaults_to_the_token() {
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let _ = theme;
    MdFooter::new().view();
    assert_eq!(
        layout_signal().get().footer,
        tokens().height,
        "the material token for the default theme"
    );
    let _ = LayoutSlot::Footer;
}

/// The whole translation in one snapshot: a filled bar, at the token's extent,
/// with the separator band along its top edge.
#[test]
fn the_ported_bar_paints_a_fill_and_a_top_band() {
    let surface = footer().view();
    let id = surface.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(&surface, id, BAR);
    assert!(
        painted.contains("RoundedRect") || painted.contains("Rect"),
        "the fill: {painted}"
    );
    assert!(
        painted.contains("Rect (0.0, 0.0, 200.0,"),
        "and a band at the very top, flush with the edge: {painted}"
    );
}

#[test]
fn an_unbordered_bar_paints_no_band() {
    let surface = footer().bordered(false).view();
    let id = surface.create(&mut AllocatingCreateContext::new());
    let mut recorder = SceneRecorder::new(BAR);
    surface.paint(&mut recorder, id);
    recorder.finish();
    let lines: Vec<String> = recorder.ops().iter().map(|op| op.snapshot_line()).collect();
    assert!(
        lines.iter().filter(|l| l.starts_with("Rect ")).count() <= 1,
        "at most the fill, and no band: {lines:?}"
    );
}
