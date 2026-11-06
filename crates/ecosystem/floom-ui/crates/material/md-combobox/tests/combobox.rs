use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::Widget;
use floom_types::{OverlayKind, Rect};

use floom_ui_composites::ComboboxStyle;
use floom_ui_composites::PopupPlacement;
use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};

use floom_core_theme::theme::Theme;
use floom_widget_md_combobox::MdCombobox;

/// The field's box, so the panel's placement can be asserted against a real anchor.
const FIELD: Rect = Rect {
    x0: 30.0,
    y0: 100.0,
    x1: 230.0,
    y1: 156.0,
};

fn options() -> Vec<String> {
    ["apple", "banana", "Cherry"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn style() -> ComboboxStyle {
    MdCombobox::new().resolved_style()
}

#[test]
fn the_tokens_come_from_combobox_tokens() {
    let style = style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Material,
    );
    let floom_core_theme::component_tokens::ComponentTokens::Combobox(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Combobox)
    else {
        panic!("ComponentId::Combobox must resolve to ComboboxTokens");
    };
    assert_eq!(style.height, tokens.height);
    assert_eq!(style.padding_horizontal, tokens.padding_horizontal);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.min_width, tokens.min_width);
    assert_eq!(style.item_height, tokens.item_height);
}

#[test]
fn the_panel_bound_is_read_where_v1_never_read_it() {
    let style = style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Material,
    );
    let floom_core_theme::component_tokens::ComponentTokens::Combobox(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Combobox)
    else {
        panic!("expected ComboboxTokens");
    };
    assert_eq!(
        style.panel_max_height, tokens.panel_max_height,
        "the declared-but-unread token is now the panel's bound"
    );
    assert!(
        style.panel_max_height > 0.0,
        "and a bound of zero would clip the panel away entirely"
    );
}

#[test]
fn the_call_site_constants_are_material_s() {
    let style = style();
    assert_eq!(style.trigger_gap, 8.0, "v1's `.gap(8.0)`");
    assert_eq!(
        style.row_padding_horizontal, 12.0,
        "v1's `.padding_horiz(12.0)`"
    );
    assert_eq!(style.chevron_size, 14.0, "v1's `.size(14.0)`");
    assert_eq!(style.border_width, 1.0, "v1's `.border(1.0)`");
}

#[test]
fn the_panel_gap_is_materials_four_and_nobody_elses() {
    assert_eq!(
        style().panel_gap,
        4.0,
        "v1's inline `.margin_top(4.0)`, which Material was the only one of the three \
         to write — so the default here is Material's and not a fourth opinion"
    );
}

#[test]
fn the_alphas_are_v1s() {
    let style = style();
    assert_eq!(style.disabled_text_alpha, 38, "the disabled field text");
    assert_eq!(style.disabled_chevron_alpha, 97, "the disabled chevron");
    assert_eq!(style.disabled_border_alpha, 97, "the disabled border");
    assert_eq!(style.selected_row_alpha, 24, "the chosen row");
    assert_eq!(style.hover_row_alpha, 8, "a hovered row");
    assert_eq!(style.panel_border_alpha, 120, "the panel's border");
    assert_eq!(style.panel_shadow_alpha, 20, "the panel's shadow");
}

#[test]
fn the_border_follows_v1s_three_cases_in_v1s_order() {
    let style = style();
    assert_eq!(
        style.border_color(false, false),
        style.outline,
        "idle: plain outline"
    );
    assert_eq!(
        style.border_color(false, true),
        style.primary,
        "open: primary"
    );
    let disabled = style.border_color(true, true);
    assert_ne!(
        disabled, style.primary,
        "disabled wins over open — reversed, this is a combobox that looks live while \
         it is not"
    );
}

#[test]
fn the_theme_is_not_selected_by_naming_it_and_nothing_has_set_it() {
    let named_material = style();
    let named_cupertino = MdCombobox::new()
        .with_theme(floom_core_theme::theme::MultiTheme::new_for(
            floom_core_theme::global_theme::DesignSystem::Cupertino,
        ))
        .resolved_style();
    assert_eq!(
        named_material.height, named_cupertino.height,
        "two different named design systems resolve the same metrics, because \
         `new_for` ignores its argument"
    );
}

#[test]
fn props_report_what_the_builder_set() {
    let combo = MdCombobox::new()
        .options(options())
        .selected(Some(1))
        .placeholder("Pick fruit")
        .disabled(true)
        .on_change(|_| {});
    let props = combo.props();
    assert_eq!(props.options, options());
    assert_eq!(props.selected, Some(1));
    assert_eq!(props.placeholder, "Pick fruit");
    assert!(props.disabled);
    assert!(
        props.on_change.is_some(),
        "v1 stored a `Box<dyn Fn(..)>` here, and `props()` must keep seeing it"
    );
}

#[test]
fn the_default_placeholder_is_v1s() {
    assert_eq!(MdCombobox::new().props().placeholder, "Select\u{2026}");
}

#[test]
fn the_viewed_combobox_places_its_panel_below_the_field_with_material_s_gap() {
    let mut combo = MdCombobox::new().options(options()).view();
    let id = combo.create(&mut AllocatingCreateContext::new());
    let mut cx = RecordingUpdateContext::with_bounds(FIELD);

    combo.set_open(true);
    combo.update(&mut cx, id);

    let opened = cx.opened_overlays();
    assert_eq!(opened.len(), 1, "one panel asked for");
    match opened[0] {
        floom_widgets::testkit::OverlayCall::Open {
            bounds,
            kind,
            modal,
            ..
        } => {
            assert_eq!(*kind, OverlayKind::Popup);
            assert!(!modal, "a dropdown is dismissed, not blocking");
            assert_eq!(
                bounds.y0,
                FIELD.y1 + 4.0,
                "Material's 4px gap below the field"
            );
            assert_eq!(bounds.x0, FIELD.x0, "aligned with the field");
            assert_eq!(bounds.x1, FIELD.x1, "and exactly as wide as it");
        }
        _ => panic!("expected an open, got a close"),
    }
}

#[test]
fn the_options_reach_the_widget_and_an_empty_query_lists_them_all() {
    let combo = MdCombobox::new().options(options()).view();
    assert_eq!(
        combo.visible(),
        options(),
        "v1's `q.is_empty() || contains(q)`, so opening shows everything"
    );
}

/// And the callback the kit was given fires when an option is chosen, through the
/// `Send + Sync` handle rather than the v1 `Box<dyn Fn(..)>`.
#[test]
fn the_callback_fires_when_an_option_is_chosen() {
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = fired.clone();
    let mut combo = MdCombobox::new()
        .options(options())
        .on_change(move |index| {
            assert_eq!(index, Some(2));
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .view();

    combo.select(2);
    assert_eq!(fired.load(Ordering::SeqCst), 1);
    assert_eq!(combo.selected(), Some(2));
}

/// `disabled` reaches the widget, and a disabled combobox paints differently from a
/// live one — the border and the text both fade.
#[test]
fn disabled_reaches_the_widget_and_changes_how_it_paints() {
    let live = MdCombobox::new().options(options()).view();
    let dead = MdCombobox::new().options(options()).disabled(true).view();
    let live_trigger = live.children().remove(0).build();
    let dead_trigger = dead.children().remove(0).build();
    let live_scene = floom_widgets::testkit::paint_snapshot(
        &*live_trigger,
        floom_core::ElementId::new(1, 1),
        FIELD,
    );
    let dead_scene = floom_widgets::testkit::paint_snapshot(
        &*dead_trigger,
        floom_core::ElementId::new(1, 1),
        FIELD,
    );
    assert_ne!(live_scene, dead_scene);
}

/// The placement is what the engine builds from the style, so a kit's gap and bound
/// reach the request rather than only living in a struct nobody reads.
#[test]
fn the_style_drives_the_placement_the_request_carries() {
    let style = style();
    assert_eq!(
        style.placement(),
        PopupPlacement::Below {
            gap: 4.0,
            height: f64::from(style.panel_max_height),
        },
        "gap from the token-era call site, height from the token v1 never read"
    );
}
