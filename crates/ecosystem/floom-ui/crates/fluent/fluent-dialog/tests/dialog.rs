//! Fluent's dialog as a kit crate.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{ElementId, Widget};
use floom_types::{OverlayKind, Rect};

use floom_widgets::testkit::RecordingUpdateContext;
use floom_widgets::text_input::{read_flag, write_flag};

use floom_core_theme::theme::Theme;
use floom_widget_fluent_dialog::{to_engine_weight, FluentDialog};

const VIEWPORT: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 800.0,
    y1: 600.0,
};

fn dialog() -> FluentDialog {
    FluentDialog::new()
        .title("Delete file")
        .content("This cannot be undone.")
}

#[test]
fn open_and_is_open_read_and_write_one_flag() {
    let mut dialog = dialog();
    assert!(!dialog.is_open(), "a dialog starts shut");
    dialog = dialog.open(true);
    assert!(dialog.is_open(), "and `open(true)` opens it");
    assert!(
        read_flag(&dialog.open_flag()),
        "and the flag a caller holds agrees"
    );
}

#[test]
fn close_reports_whether_it_changed_anything() {
    let mut dialog = dialog().open(true);
    assert!(dialog.close(), "open to shut");
    assert!(!dialog.close(), "and then nothing");
    assert!(!dialog.is_open());
}

/// quietly diverge — which is the bug a shared flag is supposed to prevent.
#[test]
fn the_flag_is_one_allocation() {
    let dialog = dialog();
    assert!(Arc::ptr_eq(&dialog.open_flag(), &dialog.open_flag()));
}

#[test]
fn a_flag_from_outside_is_shared_not_copied() {
    let external = write_flag(&floom_widgets::shared_flag(false), true);
    assert!(external);

    let flag = floom_widgets::shared_flag(false);
    let mut dialog = dialog().with_open_flag(flag.clone());
    assert!(Arc::ptr_eq(&flag, &dialog.open_flag()), "one allocation");

    write_flag(&flag, true);
    assert!(dialog.is_open(), "so an external trigger opens it");

    write_flag(&flag, false);
    assert!(!dialog.is_open(), "and closes it");
}

#[test]
fn taking_an_external_flag_seeds_the_props() {
    let flag = floom_widgets::shared_flag(true);
    let dialog = dialog().with_open_flag(flag);
    assert!(dialog.props().open, "so `props()` agrees with the flag");
}

#[test]
fn the_viewed_dialog_asks_for_a_modal_dialog_over_the_viewport() {
    let mut view = dialog().open(true).view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    view.update(&mut cx, ElementId::new(1, 1));

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one overlay asked for");
    match opened[0] {
        floom_widgets::testkit::OverlayCall::Open {
            kind,
            modal,
            bounds,
            ..
        } => {
            assert_eq!(*kind, OverlayKind::Dialog);
            assert!(
                *modal,
                "modal — so an outside press does NOT dismiss it, unlike v1"
            );
            assert_eq!(
                *bounds, VIEWPORT,
                "and it covers the window, not its own box"
            );
        }
        _ => panic!("expected an open, got a close"),
    }
}

#[test]
fn a_shut_dialog_asks_for_nothing() {
    let mut view = dialog().view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    view.update(&mut cx, ElementId::new(1, 1));
    assert!(!cx.opened_any_overlay());
}

#[test]
fn a_host_dismissal_clears_the_caller_s_flag() {
    let mut dialog = dialog().open(true);
    let flag = dialog.open_flag();
    let mut view = dialog.view();

    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    let id = ElementId::new(1, 1);
    view.update(&mut cx, id);
    let granted = cx.opened_overlays()[0].handle();
    view.overlay_opened(&mut cx, id, granted);
    assert!(read_flag(&flag), "open");

    view.overlay_closed(
        &mut cx,
        id,
        granted,
        Box::new(floom_widgets::Text::new("content")),
    );
    assert!(
        !read_flag(&flag),
        "and the host taking it down clears the flag the caller holds"
    );
}

#[test]
fn the_flag_opens_it_again_after_a_dismissal() {
    let mut dialog = dialog().open(true);
    let flag = dialog.open_flag();
    let mut view = dialog.view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    let id = ElementId::new(1, 1);

    view.update(&mut cx, id);
    let granted = cx.opened_overlays()[0].handle();
    view.overlay_opened(&mut cx, id, granted);
    view.overlay_closed(
        &mut cx,
        id,
        granted,
        Box::new(floom_widgets::Text::new("content")),
    );

    cx.clear();
    write_flag(&flag, true);
    view.update(&mut cx, id);
    assert_eq!(
        cx.opened_overlays().len(),
        1,
        "so a caller can reopen it, which is the whole point of the flag being shared"
    );
}

#[test]
fn the_card_is_a_bordered_surface() {
    let mut view = dialog().open(true).view();
    let mut cx = RecordingUpdateContext::with_viewport(VIEWPORT, VIEWPORT);
    view.update(&mut cx, ElementId::new(1, 1));
    let content = match cx.opened_overlays()[0] {
        floom_widgets::testkit::OverlayCall::Open { content, .. } => content,
        _ => panic!("expected an open"),
    };
    assert!(
        content.as_any().is::<floom_widgets::Surface>(),
        "v1's card was a surface with a max width, a padding, a radius and a shadow"
    );
}

#[test]
fn the_tokens_come_from_dialog_tokens() {
    let style = dialog().resolved_style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Fluent,
    );
    let floom_core_theme::component_tokens::ComponentTokens::Dialog(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Dialog)
    else {
        panic!("ComponentId::Dialog must resolve to DialogTokens");
    };
    assert_eq!(style.max_width, tokens.max_width);
    assert_eq!(style.padding, tokens.padding);
}

#[test]
fn the_call_site_numbers_are_v1s() {
    let style = dialog().resolved_style();
    assert_eq!(style.action_padding_horizontal, 12.0);
    assert_eq!(style.action_padding_vertical, 10.0);
    assert_eq!(style.action_radius, 8.0);
    assert_eq!(style.action_gap, 8.0);
    assert_eq!(style.close_padding, 4.0);
    assert_eq!(style.close_icon_size, 20.0);
    assert_eq!(style.shadow_alpha, 30, "v1's `with_alpha(black, 30)`");
}

#[test]
fn the_content_is_deliberately_lighter_than_the_title() {
    let style = dialog().resolved_style();
    assert_eq!(
        style.content_alpha, 222,
        "v1's `with_alpha(on_surface, 222)` on the content"
    );
    assert_ne!(
        style.content_alpha, style.on_surface.a,
        "so it is not the same colour, and 222 is what makes it so"
    );
    assert_eq!(style.on_surface.a, 255, "the title is at full strength");
}

#[test]
fn the_type_sizes_come_from_the_roles_v1_named() {
    let style = dialog().resolved_style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Fluent,
    );
    let typography = theme.typography();
    assert_eq!(style.title_size, typography.headline.small.font_size);
    assert_eq!(style.body_size, typography.body.medium.font_size);
    assert_eq!(style.action_size, typography.label.large.font_size);
}

#[test]
fn the_shadow_is_black_at_v1s_alpha() {
    let style = dialog().resolved_style();
    assert_eq!(style.shadow_alpha, 30);
}

#[test]
fn all_nine_theme_weights_map_by_name() {
    use floom_core_theme::tokens::typography::FontWeight as Theme;
    use floom_types::FontWeight;

    for (theme, engine) in [
        (Theme::Thin, FontWeight::Thin),
        (Theme::ExtraLight, FontWeight::ExtraLight),
        (Theme::Light, FontWeight::Light),
        (Theme::Regular, FontWeight::Normal),
        (Theme::Medium, FontWeight::Medium),
        (Theme::SemiBold, FontWeight::SemiBold),
        (Theme::Bold, FontWeight::Bold),
        (Theme::ExtraBold, FontWeight::ExtraBold),
        (Theme::Black, FontWeight::Black),
    ] {
        assert_eq!(
            to_engine_weight(theme),
            engine,
            "{theme:?} is the engine's {engine:?}"
        );
    }
}

/// Unverifiable from here — no call site in any kit reads `Black` — and it is a bug fix
#[test]
fn black_maps_to_900_where_v1_mapped_it_to_800() {
    use floom_core_theme::tokens::typography::FontWeight as Theme;
    use floom_types::FontWeight;

    assert_eq!(to_engine_weight(Theme::Black), FontWeight::Black);
    assert_eq!(
        floom_types::FontWeight::Black.to_value(),
        900,
        "and 900 is the point — parley had no 900, which is how v1 reached 800"
    );
    assert_eq!(
        floom_types::FontWeight::ExtraBold.to_value(),
        800,
        "so `Black` and `ExtraBold` are distinguishable here and were not in v1"
    );
}

#[test]
fn the_action_weight_comes_from_the_ramp_not_from_a_literal() {
    let style = dialog().resolved_style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Fluent,
    );
    assert_eq!(
        style.action_weight,
        to_engine_weight(theme.typography().label.large.font_weight)
    );
}

#[test]
fn props_report_the_v1_shaped_callbacks() {
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = fired.clone();
    let dialog = dialog()
        .on_close(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .action("Cancel", || {})
        .action("Delete", || {});

    let props = dialog.props();
    assert_eq!(props.title, "Delete file");
    assert_eq!(props.content, "This cannot be undone.");
    assert!(
        props.on_close.is_some(),
        "and the close callback is stored v1-shaped"
    );
    assert_eq!(
        props.actions.len(),
        2,
        "with one entry per action, as v1's `Vec<(String, Box<dyn Fn()>)>` held"
    );
}

#[test]
fn actions_accumulate() {
    let dialog = dialog()
        .action("One", || {})
        .action("Two", || {})
        .action("Three", || {});
    assert_eq!(dialog.props().actions.len(), 3);
}

#[test]
fn the_theme_is_not_selected_by_naming_it() {
    let fluent = dialog().resolved_style();
    let cupertino = dialog()
        .with_theme(floom_core_theme::theme::MultiTheme::new_for(
            floom_core_theme::global_theme::DesignSystem::Cupertino,
        ))
        .resolved_style();
    assert_eq!(fluent.max_width, cupertino.max_width);
}

#[test]
fn the_card_radius_comes_from_the_theme_ramp_not_from_the_token() {
    let style = FluentDialog::new().resolved_style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Fluent,
    );
    let floom_core_theme::component_tokens::ComponentTokens::Dialog(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Dialog)
    else {
        panic!("expected DialogTokens");
    };
    assert_eq!(
        style.border_radius,
        theme.radius().sm,
        "v1's `theme.radius().sm`"
    );
    assert_ne!(
        style.border_radius, tokens.border_radius,
        "and it is not the token, which is what makes this a disagreement rather than a \
         reading"
    );
}
