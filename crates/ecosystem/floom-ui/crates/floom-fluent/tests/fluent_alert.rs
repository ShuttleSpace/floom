#![cfg(feature = "alert")]
//! The ported Fluent alert banner.

use floom_core::Widget;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::alert::AlertVariant;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;
use floom_widgets::{Stack, Surface};

use floom_fluent::widgets::alert::{alert_accent, FluentAlert};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn alert() -> FluentAlert {
    let _ = theme();
    FluentAlert::new()
        .title("Sync failed")
        .message("Try again in a moment")
}

fn to_paint(c: floom_core_theme::tokens::color::Color) -> floom_types::Color {
    floom_types::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

#[test]
fn the_tokens_reach_the_engine_surface() {
    let theme = theme();
    let style = alert().resolved_style();
    let ComponentTokens::Alert(tokens) = theme.component(ComponentId::Alert) else {
        unreachable!("ComponentId::Alert resolves to AlertTokens")
    };

    assert_eq!(style.padding, tokens.padding);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.font_size, tokens.font_size);
    assert_eq!(style.title_font_size, tokens.title_font_size);
    assert_eq!(style.text_color, theme.colors().on_surface);
    assert_eq!(style.design_system, "fluent");

    let surface = alert().view();
    let request = surface
        .layout_request()
        .expect("a padded surface has a request");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets::uniform(f64::from(tokens.padding))),
        "v1 set `.padding(padding)` on the box"
    );
    assert_eq!(
        surface.decoration().corner_radius(),
        tokens.border_radius,
        "and the radius"
    );
}

#[test]
fn the_container_is_the_accent_at_26() {
    let theme = theme();
    let variant = alert().props().variant;
    let style = alert().resolved_style();
    assert_eq!(
        style.container_color,
        with_alpha(alert_accent(&theme, variant), 26),
        "v1's `with_alpha(accent, 26)`, on whatever variant this alert carries"
    );
    assert_eq!(
        (
            style.container_color.r,
            style.container_color.g,
            style.container_color.b
        ),
        (style.accent.r, style.accent.g, style.accent.b),
        "CONTROL: the tint is the accent's colour at a lower alpha — so the \
         assertion above is about the alpha and not about a different hue"
    );
    assert_ne!(
        style.container_color.a, 255,
        "and it really is translucent — so the alpha is not being dropped"
    );
}

#[test]
fn three_accents_are_fixed_literals_and_one_is_the_themes_error() {
    let theme = theme();

    assert_eq!(
        alert_accent(&theme, AlertVariant::Error),
        theme.colors().error,
        "error reuses the theme's error role"
    );
    for (variant, rgb) in [
        (AlertVariant::Info, (0, 120, 212)),
        (AlertVariant::Success, (16, 124, 16)),
        (AlertVariant::Warning, (255, 185, 0)),
    ] {
        let (r, g, b) = rgb;
        assert_eq!(
            alert_accent(&theme, variant),
            floom_core_theme::tokens::color::Color::rgb(r, g, b),
            "{variant:?}: v1's literal"
        );
    }

    let all = [
        alert_accent(&theme, AlertVariant::Error),
        alert_accent(&theme, AlertVariant::Info),
        alert_accent(&theme, AlertVariant::Success),
        alert_accent(&theme, AlertVariant::Warning),
    ];
    for i in 0..all.len() {
        for j in (i + 1)..all.len() {
            assert_ne!(all[i], all[j], "accents {} and {} differ", i, j);
        }
    }
}

#[test]
fn the_variant_selects_the_accent_and_therefore_the_tint() {
    let error = alert().variant(AlertVariant::Error).resolved_style();
    let warning = alert().variant(AlertVariant::Warning).resolved_style();
    assert_ne!(error.accent, warning.accent, "two variants, two accents");
    assert_ne!(
        error.container_color, warning.container_color,
        "and therefore two tints — so the variant is not read past the accent"
    );
}

#[test]
fn the_banner_is_a_surface_holding_one_row() {
    let surface = alert().view();
    assert_eq!(surface.child_count(), 1, "the banner holds one row");

    let row = surface.children()[0].build();
    assert!(row.as_any().is::<Stack>(), "the row is a horizontal stack");
}

#[test]
fn the_dismiss_slot_is_absent_unless_the_alert_is_dismissible() {
    let plain = alert().view();
    let plain_row = plain.children()[0].build();
    let plain_row = plain_row.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(
        plain_row.child_count(),
        1,
        "v1 mounted an empty `Label` here, so its row had two children and 8px of \\
         gap after the content. That is v1's bug; this port drops it."
    );

    let dismissible = alert().dismissible(true).view();
    let row = dismissible.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(row.child_count(), 2, "and a dismissible one has the button");
}

#[test]
fn the_dismiss_affordance_is_a_pressable_icon() {
    let banner = alert().dismissible(true).view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    let dismiss = row.children()[1].build();

    assert!(
        dismiss.as_any().is::<floom_widgets::Pressable>(),
        "a clickable icon is the icon inside a `Pressable` — v1 chained `.view() \\
         onto `FluentIcon` and then used `Decorators`, and 2.0 has neither"
    );

    let pressable = dismiss
        .as_any()
        .downcast_ref::<floom_widgets::Pressable>()
        .unwrap();
    assert_eq!(
        pressable.semantics_hint().label.as_deref(),
        Some("Dismiss"),
        "and it is named, which v1's bare click handler could not be"
    );
}

#[test]
fn the_content_is_a_dot_and_a_text_column() {
    let banner = alert().view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    let content = row.children()[0].build();
    let content = content.as_any().downcast_ref::<Stack>().unwrap().clone();

    assert_eq!(content.child_count(), 2, "the dot and the text column");

    let dot = content.children()[0].build();
    let dot = dot.as_any().downcast_ref::<Surface>().unwrap().clone();
    let dot_request = dot.layout_request().expect("the dot is sized");
    assert_eq!(
        dot_request.width,
        Some(floom_types::Length::Px(10.0)),
        "v1's `.width(10.0).height(10.0)`"
    );
    assert_eq!(
        dot.decoration().corner_radius(),
        9999.0,
        "and v1's `.border_radius(9999.0)`, which is how a circle is spelled here"
    );
}

#[test]
fn the_text_column_is_a_bold_title_over_a_message() {
    let banner = alert().view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    let content = row.children()[0].build();
    let content = content.as_any().downcast_ref::<Stack>().unwrap().clone();
    let column = content.children()[1].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();

    assert_eq!(column.child_count(), 2, "title and message");

    let title = column.children()[0].build();
    let title = title.as_any().downcast_ref::<Text_>().unwrap();
    assert_eq!(
        title.font_weight(),
        floom_types::FontWeight::Bold,
        "v1's `FontWeight::Bold`"
    );
    assert_eq!(title.font_size(), alert().resolved_style().title_font_size);
    assert_eq!(title.content(), "Sync failed");
}

#[test]
fn the_message_is_the_second_line_and_not_bold() {
    let banner = alert().view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    let content = row.children()[0].build();
    let content = content.as_any().downcast_ref::<Stack>().unwrap().clone();
    let column = content.children()[1].build();
    let column = column.as_any().downcast_ref::<Stack>().unwrap().clone();

    let message = column.children()[1].build();
    let message = message.as_any().downcast_ref::<Text_>().unwrap();
    assert_eq!(message.content(), "Try again in a moment");
    assert_eq!(message.font_size(), alert().resolved_style().font_size);
    assert_ne!(
        message.font_weight(),
        floom_types::FontWeight::Bold,
        "CONTROL: the title is bold and this is not, so the two are being read \\
         separately rather than both inheriting one value"
    );
}

#[test]
fn the_props_report_the_dismiss_callback() {
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let alert = {
        let hits = hits.clone();
        alert().on_dismiss(move || {
            hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
    };
    assert!(
        alert.props().on_dismiss.is_some(),
        "the v1-shaped copy is set, so `props()` reports what v1 reported"
    );
}

#[test]
fn a_callback_alone_does_not_mount_the_button() {
    let alert = alert().on_dismiss(|| {});
    assert!(!alert.props().dismissible, "CONTROL: not dismissible");
    let banner = alert.view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(row.child_count(), 1, "so there is no button");
}

type Text_ = floom_widgets::Text;
