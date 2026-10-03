use floom::{
    IntoView,
    peniko::Color as PenikoColor,
    reactive::{RwSignal, SignalGet, SignalUpdate},
    views::{Decorators, button, label, text_input, v_stack, h_stack},
};

use floom_core_theme::props::button::{ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MaterialTheme, Theme};
use floom_elements::prelude::*;

fn to_peniko(c: floom_core_theme::tokens::color::Color) -> PenikoColor {
    PenikoColor::from_rgba8(c.r, c.g, c.b, c.a)
}

fn main() {
    floom::launch(app_view);
}

fn app_view() -> impl IntoView {
    let theme = MaterialTheme::new();
    let mut counter = RwSignal::new(0);

    // ── floom-elements: construct all 10 widget models ─────────────────
    let md_primary = Button::new("Primary (MD)")
        .variant(ButtonVariant::Primary)
        .size(ButtonSize::Large)
        .with_theme(theme.clone());
    let md_secondary = Button::new("Secondary (MD)")
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::Medium)
        .with_theme(theme.clone());
    let md_danger = Button::new("Danger (MD)")
        .variant(ButtonVariant::Danger)
        .size(ButtonSize::Medium)
        .with_theme(theme.clone());

    let md_heading = Text::new("floom-elements Material Demo")
        .with_theme(theme.clone());
    let md_body = Text::new("All 10 widget types rendered below")
        .with_theme(theme.clone());

    let md_field = TextField::new()
        .placeholder("Enter text...")
        .with_theme(theme.clone());

    let md_checkbox = Checkbox::new("I agree to terms")
        .checked(true)
        .with_theme(theme.clone());

    let md_switch = Switch::new()
        .checked(true)
        .with_theme(theme.clone());

    let md_card = Card::new()
        .elevation(3)
        .with_theme(theme.clone());

    let md_icon = Icon::new("info")
        .size(20.0)
        .label("Information")
        .with_theme(theme.clone());

    let md_divider = Divider::new()
        .inset(16.0)
        .with_theme(theme.clone());

    // ── Resolve styles ────────────────────────────────────────────────
    let primary_style = md_primary.resolved_style();
    let secondary_style = md_secondary.resolved_style();
    let danger_style = md_danger.resolved_style();
    let heading_style = md_heading.resolved_style();
    let body_style = md_body.resolved_style();
    let field_style = md_field.resolved_style();
    let checkbox_style = md_checkbox.resolved_style();
    let switch_style = md_switch.resolved_style();
    let card_style = md_card.resolved_style();
    let icon_style = md_icon.resolved_style();
    let divider_style = md_divider.resolved_style();

    // Extract theme colors for background / borders
    let bg_color = to_peniko(theme.colors().background);
    let surface_color = to_peniko(theme.colors().surface);
    let outline_color = to_peniko(theme.colors().outline);
    let outline_variant = to_peniko(theme.colors().outline_variant);

    // ── Render: all 10 widget types ───────────────────────────────────
    v_stack((
        // ── Text ──────────────────────────────────────────────────
        label(move || "floom-elements Material Demo")
            .style(move |s| {
                s.font_size(heading_style.font_size + 6.0)
                 .color(to_peniko(heading_style.color))
                 .padding(8.0)
            }),
        label(move || "All 10 widget types rendered below")
            .style(move |s| {
                s.font_size(body_style.font_size)
                 .color(to_peniko(body_style.color))
                 .padding(4.0)
            }),

        // ── Divider ───────────────────────────────────────────────
        label(move || "")
            .style(move |s| {
                s.height(divider_style.thickness)
                 .width_full()
                 .background(to_peniko(divider_style.color))
                 .margin_horiz(divider_style.inset)
            }),

        // ── Buttons ───────────────────────────────────────────────
        h_stack((
            button("Primary Action")
                .action(move || counter.update(|v| *v += 1))
                .style(move |s| {
                    s.background(to_peniko(primary_style.background))
                     .color(to_peniko(primary_style.foreground))
                     .height(primary_style.height)
                     .padding_horiz(primary_style.padding_horizontal)
                     .border_radius(primary_style.border_radius)
                }),
            button("Secondary Action")
                .action(move || counter.update(|v| *v -= 1))
                .style(move |s| {
                    s.background(to_peniko(secondary_style.background))
                     .color(to_peniko(secondary_style.foreground))
                     .height(secondary_style.height)
                     .padding_horiz(secondary_style.padding_horizontal)
                     .border_radius(secondary_style.border_radius)
                }),
            button("Reset")
                .action(move || counter.set(0))
                .style(move |s| {
                    s.background(to_peniko(danger_style.background))
                     .color(to_peniko(danger_style.foreground))
                     .height(danger_style.height)
                     .padding_horiz(danger_style.padding_horizontal)
                     .border_radius(danger_style.border_radius)
                }),
        ))
        .style(|s| s.gap(8.0)),

        // ── Counter label ──────────────────────────────────────────
        label(move || format!("Counter: {}", counter.get()))
            .style(move |s| {
                s.font_size(body_style.font_size)
                 .color(to_peniko(body_style.color))
                 .padding(8.0)
                 .border(1.0)
                 .border_radius(4.0)
                 .border_color(outline_color)
            }),

        // ── TextField ─────────────────────────────────────────────
        text_input(RwSignal::new(String::new()))
            .style(move |s| {
                s.height(field_style.height)
                 .padding_horiz(field_style.padding_horizontal)
                 .border(1.0)
                 .border_color(to_peniko(field_style.border_color))
                 .border_radius(field_style.border_radius)
                 .color(to_peniko(field_style.text_color))
            }),

        // ── Checkbox ──────────────────────────────────────────────
        h_stack((
            label(move || "☑")
                .style(move |s| {
                    s.font_size(checkbox_style.size + 2.0)
                     .color(to_peniko(checkbox_style.container_color))
                     .padding(2.0)
                }),
            label(move || "I agree to terms (checked)")
                .style(move |s| {
                    s.font_size(body_style.font_size)
                     .color(to_peniko(body_style.color))
                }),
        ))
        .style(|s| s.items_center().gap(8.0)),

        // ── Switch ────────────────────────────────────────────────
        h_stack((
            label(move || "●")
                .style(move |s| {
                    s.font_size(switch_style.thumb_size)
                     .color(to_peniko(switch_style.thumb_color))
                     .padding(2.0)
                }),
            label(move || "Switch (on)")
                .style(move |s| {
                    s.font_size(body_style.font_size)
                     .color(to_peniko(body_style.color))
                }),
        ))
        .style(|s| s.items_center().gap(8.0)),

        // ── Card ──────────────────────────────────────────────────
        h_stack((
            label(move || "📇")
                .style(move |s| {
                    s.font_size(icon_style.size)
                     .color(to_peniko(icon_style.color))
                }),
            label(move || "Card (elevation 3)")
                .style(move |s| {
                    s.font_size(body_style.font_size)
                     .color(to_peniko(card_style.content_color))
                }),
        ))
        .style(move |s| {
            s.padding(card_style.padding)
             .border_radius(card_style.border_radius)
             .background(to_peniko(card_style.container_color))
             .items_center()
             .gap(8.0)
        }),

        // ── Icon ──────────────────────────────────────────────────
        h_stack((
            label(move || "ℹ")
                .style(move |s| {
                    s.font_size(icon_style.size)
                     .color(to_peniko(icon_style.color))
                }),
            label(move || "Icon (info, 20px)")
                .style(move |s| {
                    s.font_size(body_style.font_size)
                     .color(to_peniko(body_style.color))
                }),
        ))
        .style(|s| s.items_center().gap(8.0)),

        // ── Divider ───────────────────────────────────────────────
        label(move || "")
            .style(move |s| {
                s.height(divider_style.thickness)
                 .width_full()
                 .background(to_peniko(divider_style.color))
                 .margin_horiz(divider_style.inset)
            }),
    ))
    .style(move |s| {
        s.padding(20.0)
         .gap(12.0)
         .background(bg_color)
    })
}
