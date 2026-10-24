//! The ported Cupertino banner -- a tinted strip with an icon, a message and an
//! action.
//!
//! The shape is the engine's vocabulary, so what is tested here is the
//! translation and the pixels: the tokens reach the style, the strip is painted
//! v1's tint, the three pieces are painted in v1's inks at v1's sizes, and the
//! slots v1 filled with an empty view are absent here.
//!
//! ## The tripwires
//!
//! `the_strip_is_painted_v1_s_tint` and
//! `a_banner_without_an_icon_or_an_action_has_one_child` would fail if this kit
//! stopped being this kit.

use floom_core::{ElementId, Point, PointerEvent, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder, TestEventContext};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_banner::{banner_accent, CupertinoBanner, CupertinoBannerStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 80.0,
};

const AT: Point = Point { x: 20.0, y: 20.0 };

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    // this test's alone.
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoBanner {
    CupertinoBanner::new().with_theme(theme())
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

/// The row: the message group on the left, the action on the right.
fn row_of(kit: CupertinoBanner) -> Box<dyn floom_core::WidgetAny> {
    let surface = kit.view();
    surface.children().remove(0).build()
}

/// The message group: the icon (if there is one) and the message.
fn group_of(kit: CupertinoBanner) -> Box<dyn floom_core::WidgetAny> {
    let row = row_of(kit);
    row.children().remove(0).build()
}

/// The text and the ink one of the group's texts painted.
fn text_of(kit: CupertinoBanner, index: usize) -> (String, f32, Color) {
    let group = group_of(kit);
    let text = group.children().remove(index).build();
    let ops = ops_of(&*text);
    ops.iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("a banner paints its texts")
}

/// Taps the action, the way the host would.
fn tap(pressable: &mut floom_widgets::Pressable) {
    let mut create = AllocatingCreateContext::new();
    let id = pressable.create(&mut create);
    for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
        let mut cx = TestEventContext::new(id, BOX, phase.into());
        pressable.event(&mut cx, id);
    }
}

// --- the translation ------------------------------------------------------

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoBannerStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Banner) {
        floom_core_theme::component_tokens::ComponentTokens::Banner(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.height, tokens.height, "the token's height");
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(style.font_size, tokens.font_size, "the message's size");
    assert_eq!(
        style.action_font_size, tokens.action_font_size,
        "the action's size"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(style.icon_size, tokens.icon_size, "the icon's size");
    assert_eq!(
        style.text_color,
        theme().colors().on_surface,
        "the message's ink"
    );
    assert_eq!(
        style.action_color,
        theme().colors().primary,
        "the action's ink"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

/// Three of the four accents are fixed RGB and only `error` reads the theme.
/// Cupertino's three are its own -- that is the tripwire keeping the three kits
/// three crates.
#[test]
fn three_accents_are_fixed_literals_and_one_is_the_themes_error() {
    let theme = theme();

    assert_eq!(
        banner_accent(
            &theme,
            floom_core_theme::props::banner::BannerVariant::Error
        ),
        theme.colors().error,
        "error reuses the theme's error role"
    );
    for (variant, rgb) in [
        (
            floom_core_theme::props::banner::BannerVariant::Info,
            (0, 122, 255),
        ),
        (
            floom_core_theme::props::banner::BannerVariant::Success,
            (52, 199, 89),
        ),
        (
            floom_core_theme::props::banner::BannerVariant::Warning,
            (255, 149, 0),
        ),
    ] {
        let (r, g, b) = rgb;
        assert_eq!(
            banner_accent(&theme, variant),
            floom_core_theme::tokens::color::Color::rgb(r, g, b),
            "{variant:?}: v1's literal"
        );
    }
}

/// v1's container colour is the accent at **26/255** -- about 10%, a tint rather
/// than a wash. 26 is a strange number to survive a port by accident, so it is
/// asserted.
#[test]
fn the_strip_is_the_accent_at_26() {
    let theme = theme();
    let style = kit().resolved_style();
    assert_eq!(
        style.container_color,
        floom_core_theme::tokens::color_ext::with_alpha(style.accent, 26),
        "v1's `with_alpha(accent,  26)`"
    );
    assert_eq!(
        (
            style.container_color.r,
            style.container_color.g,
            style.container_color.b
        ),
        (style.accent.r, style.accent.g, style.accent.b),
        "CONTROL: the tint is the accent's colour at a lower alpha -- so the \
         assertion above is about the alpha and not about a different hue"
    );
    assert_ne!(
        style.container_color.a, 255,
        "and it really is translucent -- so the alpha is not being dropped"
    );
}

// --- the pixels -----------------------------------------------------------

/// The strip is painted the colour v1 painted.
#[test]
fn the_strip_is_painted_v1_s_tint() {
    let resolved = kit().resolved_style();
    let surface = kit().view();
    let ops = ops_of(&surface);
    let fill = ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(resolved.container_color)),
        "the strip's fill"
    );
}

/// The message is painted in v1's ink at v1's size.
#[test]
fn the_message_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let (text, size, ink) = text_of(kit().message("Sync failed"), 0);
    assert_eq!(text, "Sync failed");
    assert_eq!(size, resolved.font_size, "v1's message size");
    assert_eq!(ink, paint(resolved.text_color), "v1's message ink");
}

/// The icon is painted in the accent at the token's size.
#[test]
fn the_icon_is_painted_in_the_accent() {
    let resolved = kit().resolved_style();
    let (text, size, ink) = text_of(kit().icon("!").message("Sync failed"), 0);
    assert_eq!(text, "!");
    assert_eq!(size, resolved.icon_size, "the token's icon size");
    assert_eq!(ink, paint(resolved.accent), "the icon's ink is the accent");
}

/// The action is painted in `primary` at the token's size.
#[test]
fn the_action_is_painted_in_primary() {
    let resolved = kit().resolved_style();
    let row = row_of(kit().action("Retry").message("Sync failed"));
    let pressable = row.children().remove(1).build();
    let pressable = pressable
        .as_any()
        .downcast_ref::<floom_widgets::Pressable>()
        .expect("the action is a pressable");
    // The pressable holds the padded surface, which holds the label.
    let surface = pressable.child().build();
    let text = surface.children().remove(0).build();
    let ops = ops_of(&*text);
    let (label, size, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the action paints its label");
    assert_eq!(label, "Retry");
    assert_eq!(size, resolved.action_font_size, "the token's action size");
    assert_eq!(ink, paint(resolved.action_color), "the action's ink");
}

/// The action fires v1's callback when it is tapped.
#[test]
fn the_action_fires_the_callback() {
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter = hits.clone();
    let kit = kit().action("Retry").on_action(move || {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });
    let mut pressable = row_of(kit).children().remove(1).build();
    let pressable = pressable
        .as_any_mut()
        .downcast_mut::<floom_widgets::Pressable>()
        .expect("the action is a pressable");

    tap(pressable);
    assert_eq!(
        hits.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "the callback fired once"
    );
}

// --- the structure --------------------------------------------------------

/// v1 filled the missing icon and the missing action with an `Empty` view, so
/// its row always had two children. Here the slots are absent, so a banner with
/// neither has one child.
#[test]
fn a_banner_without_an_icon_or_an_action_has_one_child() {
    let row = row_of(kit().message("Sync failed"));
    assert_eq!(
        row.children().len(),
        1,
        "v1 mounted an `Empty` view for the missing icon and the missing action, \
         so its row always had two children. `Empty` had no size, so there is no \
         pixel difference -- only a structural one."
    );
}

/// With an icon and an action, the row has two children and the group has two.
#[test]
fn an_icon_and_an_action_fill_the_slots() {
    let row = row_of(kit().icon("!").message("Sync failed").action("Retry"));
    assert_eq!(row.children().len(), 2, "the group and the action");

    let group = group_of(kit().icon("!").message("Sync failed").action("Retry"));
    assert_eq!(group.children().len(), 2, "the icon and the message");
}

/// v1 wrote `gap(gap)` on a container that held one child, so the gap never
/// showed.
#[test]
fn the_strip_has_no_gap() {
    let surface = kit().view();
    let request = surface
        .layout_request()
        .expect("the strip asks for its own space");
    assert_eq!(
        request.gap, None,
        "the gap v1 wrote never had two children to separate"
    );
}

/// The strip's padding and minimum height are v1's.
#[test]
fn the_strip_is_padded_and_has_a_minimum_height() {
    let resolved = kit().resolved_style();
    let surface = kit().view();
    let request = surface
        .layout_request()
        .expect("the strip asks for its own space");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets::uniform(f64::from(resolved.padding))),
        "v1's `.padding(padding)`"
    );
    assert_eq!(
        request.min_height,
        Some(floom_core::Length::Px(resolved.height)),
        "v1's `.min_height(tokens.height)`"
    );
}

// --- the props ------------------------------------------------------------

/// The props report the text, and the callback is stored in the v1 shape too.
#[test]
fn the_props_report_the_message_and_the_callback() {
    let kit = kit()
        .icon("!")
        .message("Sync failed")
        .action("Retry")
        .on_action(|| {});
    assert_eq!(kit.props().message, "Sync failed");
    assert_eq!(kit.props().icon.as_deref(), Some("!"));
    assert_eq!(kit.props().action_label.as_deref(), Some("Retry"));
    assert!(
        kit.props().on_action.is_some(),
        "the v1-shaped copy is set, so `props()` reports what v1 reported"
    );
}

/// v1's `single_line` never reached the paint: the message was never
/// truncated.
#[test]
fn single_line_is_carried_but_never_truncates() {
    let kit = kit().message("Sync failed").single_line(true);
    assert!(kit.props().single_line, "the prop is kept");
    let (text, _, _) = text_of(kit, 0);
    assert_eq!(
        text, "Sync failed",
        "v1 did not truncate the message and neither does this"
    );
}

// --- the theme ------------------------------------------------------------

/// The theme: v1's Cupertino banner took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    // exactly the thing this test is about.
    assert_ne!(
        CupertinoBanner::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino banner took a snapshot, not the app-wide signal"
    );
}
