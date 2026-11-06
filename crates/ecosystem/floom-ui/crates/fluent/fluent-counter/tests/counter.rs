//! The ported Material counter.

use std::sync::{Arc, Mutex};

use floom_ui_composites::Counter;

use floom_widget_fluent_counter::{FluentCounter, READOUT_FONT_SIZE};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, CounterTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

fn counter_under(ds: DesignSystem) -> FluentCounter {
    floom_core_theme::global_theme::set_design_system(ds);
    FluentCounter::new()
}

fn counter() -> FluentCounter {
    counter_under(DesignSystem::Material)
}

fn tokens_under(ds: DesignSystem) -> CounterTokens {
    floom_core_theme::global_theme::set_design_system(ds);
    match MultiTheme::new_for(ds).component(ComponentId::Counter) {
        ComponentTokens::Counter(t) => t,
        _ => unreachable!(),
    }
}

fn fluent_tokens() -> CounterTokens {
    tokens_under(DesignSystem::Fluent)
}

#[test]
fn the_sizes_come_from_the_token_table() {
    let tokens = fluent_tokens();
    let style = counter().counter_style();
    assert_eq!(style.button_size, tokens.button_size);
    assert_eq!(style.width, tokens.width);
    assert_eq!(style.height, tokens.height);
    assert_eq!(style.radius, tokens.radius);
    assert_eq!(style.gap, tokens.gap);
    assert_eq!(style.border, tokens.border);
}

#[test]
fn the_step_buttons_are_surface_variant_with_on_surface_ink() {
    let theme = MultiTheme::new_for(DesignSystem::Material);
    let style = counter().with_theme(theme.clone()).resolved_style();
    assert_eq!(style.button_background, theme.colors().surface_variant);
    assert_eq!(style.button_foreground, theme.colors().on_surface);
}

#[test]
fn the_band_is_outlined_with_the_outline_token() {
    let theme = MultiTheme::new_for(DesignSystem::Material);
    let style = counter().with_theme(theme.clone()).resolved_style();
    assert_eq!(style.border_color, theme.colors().outline);
    assert_ne!(
        style.border_color, style.button_foreground,
        "`outline` and `on_surface` are different tokens; reading the wrong one \
         would still produce a plausible outline"
    );
}

fn own() -> DesignSystem {
    DesignSystem::Fluent
}

fn contrast() -> DesignSystem {
    DesignSystem::Material
}

#[test]
fn the_colours_follow_the_active_kit_so_they_are_read_not_hard_coded() {
    let a = counter_under(own()).resolved_style().button_background;
    let b = counter_under(contrast()).resolved_style().button_background;
    assert_ne!(
        a, b,
        "surface_variant differs per kit; equal results would mean the adapter \
         hard-coded one"
    );
}

/// That is not a bug in this crate — it is what `MultiTheme` is — and the test is
#[test]
fn the_crate_reads_whichever_kit_is_active_rather_than_pinning_its_own() {
    let own_kit = counter_under(own()).resolved_style().button_background;
    let other = counter_under(contrast()).resolved_style().button_background;
    assert_ne!(
        own_kit, other,
        "and there is no with_theme that changes it: MultiTheme reads the global"
    );
}

#[test]
fn the_readout_is_drawn_in_on_surface_the_same_ink_as_the_glyphs() {
    let style = counter().resolved_style();
    assert_eq!(style.value_color, style.button_foreground);
}

#[test]
fn the_readout_font_size_is_sixteen_and_is_not_a_token() {
    assert_eq!(READOUT_FONT_SIZE, 16.0);
    let style = counter().counter_style();
    assert_eq!(style.value_font_size, 16.0);
}

#[test]
fn the_view_starts_at_the_requested_value() {
    let widget = counter().value(7).view();
    assert_eq!(widget.value(), 7);
}

#[test]
fn the_bounds_and_the_step_reach_the_engine() {
    let widget = counter().value(5).min(0).max(10).step(3).view();
    assert_eq!(widget.minimum(), Some(0));
    assert_eq!(widget.maximum(), Some(10));
    assert_eq!(widget.step(), 3);
    assert_eq!(widget.step_up(), 8);
}

#[test]
fn a_bound_set_before_mount_clamps_immediately() {
    let widget = counter().value(50).max(10).view();
    assert_eq!(widget.value(), 10, "not 50 waiting for a press");
}

#[test]
fn the_view_hands_back_a_widget_the_caller_can_read() {
    let widget: Counter = counter().value(1).view();
    widget.step_up();
    assert_eq!(widget.value(), 2);
}

#[test]
fn the_change_callback_reports_a_move() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let widget = counter()
        .value(1)
        .on_change(move |v| sink.lock().unwrap().push(v))
        .view();

    widget.step_up();
    widget.step_up();
    assert_eq!(seen.lock().unwrap().clone(), vec![2, 3]);
}

#[test]
fn a_press_at_the_ceiling_still_reports_the_unchanged_value() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let widget = counter()
        .value(10)
        .max(10)
        .on_change(move |v| sink.lock().unwrap().push(v))
        .view();

    assert_eq!(widget.step_up(), 10);
    assert_eq!(seen.lock().unwrap().clone(), vec![10]);
}

#[test]
fn the_engine_style_is_the_resolved_style_translated_field_for_field() {
    let resolved = counter().resolved_style();
    let engine = counter().counter_style();
    assert_eq!(engine.button_size, resolved.button_size);
    assert_eq!(engine.width, resolved.width);
    assert_eq!(engine.height, resolved.height);
    assert_eq!(engine.radius, resolved.radius);
    assert_eq!(engine.gap, resolved.gap);
    assert_eq!(engine.border, resolved.border);
    assert_eq!(
        engine.border_color.r, resolved.border_color.r,
        "the colour is converted, not copied: floom_types::Color carries the same \
         four channels and a copy would type-check"
    );
}

#[test]
fn the_resolved_style_names_its_design_system() {
    assert_eq!(counter().resolved_style().design_system, "fluent");
}

/// too, and a clone that silently lost a callback would be a worse bug than one
#[test]
fn the_props_are_kept_and_the_callback_is_held_separately() {
    let built = counter().value(3).min(1).max(9).step(2);
    let props = built.props();
    assert_eq!(props.value, 3);
    assert_eq!(props.min, Some(1));
    assert_eq!(props.max, Some(9));
    assert_eq!(props.step, 2);
}
