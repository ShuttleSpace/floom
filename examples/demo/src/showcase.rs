//! Vant-style component showcase helpers for the floem-ui demo.
//!
//! Each component page is composed of one or more `Demo` entries rendered as
//! cards. A `Demo` is either a static variant (`demo` / `demo_note`) or an
//! interactive one (`demo_ctl`) that pairs the component with a live controls
//! panel. The controls write to `RwSignal`s that the component reads, so
//! toggling a control updates the rendered component immediately — the same
//! pattern Vant uses in its right-hand preview pane.

use floem::IntoView;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::view::AnyView;
use floem::views::{Decorators, Label, Scroll, Stack, dyn_view};
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color_ext::with_alpha;
use floem_core_theme::tokens::typography::FontWeight;
use floem_elements::button::{ButtonSize, ButtonVariant};
use floem_elements::prelude::*;
use floem_widget_runtime::theme_signal;

/// A single showcase entry.
pub struct Demo {
    label: String,
    note: Option<String>,
    /// Builds the entry's renderable content (component + optional controls).
    /// `FnOnce` because the factory consumes its captured component on the
    /// single call `showcase` makes.
    view: Box<dyn FnOnce() -> AnyView>,
}

/// A static variant with a label.
pub fn demo<IV: IntoView + 'static>(label: impl Into<String>, view: IV) -> Demo {
    Demo {
        label: label.into(),
        note: None,
        view: Box::new(move || view.into_any()),
    }
}

/// A static variant with a label and a short descriptive note.
pub fn demo_note<IV: IntoView + 'static>(
    label: impl Into<String>,
    view: IV,
    note: impl Into<String>,
) -> Demo {
    Demo {
        label: label.into(),
        note: Some(note.into()),
        view: Box::new(move || view.into_any()),
    }
}

/// An interactive variant: the component (`view`) plus a `controls` panel that
/// writes to the same signals the component reads, so changes are live.
pub fn demo_ctl<IV: IntoView + 'static, CV: IntoView + 'static>(
    label: impl Into<String>,
    view: IV,
    controls: CV,
) -> Demo {
    Demo {
        label: label.into(),
        note: None,
        view: Box::new(move || {
            Stack::vertical((view.into_any(), controls.into_any()))
                .style(|s| s.gap(10.0).items_start())
                .into_any()
        }),
    }
}

/// Compose a full component page: heading + intro + a vertical stack of demo cards.
pub fn showcase(name: &str, intro: &str, demos: Vec<Demo>) -> AnyView {
    let cards: Vec<AnyView> = demos
        .into_iter()
        .map(|d| {
            let label = d.label.clone();
            let note = d.note.clone();
            let content = (d.view)();
            let mut col: Vec<AnyView> = Vec::new();
            col.push(
                Label::new(label)
                    .style(|s| {
                        let t = theme_signal().get();
                        s.font_size(13.0)
                            .font_weight(FontWeight::Medium.to_floem())
                            .color(t.colors().on_surface_variant.to_peniko())
                    })
                    .into_any(),
            );
            col.push(content);
            if let Some(note) = note {
                col.push(
                    Label::new(note)
                        .style(|s| {
                            let t = theme_signal().get();
                            s.font_size(12.0)
                                .color(with_alpha(t.colors().on_surface, 150).to_peniko())
                        })
                        .into_any(),
                );
            }
            Stack::vertical_from_iter(col)
                .style(|s| {
                    let t = theme_signal().get();
                    s.gap(8.0)
                        .items_start()
                        .padding(16.0)
                        .border_radius(12.0)
                        .background(t.colors().surface.to_peniko())
                        .border(1.0)
                        .border_color(with_alpha(t.colors().on_surface, 24).to_peniko())
                })
                .into_any()
        })
        .collect();

    Stack::vertical((
        Label::new(name.to_string())
            .style(|s| {
                let t = theme_signal().get();
                s.font_size(18.0)
                    .font_weight(FontWeight::Medium.to_floem())
                    .color(t.colors().on_surface.to_peniko())
            })
            .into_any(),
        Label::new(intro.to_string())
            .style(|s| {
                let t = theme_signal().get();
                s.font_size(13.0)
                    .color(with_alpha(t.colors().on_surface, 170).to_peniko())
            })
            .into_any(),
        Stack::vertical_from_iter(cards).style(|s| s.gap(16.0).items_start()),
    ))
    .style(|s| s.gap(14.0).items_start())
    .into_any()
}

/// A labelled toggle row bound to a `bool` signal. Used inside `demo_ctl` panels.
pub fn toggle_row(label: &str, signal: RwSignal<bool>) -> AnyView {
    let lbl = label.to_string();
    Stack::horizontal((
        Label::new(lbl)
            .style(|s| {
                let t = theme_signal().get();
                s.font_size(13.0).color(t.colors().on_surface.to_peniko())
            })
            .into_any(),
        Switch::new()
            .checked(signal.get())
            .on_change(move |v| signal.set(v))
            .view()
            .into_any(),
    ))
    .style(|s| s.gap(8.0).items_center())
    .into_any()
}

/// A reactive segmented picker bound to a signal. `options` maps display labels
/// to values; clicking an option sets the signal and the active highlight
/// follows live.
pub fn segmented<T>(label: &str, options: Vec<(&'static str, T)>, signal: RwSignal<T>) -> AnyView
where
    T: PartialEq + Copy + 'static,
{
    let label = label.to_string();
    let opts = options;
    let sig = signal;
    dyn_view(move || {
        let cur = sig.get();
        let chips: Vec<AnyView> = opts
            .iter()
            .map(|(text, val)| {
                let val = *val;
                let active = cur == val;
                Button::new(text.to_string())
                    .variant(ButtonVariant::Text)
                    .on_click(move || sig.set(val))
                    .view()
                    .style(move |s| {
                        let t = theme_signal().get();
                        s.padding(6.0)
                            .border_radius(8.0)
                            .background(if active {
                                t.colors().secondary_container.to_peniko()
                            } else {
                                with_alpha(t.colors().on_surface, 0).to_peniko()
                            })
                            .color(if active {
                                t.colors().on_secondary_container.to_peniko()
                            } else {
                                t.colors().on_surface.to_peniko()
                            })
                    })
                    .into_any()
            })
            .collect();
        Stack::vertical((
            Label::new(label.clone())
                .style(|s| {
                    let t = theme_signal().get();
                    s.font_size(13.0).color(t.colors().on_surface.to_peniko())
                })
                .into_any(),
            Stack::horizontal_from_iter(chips).style(|s| s.gap(4.0)),
        ))
        .style(|s| s.gap(6.0).items_start())
        .into_any()
    })
    .into_any()
}

// ---------------------------------------------------------------------------
// Extended showcase helpers (Phase A): method demos, API tables, code snippets
// ---------------------------------------------------------------------------

/// A prominent status pill bound to a `bool` signal — used inside `demo_method`
/// result areas to give clear, live visual feedback when a component method fires.
pub fn status_badge(
    signal: RwSignal<bool>,
    on_text: &'static str,
    off_text: &'static str,
) -> AnyView {
    let on = on_text.to_string();
    let off = off_text.to_string();
    dyn_view(move || {
        let active = signal.get();
        let t = theme_signal().get();
        let (bg, fg, txt) = if active {
            (
                t.colors().primary_container.to_peniko(),
                t.colors().on_primary_container.to_peniko(),
                on.clone(),
            )
        } else {
            (
                with_alpha(t.colors().on_surface, 24).to_peniko(),
                t.colors().on_surface_variant.to_peniko(),
                off.clone(),
            )
        };
        Label::new(txt)
            .style(move |s| {
                s.padding(8.0)
                    .border_radius(99.0)
                    .background(bg)
                    .color(fg)
                    .font_size(13.0)
                    .font_weight(FontWeight::Medium.to_floem())
            })
            .into_any()
    })
    .into_any()
}

/// A method-demo card: a trigger button runs `on_trigger` (which calls a
/// component's exposed method — typically via an `open_signal()` / `dark_signal()`),
/// and `result` shows the prominent, reactive feedback (e.g. a `status_badge`).
pub fn demo_method<F, RV>(
    label: impl Into<String>,
    trigger_label: impl Into<String>,
    on_trigger: F,
    result: RV,
) -> Demo
where
    F: Fn() + 'static,
    RV: IntoView + 'static,
{
    let trigger = trigger_label.into();
    Demo {
        label: label.into(),
        note: None,
        view: Box::new(move || {
            Stack::vertical((
                Button::new(trigger)
                    .variant(ButtonVariant::Secondary)
                    .on_click(move || on_trigger())
                    .view()
                    .into_any(),
                result.into_any(),
            ))
            .style(|s| s.gap(10.0).items_start())
            .into_any()
        }),
    }
}

/// One row in an API reference table.
pub struct ApiRow {
    pub name: &'static str,
    pub ty: &'static str,
    pub default: &'static str,
    pub desc: &'static str,
}

fn api_cell(text: String, width: f32, head: bool, grow: bool) -> AnyView {
    Label::new(text)
        .style(move |s| {
            let t = theme_signal().get();
            let mut s = s
                .padding(8.0)
                .width(width)
                .font_size(12.0)
                .color(if head {
                    t.colors().on_surface.to_peniko()
                } else {
                    t.colors().on_surface_variant.to_peniko()
                })
                .font_weight(if head {
                    FontWeight::Medium.to_floem()
                } else {
                    FontWeight::Regular.to_floem()
                });
            if grow {
                s = s.flex_grow(1.0);
            }
            s
        })
        .into_any()
}

/// Render a Props / Events / Methods / Slots reference table.
/// Columns: 名称 | 类型 | 默认值 | 说明.
pub fn api_table(title: &str, rows: Vec<ApiRow>) -> Demo {
    let title = title.to_string();
    let mut children: Vec<AnyView> = Vec::new();
    children.push(
        Label::new(title.clone())
            .style(|s| {
                let t = theme_signal().get();
                s.font_size(13.0)
                    .font_weight(FontWeight::Medium.to_floem())
                    .color(t.colors().on_surface.to_peniko())
                    .padding(4.0)
            })
            .into_any(),
    );

    let header = Stack::horizontal_from_iter(vec![
        api_cell("名称".into(), 150.0, true, false),
        api_cell("类型".into(), 150.0, true, false),
        api_cell("默认值".into(), 100.0, true, false),
        api_cell("说明".into(), 0.0, true, true),
    ])
    .style(|s| {
        let t = theme_signal().get();
        s.width_full()
            .border_bottom(1.0)
            .border_color(with_alpha(t.colors().on_surface, 24).to_peniko())
            .background(with_alpha(t.colors().on_surface, 8).to_peniko())
    })
    .into_any();
    children.push(header);

    for r in rows {
        let row = Stack::horizontal_from_iter(vec![
            api_cell(r.name.to_string(), 150.0, false, false),
            api_cell(r.ty.to_string(), 150.0, false, false),
            api_cell(r.default.to_string(), 100.0, false, false),
            api_cell(r.desc.to_string(), 0.0, false, true),
        ])
        .style(|s| {
            let t = theme_signal().get();
            s.width_full()
                .border_bottom(1.0)
                .border_color(with_alpha(t.colors().on_surface, 12).to_peniko())
                .items_center()
        })
        .into_any();
        children.push(row);
    }

    let table = Stack::vertical_from_iter(children)
        .style(|s| {
            let t = theme_signal().get();
            s.gap(0.0)
                .width_full()
                .items_start()
                .border(1.0)
                .border_radius(12.0)
                .border_color(with_alpha(t.colors().on_surface, 24).to_peniko())
        })
        .into_any();
    Demo {
        label: title,
        note: None,
        view: Box::new(move || table),
    }
}

/// A collapsible source-code card showing the real usage snippet for a demo.
pub fn code_snippet(label: impl Into<String>, src: &'static str) -> Demo {
    let label = label.into();
    let src = src.to_string();
    Demo {
        label,
        note: None,
        view: Box::new(move || {
            Scroll::new(
                Text::new(src.clone())
                    .view()
                    .style(|s| {
                        let t = theme_signal().get();
                        s.font_family("monospace")
                            .font_size(12.0)
                            .color(t.colors().on_surface.to_peniko())
                            .padding(12.0)
                    })
                    .into_any(),
            )
            .style(|s| {
                let t = theme_signal().get();
                s.height(240.0)
                    .width_full()
                    .border_radius(10.0)
                    .border(1.0)
                    .border_color(with_alpha(t.colors().on_surface, 16).to_peniko())
                    .background(with_alpha(t.colors().on_surface, 8).to_peniko())
            })
            .into_any()
        }),
    }
}

/// An integer slider bound to an `i32` signal — for props that are counts/indices.
pub fn slider_int_row(label: &str, min: i32, max: i32, signal: RwSignal<i32>) -> AnyView {
    let label = label.to_string();
    dyn_view(move || {
        let v = signal.get();
        Stack::vertical((
            Label::new(format!("{}: {}", label, v))
                .style(|s| {
                    let t = theme_signal().get();
                    s.font_size(13.0).color(t.colors().on_surface.to_peniko())
                })
                .into_any(),
            Slider::new()
                .value(v as f32)
                .min(min as f32)
                .max(max as f32)
                .step(1.0)
                .on_change(move |f| signal.set(f as i32))
                .view()
                .into_any(),
        ))
        .style(|s| s.gap(6.0).items_start().width(320.0))
        .into_any()
    })
    .into_any()
}

// ---------------------------------------------------------------------------
// Exemplar interactive pages
// ---------------------------------------------------------------------------

/// Button — interactive property controls (variant / size / disabled / loading)
/// plus a row of static variants.
pub fn button_showcase() -> AnyView {
    let disabled = RwSignal::new(false);
    let loading = RwSignal::new(false);
    let variant = RwSignal::new(ButtonVariant::Primary);
    let size = RwSignal::new(ButtonSize::Medium);

    let comp = {
        let disabled = disabled;
        let loading = loading;
        let variant = variant;
        let size = size;
        dyn_view(move || {
            Button::new("Button")
                .variant(variant.get())
                .size(size.get())
                .disabled(disabled.get())
                .loading(loading.get())
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        segmented(
            "Variant",
            vec![
                ("Primary", ButtonVariant::Primary),
                ("Secondary", ButtonVariant::Secondary),
                ("Danger", ButtonVariant::Danger),
                ("Text", ButtonVariant::Text),
                ("Ghost", ButtonVariant::Ghost),
            ],
            variant,
        ),
        segmented(
            "Size",
            vec![
                ("Small", ButtonSize::Small),
                ("Medium", ButtonSize::Medium),
                ("Large", ButtonSize::Large),
            ],
            size,
        ),
        toggle_row("Disabled", disabled),
        toggle_row("Loading", loading),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Button",
        "Buttons let people trigger actions. Pick a variant and size, or toggle \
         disabled / loading to see the states update live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Filled (Primary)", Button::new("Primary").variant(ButtonVariant::Primary).view()),
            demo("Tonal (Secondary)", Button::new("Secondary").variant(ButtonVariant::Secondary).view()),
            demo("Danger", Button::new("Danger").variant(ButtonVariant::Danger).view()),
            demo("Text", Button::new("Text").variant(ButtonVariant::Text).view()),
            demo("Ghost", Button::new("Ghost").variant(ButtonVariant::Ghost).view()),
            demo_note("Disabled", Button::new("Disabled").disabled(true).view(), "Non-interactive state."),
            demo_note("Loading", Button::new("Loading").loading(true).view(), "Shows a progress indicator and blocks input."),
            code_snippet("Button · 基础", BUTTON_SRC),
            api_table("Props", button_api()),
        ],
    )
}

/// Switch — interactive disabled control plus static on/off/disabled states.
pub fn switch_showcase() -> AnyView {
    let disabled = RwSignal::new(false);

    let comp = {
        let disabled = disabled;
        dyn_view(move || Switch::new().checked(true).disabled(disabled.get()).view().into_any())
    };

    let controls = Stack::vertical((toggle_row("Disabled", disabled),))
        .style(|s| s.gap(12.0).items_start());

    showcase(
        "Switch",
        "Switches toggle a single setting on or off.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo_note("On", Switch::new().checked(true).view(), "Selected state."),
            demo_note("Off", Switch::new().checked(false).view(), "Unselected state."),
            demo_note("Disabled on", Switch::new().checked(true).disabled(true).view(), "Selected + non-interactive."),
            demo_note("Disabled off", Switch::new().checked(false).disabled(true).view(), "Unselected + non-interactive."),
            code_snippet("Switch · 基础", SWITCH_SRC),
            api_table("Props", switch_api()),
        ],
    )
}

/// Slider — interactive value + disabled, plus static states.
pub fn slider_showcase() -> AnyView {
    let value = RwSignal::new(40.0f32);
    let disabled = RwSignal::new(false);
    let step = RwSignal::new(5i32);

    let comp = {
        let value = value;
        let disabled = disabled;
        dyn_view(move || {
            Stack::vertical((
                Slider::new()
                    .value_signal(value)
                    .min(0.0)
                    .max(100.0)
                    .step(1.0)
                    .disabled(disabled.get())
                    .view()
                    .into_any(),
                Label::derived(move || format!("Value: {:.0}", value.get()))
                    .style(|s| s.font_size(13.0))
                    .into_any(),
            ))
            .style(|s| s.gap(8.0).width(360.0))
            .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        slider_int_row("Step (1–10)", 1, 10, step),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Slider",
        "Sliders let users pick a value from a continuous range.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Slider::new().value(50.0).min(0.0).max(100.0).view()),
            demo("Disabled", Slider::new().value(50.0).min(0.0).max(100.0).disabled(true).view()),
            code_snippet("Slider · 基础", SLIDER_SRC),
            api_table("Props", slider_api()),
        ],
    )
}

/// Tabs — interactive selection, plus a basic variant.
pub fn tabs_showcase() -> AnyView {
    let selected = RwSignal::new(0usize);

    let comp = {
        let selected = selected;
        dyn_view(move || {
                Tabs::new()
                .tabs(vec!["One".to_string(), "Two".to_string(), "Three".to_string()])
                .selected(selected.get())
                .on_change(move |i| selected.set(i))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        Label::derived(move || format!("Selected tab: {}", selected.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Tabs",
        "Tabs organize content into separate, quickly switchable views.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Basic", Tabs::new().tabs(vec!["A".to_string(), "B".to_string()]).selected(0).view()),
            code_snippet("Tabs · 基础", TABS_SRC),
            api_table("Props", tabs_api()),
        ],
    )
}

/// Chip — interactive selected/disabled toggles, plus static variants.
pub fn chip_showcase() -> AnyView {
    let sel1 = RwSignal::new(true);
    let dis = RwSignal::new(false);

    let comp = {
        let sel1 = sel1;
        let dis = dis;
        dyn_view(move || {
            Stack::horizontal((
                Chip::new()
                    .label("Apple".to_string())
                    .selected(sel1.get())
                    .on_click(move || sel1.set(!sel1.get()))
                    .view()
                    .into_any(),
                Chip::new().label("Banana".to_string()).on_click(|| {}).view().into_any(),
                Chip::new()
                    .label("Cherry".to_string())
                    .disabled(dis.get())
                    .on_click(|| {})
                    .view()
                    .into_any(),
            ))
            .style(|s| s.gap(8.0).items_center())
            .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Apple selected", sel1),
        toggle_row("Cherry disabled", dis),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Chip",
        "Chips are compact, actionable elements for filters, choices, and tags.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Selected", Chip::new().label("Selected".to_string()).selected(true).on_click(|| {}).view()),
            demo("Default", Chip::new().label("Default".to_string()).on_click(|| {}).view()),
            demo("Disabled", Chip::new().label("Disabled".to_string()).disabled(true).on_click(|| {}).view()),
            code_snippet("Chip · 基础", CHIP_SRC),
            api_table("Props", chip_api()),
        ],
    )
}

/// Card — static elevation variants (Material surfaces).
pub fn card_showcase() -> AnyView {
    let content = || {
        Stack::vertical((
            Text::new("Card title").view().into_any(),
            Text::new("Cards group related content on a Material surface.")
                .view()
                .into_any(),
        ))
        .style(|s| s.gap(8.0))
    };
    showcase(
        "Card",
        "Cards group related content on a surface. Elevation controls the rest state.",
        vec![
            demo("Elevation 0", Card::new().elevation(0).child(content()).view()),
            demo("Elevation 1", Card::new().elevation(1).child(content()).view()),
            demo("Elevation 3", Card::new().elevation(3).child(content()).view()),
            demo("Elevation 5", Card::new().elevation(5).child(content()).view()),
            code_snippet("Card · 基础", CARD_SRC),
            api_table("Props", card_api()),
        ],
    )
}

// ---------------------------------------------------------------------------
// API reference metadata + source snippets for the pre-built interactive demos
// (Button / Switch / Slider / Tabs / Chip / Card) so every pilot category is
// fully templated (props + events + examples + notes + API table + code).
// ---------------------------------------------------------------------------

const BUTTON_SRC: &str = r#"Button::new("Button")
    .variant(ButtonVariant::Primary)
    .size(ButtonSize::Medium)
    .on_click(|| {})
    .view();"#;

fn button_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "variant", ty: "ButtonVariant", default: "Primary", desc: "Primary / Secondary / Danger / Text / Ghost." },
        ApiRow { name: "size", ty: "ButtonSize", default: "Medium", desc: "Small / Medium / Large." },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "Block interaction." },
        ApiRow { name: "loading", ty: "bool", default: "false", desc: "Show spinner, block input." },
        ApiRow { name: "on_click", ty: "Fn()", default: "—", desc: "Fires on press (event)." },
    ]
}

const SWITCH_SRC: &str = r#"Switch::new()
    .checked(true)
    .disabled(false)
    .view();"#;

fn switch_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "checked", ty: "bool", default: "false", desc: "On / off state." },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "Block interaction." },
    ]
}

const SLIDER_SRC: &str = r#"let value = RwSignal::new(40.0f32);
Slider::new()
    .value_signal(value)
    .min(0.0).max(100.0).step(1.0)
    .view();"#;

fn slider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "value / value_signal", ty: "f32 / RwSignal<f32>", default: "0.0", desc: "Current value (method)." },
        ApiRow { name: "min / max", ty: "f32", default: "0.0 / 100.0", desc: "Range bounds." },
        ApiRow { name: "step", ty: "f32", default: "1.0", desc: "Increment." },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "Block interaction." },
        ApiRow { name: "on_change", ty: "Fn(f32)", default: "—", desc: "Fires on drag (event)." },
    ]
}

const TABS_SRC: &str = r#"let selected = RwSignal::new(0usize);
Tabs::new()
    .tabs(vec!["One".into(), "Two".into(), "Three".into()])
    .selected(selected.get())
    .on_change(move |i| selected.set(i))
    .view();"#;

fn tabs_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "tabs", ty: "Vec<String>", default: "[]", desc: "Tab labels." },
        ApiRow { name: "selected", ty: "usize", default: "0", desc: "Active index." },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "Fires on switch (event)." },
    ]
}

const CHIP_SRC: &str = r#"Chip::new()
    .label("Apple".into())
    .selected(true)
    .on_click(|| {})
    .view();"#;

fn chip_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "label", ty: "String", default: "\"\"", desc: "Chip text." },
        ApiRow { name: "selected", ty: "bool", default: "false", desc: "Selected state." },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "Block interaction." },
        ApiRow { name: "on_click", ty: "Fn()", default: "—", desc: "Fires on tap (event)." },
    ]
}

const CARD_SRC: &str = r#"Card::new()
    .elevation(1)
    .child(/* content */)
    .view();"#;

fn card_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "elevation", ty: "u8", default: "0", desc: "Resting shadow height." },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "Card content." },
    ]
}
