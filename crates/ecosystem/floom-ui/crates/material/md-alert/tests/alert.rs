//! The ported Material alert banner.
//!
//! Two claims, and the second is the one that would have been easy to get wrong:
//!
//! * the tokens and the four-variant accent table reach the engine;
//! * **the dismiss slot is absent when the alert is not dismissible.** v1 mounted
//!   an empty `Label` there, so the row always had two children and the 8px gap was
//!   spent on nothing. Reproducing that faithfully would mean reproducing a bug, and
//!   dropping it is a behaviour change rather than a translation — so it is pinned
//!   here where the difference is visible.

use floom_core::Widget;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::alert::AlertVariant;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;
use floom_widgets::{Stack, Surface};

use floom_widget_md_alert::{alert_accent, MdAlert};

/// Activate Material and hand back a theme that will read it.
///
/// `MultiTheme::new_for(ds)` does **not** select `ds` (the porting plan's
/// correction 9) — it reads a process-global at every access.
fn material() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn alert() -> MdAlert {
    let _ = material();
    MdAlert::new()
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

// --- the tokens -----------------------------------------------------------

#[test]
fn the_tokens_reach_the_engine_surface() {
    let theme = material();
    let style = alert().resolved_style();
    let ComponentTokens::Alert(tokens) = theme.component(ComponentId::Alert) else {
        unreachable!("ComponentId::Alert resolves to AlertTokens")
    };

    assert_eq!(style.padding, tokens.padding);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.font_size, tokens.font_size);
    assert_eq!(style.title_font_size, tokens.title_font_size);
    assert_eq!(style.text_color, theme.colors().on_surface);
    assert_eq!(style.design_system, "material");

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

/// v1's container colour is the accent at **26/255** — about 10%, a tint rather than
/// a wash. 26 is a strange number to survive a port by accident, so it is asserted.
#[test]
fn the_container_is_the_accent_at_26() {
    let theme = material();
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

// --- the accent table -----------------------------------------------------

/// Three of the four accents are fixed RGB and only `error` reads the theme. Kept
/// as v1 wrote it: they are the design, not an oversight, and resolving them through
/// the theme would change three kits' colours.
#[test]
fn three_accents_are_fixed_literals_and_one_is_the_themes_error() {
    let theme = material();

    assert_eq!(
        alert_accent(&theme, AlertVariant::Error),
        theme.colors().error,
        "error reuses the theme's error role"
    );
    for (variant, rgb) in [
        (AlertVariant::Info, (33, 150, 243)),
        (AlertVariant::Success, (76, 175, 80)),
        (AlertVariant::Warning, (255, 152, 0)),
    ] {
        let (r, g, b) = rgb;
        assert_eq!(
            alert_accent(&theme, variant),
            floom_core_theme::tokens::color::Color::rgb(r, g, b),
            "{variant:?}: v1's literal"
        );
    }

    // The control: four variants really are four different accents, so the table is
    // not accidentally constant.
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

/// The variant reaches the style, and through it the tint.
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

// --- the tree -------------------------------------------------------------

/// The outer shape: a tinted, rounded `Surface` holding one row.
#[test]
fn the_banner_is_a_surface_holding_one_row() {
    let surface = alert().view();
    assert_eq!(surface.child_count(), 1, "the banner holds one row");

    let row = surface.children()[0].build();
    assert!(row.as_any().is::<Stack>(), "the row is a horizontal stack");
}

/// The dismiss slot, which is the whole point of the difference from v1.
///
/// v1 wrote `if dismissible { MdIcon.. } else { Label::new("") }`, so the row had
/// two children either way and the 8px gap after the content was spent on an empty
/// label. Here the slot is absent, so a non-dismissible banner's row has **one**
/// child. A port that reproduced v1 exactly would pass a "two children" test and be
/// wrong; a port that dropped the dismiss button entirely would also pass one. This
/// asserts both halves.
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

/// The dismiss affordance is a `Pressable` around an icon — the 2.0 spelling of
/// v1's `.on_event(listener::Click, ..)` chain, and the site of the removed
/// `MdIcon::view()`.
#[test]
fn the_dismiss_affordance_is_a_pressable_icon() {
    let banner = alert().dismissible(true).view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    let dismiss = row.children()[1].build();

    assert!(
        dismiss.as_any().is::<floom_widgets::Pressable>(),
        "a clickable icon is the icon inside a `Pressable` — v1 chained `.view() \\
         onto `MdIcon` and then used `Decorators`, and 2.0 has neither"
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

/// The content row: the accent dot and the text column, 12px apart.
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

/// The text column: the title above the message, 4px apart, title bold.
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

/// The message is the second line, and it is not bold.
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

// --- the props ------------------------------------------------------------

/// `on_dismiss` is stored twice — a v1 `Box` for the props and an
/// `Arc<dyn Fn() + Send + Sync>` for the engine — from one closure.
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

/// A non-dismissible alert with a callback is still non-dismissible: v1's
/// `dismissible` flag is what mounted the button, and the callback alone did not.
#[test]
fn a_callback_alone_does_not_mount_the_button() {
    let alert = alert().on_dismiss(|| {});
    assert!(!alert.props().dismissible, "CONTROL: not dismissible");
    let banner = alert.view();
    let row = banner.children()[0].build();
    let row = row.as_any().downcast_ref::<Stack>().unwrap().clone();
    assert_eq!(row.child_count(), 1, "so there is no button");
}

/// `Text` under the name the assertions above use.
type Text_ = floom_widgets::Text;
