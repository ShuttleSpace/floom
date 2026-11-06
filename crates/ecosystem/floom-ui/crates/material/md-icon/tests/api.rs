use floom_core::{Color as EngineColor, Rect, Widget};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widgets::testkit::{paint_snapshot, semantics_snapshot, AllocatingCreateContext};

use floom_widget_md_icon::{MdIcon, MD_ICON_VIEW_BOX};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

fn material() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

fn allocate(widget: &dyn Widget) -> (floom_core::ElementId, AllocatingCreateContext) {
    let mut create = AllocatingCreateContext::new();
    let id = widget.create(&mut create);
    (id, create)
}

#[test]
fn icon_exposes_accessible_builder_api_and_theme_style() {
    let theme = material();
    let icon = MdIcon::new("home").size(20.0).label("Home");
    let style = icon.with_theme(theme.clone()).resolved_style();

    assert_eq!(style.color, theme.colors().on_surface_variant);
    assert_eq!(style.size, 20.0);
}

#[test]
fn the_size_falls_back_to_the_icon_tokens() {
    use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};

    let theme = material();
    let icon = MdIcon::new("home").with_theme(theme.clone());
    let style = icon.resolved_style();

    let ComponentTokens::Icon(tokens) = theme.component(ComponentId::Icon) else {
        panic!("ComponentId::Icon must resolve to IconTokens")
    };
    assert_eq!(
        style.size, tokens.size,
        "with no explicit size the token decides"
    );
}

#[test]
fn a_label_implies_the_icon_is_not_decorative() {
    let icon = MdIcon::new("home").label("Home");
    assert!(!icon.props().decorative);

    let plain = MdIcon::new("home");
    assert!(plain.props().decorative, "an unlabelled icon is decoration");
}

#[test]
fn resolve_produces_a_widget_sized_by_the_theme() {
    let theme = material();
    let icon = MdIcon::new("home").size(20.0).with_theme(theme.clone());
    let widget = icon.resolve();
    let req = widget
        .layout_request()
        .expect("an icon always wants a size");

    assert_eq!(req.width, Some(floom_core::Length::Px(20.0)));
    assert_eq!(req.height, Some(floom_core::Length::Px(20.0)));
}

#[test]
fn resolve_fills_the_outline_in_the_themes_foreground() {
    let theme = material();
    let icon = MdIcon::new("home").with_theme(theme.clone());
    let widget = icon.resolve();
    let (id, _create) = allocate(&widget);

    let ops = paint_snapshot(&widget, id, BOX);
    assert!(
        ops.starts_with("Path"),
        "expected a filled path, got {ops:?}"
    );

    let expected = theme.colors().on_surface_variant;
    let hex = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        expected.r, expected.g, expected.b, expected.a
    );
    assert!(
        ops.contains(&hex),
        "expected the glyph to be filled {hex}, got {ops:?}"
    );
}

#[test]
fn a_labelled_resolved_icon_reports_its_name() {
    let icon = MdIcon::new("home").label("Home");
    let widget = icon.resolve();
    let (id, _create) = allocate(&widget);
    assert_eq!(
        semantics_snapshot(&widget, id, BOX),
        r#"label="Home" (0.0, 0.0, 24.0, 24.0)"#
    );
}

#[test]
fn an_unlabelled_resolved_icon_is_decorative() {
    let icon = MdIcon::new("home");
    let widget = icon.resolve();
    let (id, _create) = allocate(&widget);
    assert_eq!(semantics_snapshot(&widget, id, BOX), "decorative");
}

#[test]
fn an_unknown_name_falls_back_to_the_dot_and_still_paints() {
    let icon = MdIcon::new("definitely_not_a_real_glyph");
    let widget = icon.resolve();
    let (id, _create) = allocate(&widget);

    assert!(!widget.path().is_empty(), "the fallback glyph must exist");
    assert!(
        paint_snapshot(&widget, id, BOX).starts_with("Path"),
        "the fallback must still paint"
    );
}

#[test]
fn every_named_glyph_in_the_set_parses() {
    for name in [
        "menu",
        "close",
        "check",
        "check_circle",
        "search",
        "home",
        "settings",
        "add",
        "delete",
        "arrow_back",
        "arrow_forward",
        "star",
        "favorite",
        "person",
        "info",
        "warning",
        "edit",
        "visibility",
        "notifications",
        "more_vert",
        "chevron_right",
        "chevron_left",
        "expand_more",
        "favorite_border",
        "account_circle",
        "refresh",
        "visibility_off",
        "folder",
        "description",
        "photo_library",
    ] {
        let icon = MdIcon::new(name);
        let widget = icon.resolve();
        assert!(
            !widget.path().is_empty(),
            "{name:?} produced an empty path: {}",
            icon.path_data()
        );
    }
}

#[test]
fn name_normalisation_is_underscore_insensitive() {
    let a = MdIcon::new("expand_more");
    let b = MdIcon::new("expandMore");
    let c = MdIcon::new("Expand More");
    assert_eq!(a.path_data(), b.path_data());
    assert_eq!(a.path_data(), c.path_data());
}

#[test]
fn the_view_box_is_the_material_24() {
    assert_eq!(MD_ICON_VIEW_BOX.x0, 0.0);
    assert_eq!(MD_ICON_VIEW_BOX.y0, 0.0);
    assert_eq!(MD_ICON_VIEW_BOX.x1, 24.0);
    assert_eq!(MD_ICON_VIEW_BOX.y1, 24.0);
}

#[test]
fn the_v1_markup_helper_still_matches_the_glyph_data() {
    let data = floom_widget_md_icon::MdIcon::new("home").path_data();
    let markup = floom_widget_md_icon::md_icon_svg("home");
    assert!(
        markup.contains(data),
        "markup must embed the same path data"
    );
}

#[test]
fn resolving_twice_is_stable() {
    let icon = MdIcon::new("home").size(20.0).label("Home");
    let a = icon.resolve();
    let b = icon.resolve();
    assert_eq!(a.path(), b.path());
    assert_eq!(a.size_of(), b.size_of());
}

/// Small helper: the icon's configured edge length, which `layout_request`
/// already folds into its own numbers.
trait SizeOf {
    fn size_of(&self) -> Option<f32>;
}

impl SizeOf for floom_widgets::VectorIcon {
    fn size_of(&self) -> Option<f32> {
        self.layout_request()
            .and_then(|r| r.width)
            .and_then(|l| l.px())
    }
}

/// Keeps the engine `Color` import honest: the conversion under test is the
/// design-system byte struct becoming an engine byte struct.
#[test]
fn design_system_colour_becomes_an_engine_colour() {
    let theme = material();
    let style = MdIcon::new("home")
        .with_theme(theme.clone())
        .resolved_style();
    let engine = EngineColor {
        r: style.color.r,
        g: style.color.g,
        b: style.color.b,
        a: style.color.a,
    };
    assert_eq!(
        engine.r,
        theme.colors().on_surface_variant.r,
        "the resolved colour must be the theme's, field for field"
    );
}
