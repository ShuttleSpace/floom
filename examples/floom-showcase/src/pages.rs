// SPDX-License-Identifier: MIT
//! The six pages of the showcase, each returning a `Stack`.
//!
//! Pages are declared by `page_for`, which the shell routes to. Each page owns
//! its own `ChildKey` slate through the builder closures below.

use std::sync::Arc;

use floom::floom_core::{Align, Axis, Child, ChildKey, FontWeight, LayoutRequest};
use floom::floom_widgets::{Divider, Stack, Switch};
use floom_signal::{SignalGet, SignalUpdate};

use crate::editor::SharedEditor;
use crate::routes::{AppRoute, NavIntent};
use crate::state::AppState;
use crate::widgets::{
    ACCENT, BORDER, FAINT, INK, MUTED, button_child, card, disabled_button, go_button, label,
    paragraph, signal_text, spacer,
};

/// Dispatches a route to its page builder.
pub fn page_for(
    route: AppRoute,
    state: Arc<AppState>,
    pending: Arc<std::sync::Mutex<Option<NavIntent>>>,
) -> Stack {
    match route {
        AppRoute::Home => page_home(state, pending),
        AppRoute::Components => page_components(state),
        AppRoute::Signals => page_signals(state),
        AppRoute::Kits => page_kits(state),
        AppRoute::Editor => page_editor(state),
        AppRoute::Devtools => page_devtools(state),
    }
}

/// Common page header: title + subtitle + gap.
fn base_page(title: &str, subtitle: &str) -> Stack {
    Stack::new(Axis::Vertical)
        .gap(9.0)
        .child(label("page-title", title, 28.0, FontWeight::SemiBold, INK))
        .child(paragraph(
            "page-subtitle",
            subtitle,
            14.0,
            FontWeight::Normal,
            MUTED,
        ))
        .child(spacer("page-gap", 10.0))
}

/// Home page: intro text + four home cards + devtools card.
fn page_home(state: Arc<AppState>, pending: Arc<std::sync::Mutex<Option<NavIntent>>>) -> Stack {
    let _ = &state;
    let mut page = base_page(
        "Floom 2.0 Showcase",
        "One window over the engine, the reactive core, the ported kits, and the editor.",
    );

    page = page.child(paragraph(
        "intro",
        "Every page renders with the same retained widget tree and the same router.",
        13.0,
        FontWeight::Normal,
        FAINT,
    ));
    page = page.child(spacer("cards-gap", 8.0));

    let mut row1 = Stack::new(Axis::Horizontal).gap(16.0);
    row1 = row1.child(home_card(
        "card-components",
        "Components",
        "Button, Switch, Progress, Spinner, Divider and Text, mounted straight from floom-widgets.",
        AppRoute::Components,
        pending.clone(),
    ));
    row1 = row1.child(home_card(
        "card-signals",
        "Signals",
        "SyncBaseSignal counters and toggles, read back by a widget that tracks the value.",
        AppRoute::Signals,
        pending.clone(),
    ));
    page = page.child(Child::new(
        ChildKey::from("row1"),
        LayoutRequest::new().width_pct(100.0).height(168.0),
        move || Box::new(row1.clone()),
    ));

    let mut row2 = Stack::new(Axis::Horizontal).gap(16.0);
    row2 = row2.child(home_card(
        "card-kits",
        "Kits",
        "The ported material adapters from the floom-ui satellite workspace.",
        AppRoute::Kits,
        pending.clone(),
    ));
    row2 = row2.child(home_card(
        "card-editor",
        "Editor",
        "floom-editor's TextDocument, transactions, undo and caret, in one widget.",
        AppRoute::Editor,
        pending.clone(),
    ));
    page = page.child(Child::new(
        ChildKey::from("row2"),
        LayoutRequest::new().width_pct(100.0).height(168.0),
        move || Box::new(row2.clone()),
    ));

    let devtools_pending = pending.clone();
    page = page.child(card(
        "card-devtools",
        132.0,
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .align(Align::SpaceBetween, Align::Start)
            .child(label("t", "Devtools", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "b",
                "Press F12 to open the live element tree; Ctrl+Shift+C arms the pick.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(go_button(
                "open",
                "Open",
                AppRoute::Devtools,
                devtools_pending,
            )),
    ));

    page
}

/// A two-up home card: title, blurb, open button.
fn home_card(
    key: &'static str,
    title: &'static str,
    blurb: &'static str,
    route: AppRoute,
    pending: Arc<std::sync::Mutex<Option<NavIntent>>>,
) -> floom::floom_core::Child {
    let body = Stack::new(Axis::Vertical)
        .gap(6.0)
        .align(Align::SpaceBetween, Align::Start)
        .child(label("t", title, 16.0, FontWeight::SemiBold, INK))
        .child(paragraph("b", blurb, 13.0, FontWeight::Normal, MUTED))
        .child(go_button("open", "Open", route, pending));
    let mut child = card(key, 168.0, body);
    // Half of a two-up row: a zero basis plus grow gives both cards the same
    // width. `fill()` alone keeps each card's content-sized basis, so the
    // longer blurb would make its card wider than its neighbour.
    child.layout = LayoutRequest::new()
        .width(0.0)
        .fill()
        .min_width(0.0)
        .height(168.0);
    child
}

/// Components page: engine Tier-0 widgets mounted directly.
fn page_components(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Components",
        "The engine's Tier-0 widgets, mounted directly with no kit layer in between.",
    );

    let clicks = state.clone();
    let click_count = state.clone();
    let clicks_2 = state.clone();
    let click_count_2 = state.clone();

    let buttons = Stack::new(Axis::Horizontal)
        .gap(10.0)
        .align(Align::Start, Align::Center)
        .child(button_child(
            "primary",
            "Primary",
            LayoutRequest::new().width(104.0).height(36.0),
            {
                let state = clicks.clone();
                move || {
                    state.clicks.update(|n| *n += 1);
                }
            },
        ))
        .child(button_child(
            "secondary",
            "Secondary",
            LayoutRequest::new().width(112.0).height(36.0),
            {
                let state = clicks_2.clone();
                move || {
                    state.clicks.update(|n| *n += 1);
                }
            },
        ))
        .child(disabled_button("disabled", "Disabled"));

    page = page.child(card(
        "buttons",
        176.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label("t", "Buttons", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "b",
                "The two live buttons share a counter; the readout is a signal-backed widget.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("buttons-row"),
                LayoutRequest::new().height(36.0),
                move || Box::new(buttons.clone()),
            ))
            .child(signal_text(
                "clicks-readout",
                {
                    let state = click_count.clone();
                    Arc::new(move || format!("clicks = {}", state.clicks.get()))
                },
                13.0,
                INK,
            )),
    ));

    let switch_state = state.clone();
    let bright_read = state.clone();
    let bright_read_2 = state.clone();
    let _ = click_count_2;

    let controls = Stack::new(Axis::Horizontal)
        .gap(22.0)
        .align(Align::Start, Align::Center)
        .child(Child::new(
            ChildKey::from("switch"),
            LayoutRequest::new().height(28.0),
            move || {
                let state = switch_state.clone();
                Box::new(
                    Switch::new("Wi-Fi")
                        .on(state.bright.get())
                        .on_toggle(move |on| state.bright.set(on)),
                )
            },
        ))
        .child(signal_text(
            "bright-readout",
            {
                let state = bright_read.clone();
                Arc::new(move || {
                    format!("Wi-Fi is {}", if state.bright.get() { "on" } else { "off" })
                })
            },
            13.0,
            MUTED,
        ))
        .child(Child::new(
            ChildKey::from("progress"),
            LayoutRequest::new().width(190.0).height(8.0),
            move || Box::new(floom::floom_widgets::Progress::determinate(0.42)),
        ))
        .child(Child::new(
            ChildKey::from("progress-busy"),
            LayoutRequest::new().width(190.0).height(8.0),
            move || Box::new(floom::floom_widgets::Progress::indeterminate()),
        ))
        .child(Child::new(
            ChildKey::from("spinner"),
            LayoutRequest::new().width(26.0).height(26.0),
            move || {
                Box::new(
                    floom::floom_widgets::Spinner::new()
                        .size(24.0)
                        .color(ACCENT)
                        .track(BORDER),
                )
            },
        ));

    page = page.child(card(
        "controls",
        168.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label(
                "t",
                "Toggles, progress and motion",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "b",
                "Switch toggles in place; the two progress bars and the spinner animate themselves.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("controls-row"),
                LayoutRequest::new().height(36.0),
                move || Box::new(controls.clone()),
            ))
            .child(signal_text(
                "bright-readout-slot",
                {
                    let state = bright_read_2.clone();
                    Arc::new(move || {
                        format!(
                            "wired to the same signal: {}",
                            if state.bright.get() { "on" } else { "off" }
                        )
                    })
                },
                12.0,
                FAINT,
            )),
    ));

    page = page.child(card(
        "text",
        176.0,
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(label("h", "Typography", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "body",
                "Text is a leaf with an intrinsic measure: the box is the run, not a magic number.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "mono",
                "Family and weight are part of the measure as well as the paint.",
                13.0,
                FontWeight::Medium,
                INK,
            ))
            .child(Child::new(
                ChildKey::from("divider"),
                LayoutRequest::new().width_pct(100.0).height(1.0),
                move || Box::new(Divider::new().color(BORDER)),
            ))
            .child(paragraph(
                "after",
                "Divider above: a rule whose colour is a value, not a theme lookup.",
                12.0,
                FontWeight::Normal,
                FAINT,
            )),
    ));

    page
}

/// Signals page: `SyncBaseSignal` counters and toggles.
fn page_signals(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Signals",
        "floom-signal's SyncBaseSignal: the thread-safe variant, which is the one a page can hand to a child builder.",
    );

    let counter_row = {
        let dec = state.clone();
        let inc = state.clone();
        let reset = state.clone();
        Stack::new(Axis::Horizontal)
            .gap(10.0)
            .align(Align::Start, Align::Center)
            .child(button_child(
                "dec",
                "-1",
                LayoutRequest::new().width(60.0).height(34.0),
                move || {
                    dec.signal_count.update(|n| *n -= 1);
                },
            ))
            .child(signal_text(
                "value",
                {
                    let state = state.clone();
                    Arc::new(move || format!("value = {}", state.signal_count.get()))
                },
                18.0,
                INK,
            ))
            .child(button_child(
                "inc",
                "+1",
                LayoutRequest::new().width(60.0).height(34.0),
                move || {
                    inc.signal_count.update(|n| *n += 1);
                },
            ))
            .child(button_child(
                "reset",
                "Reset",
                LayoutRequest::new().width(76.0).height(34.0),
                move || {
                    reset.signal_count.set(3);
                },
            ))
    };

    let parity = {
        let state = state.clone();
        Arc::new(move || {
            let n = state.signal_count.get();
            format!(
                "read back every frame: {} is {}",
                n,
                if n % 2 == 0 { "even" } else { "odd" }
            )
        })
    };

    page = page.child(card(
        "counter",
        170.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label("t", "Counter", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "b",
                "The buttons write the signal; the readout widget asks for the current value in update.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("counter-row"),
                LayoutRequest::new().height(40.0),
                move || Box::new(counter_row.clone()),
            ))
            .child(signal_text("parity", parity, 12.0, FAINT)),
    ));

    let toggle_state = state.clone();
    let toggle_read = state.clone();
    let toggles = Stack::new(Axis::Horizontal)
        .gap(14.0)
        .align(Align::Start, Align::Center)
        .child(Child::new(
            ChildKey::from("flag"),
            LayoutRequest::new().height(28.0),
            move || {
                let state = toggle_state.clone();
                Box::new(
                    Switch::new("Feature flag")
                        .on(state.signal_flag.get())
                        .on_toggle(move |on| state.signal_flag.set(on)),
                )
            },
        ))
        .child(signal_text(
            "flag-readout",
            {
                let state = toggle_read.clone();
                Arc::new(move || {
                    format!(
                        "feature = {}",
                        if state.signal_flag.get() {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    )
                })
            },
            13.0,
            INK,
        ));

    page = page.child(card(
        "toggle",
        132.0,
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .align(Align::Start, Align::Start)
            .child(label("t", "Toggle", 16.0, FontWeight::SemiBold, INK))
            .child(Child::new(
                ChildKey::from("toggle-row"),
                LayoutRequest::new().height(32.0),
                move || Box::new(toggles.clone()),
            )),
    ));

    page = page.child(card(
        "why",
        148.0,
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .child(label(
                "t",
                "Why SyncBaseSignal",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "b1",
                "Child::new builders are Fn + Send + Sync, so an Rc-backed RwSignal cannot be captured by one.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "b2",
                "SyncBaseSignal stores its value in the sync runtime and can be cloned into those builders.",
                13.0,
                FontWeight::Normal,
                MUTED,
            )),
    ));

    page
}

/// Kits page: only the ported material crates (md-text, md-button, md-checkbox).
fn page_kits(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Kits",
        "Only the ported material crates are imported: md-text, md-button and md-checkbox.",
    );

    page = page.child(paragraph(
        "note",
        "The floom-ui satellite workspace has 300+ members; each is its own crate, so a demo names the ones it shows.",
        12.0,
        FontWeight::Normal,
        FAINT,
    ));

    page = page.child(card(
        "text",
        128.0,
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(label(
                "t",
                "floom-widget-md-text",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(Child::new(
                ChildKey::from("material-text"),
                LayoutRequest::new(),
                move || {
                    Box::new(
                        floom_material::widgets::text::MaterialText::new(
                            "Material body text, resolved from the theme",
                        )
                        .view(),
                    )
                },
            )),
    ));

    let kit_clicks = state.clone();
    let kit_read = state.clone();
    let kit_buttons = Stack::new(Axis::Horizontal)
        .gap(10.0)
        .align(Align::Start, Align::Center)
        .child(Child::new(
            ChildKey::from("kit-primary"),
            LayoutRequest::new().height(40.0),
            {
                let state = kit_clicks.clone();
                move || {
                    let state = state.clone();
                    Box::new(
                        floom_material::widgets::button::MdButton::new()
                            .label("Primary")
                            .on_click(move || {
                                state.kit_clicks.update(|n| *n += 1);
                            })
                            .view(),
                    )
                }
            },
        ))
        .child(Child::new(
            ChildKey::from("kit-secondary"),
            LayoutRequest::new().height(40.0),
            {
                let state = kit_clicks.clone();
                move || {
                    let state = state.clone();
                    Box::new(
                        floom_material::widgets::button::MdButton::new()
                            .label("Secondary")
                            .variant(floom_core_theme::props::button::ButtonVariant::Secondary)
                            .on_click(move || {
                                state.kit_clicks.update(|n| *n += 1);
                            })
                            .view(),
                    )
                }
            },
        ))
        .child(Child::new(
            ChildKey::from("kit-ghost"),
            LayoutRequest::new().height(40.0),
            move || {
                Box::new(
                    floom_material::widgets::button::MdButton::new()
                        .label("Ghost")
                        .variant(floom_core_theme::props::button::ButtonVariant::Ghost)
                        .view(),
                )
            },
        ))
        .child(Child::new(
            ChildKey::from("kit-disabled"),
            LayoutRequest::new().height(40.0),
            move || {
                Box::new(
                    floom_material::widgets::button::MdButton::new()
                        .label("Disabled")
                        .disabled(true)
                        .view(),
                )
            },
        ));

    page = page.child(card(
        "buttons",
        180.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label(
                "t",
                "floom-widget-md-button",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "b",
                "Four variants from the same adapter; the primary and secondary share a counter.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("kit-buttons-row"),
                LayoutRequest::new().height(40.0),
                move || Box::new(kit_buttons.clone()),
            ))
            .child(signal_text(
                "kit-readout",
                {
                    let state = kit_read.clone();
                    Arc::new(move || format!("kit clicks = {}", state.kit_clicks.get()))
                },
                12.0,
                FAINT,
            )),
    ));

    let checkbox_state = state.clone();
    let checkbox_read = state.clone();
    let checkbox = Child::new(
        ChildKey::from("checkbox"),
        LayoutRequest::new().height(28.0),
        move || {
            let state = checkbox_state.clone();
            Box::new(
                floom_material::widgets::checkbox::MdCheckbox::new("Receive updates")
                    .checked(state.kit_checked.get())
                    .on_change(move |checked| state.kit_checked.set(checked))
                    .view(),
            )
        },
    );

    page = page.child(card(
        "checkbox",
        132.0,
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .align(Align::Start, Align::Start)
            .child(label(
                "t",
                "floom-widget-md-checkbox",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(checkbox)
            .child(signal_text(
                "check-readout",
                {
                    let state = checkbox_read.clone();
                    Arc::new(move || {
                        format!(
                            "checked = {}",
                            if state.kit_checked.get() {
                                "true"
                            } else {
                                "false"
                            }
                        )
                    })
                },
                12.0,
                FAINT,
            )),
    ));

    page
}

/// Editor page: `SharedEditor` mounted as a widget.
fn page_editor(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Editor",
        "floom-editor's document model, mounted as a widget: rope, transactions, undo, selection and caret.",
    );

    let editor = state.editor();

    let undo_editor = editor.clone();
    let redo_editor = editor.clone();
    let reset_editor = editor.clone();

    let toolbar = Stack::new(Axis::Horizontal)
        .gap(10.0)
        .align(Align::Start, Align::Center)
        .child(button_child(
            "undo",
            "Undo",
            LayoutRequest::new().width(86.0).height(34.0),
            move || {
                undo_editor.undo();
            },
        ))
        .child(button_child(
            "redo",
            "Redo",
            LayoutRequest::new().width(86.0).height(34.0),
            move || {
                redo_editor.redo();
            },
        ))
        .child(button_child(
            "reset",
            "Reset sample",
            LayoutRequest::new().width(118.0).height(34.0),
            move || {
                reset_editor.set_text(crate::state::SAMPLE);
            },
        ));

    page = page.child(paragraph(
        "hint",
        "The editor takes the keyboard when this page opens; click it to put the caret at the end. Mod+Z / Mod+Y, Enter, Backspace and Delete are handled by the widget.",
        13.0,
        FontWeight::Normal,
        MUTED,
    ));

    page = page.child(Child::new(
        ChildKey::from("toolbar"),
        LayoutRequest::new().width_pct(100.0).height(36.0),
        move || Box::new(toolbar.clone()),
    ));

    page = page.child(card(
        "editor",
        470.0,
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .align(Align::Start, Align::Stretch)
            .child(label(
                "label",
                "sample.txt",
                12.0,
                FontWeight::Medium,
                FAINT,
            ))
            .child(Child::new(
                ChildKey::from("editor-slot"),
                LayoutRequest::new().width_pct(100.0).height(410.0),
                move || Box::new(SharedEditor::new(editor.clone())),
            )),
    ));

    page
}

/// Devtools page: explains F12 panel shortcuts.
fn page_devtools(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Devtools",
        "The panel is attached by run_desktop_with_devtools, so it is wired for the whole app.",
    );

    page = page.child(card(
        "shortcuts",
        250.0,
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(label(
                "t",
                "Shortcuts",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "f12",
                "F12  (or Cmd/Ctrl+Shift+I)  —  show or hide the panel",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "pick",
                "Ctrl+Shift+C  —  arm the pick, then click any element to inspect it",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "escape",
                "Escape  —  close the panel, and only while it is open",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("divider"),
                LayoutRequest::new().width_pct(100.0).height(1.0),
                move || Box::new(Divider::new().color(BORDER)),
            ))
            .child(paragraph(
                "readout",
                "The panel's readout is answered from this app's own state: route, engine clicks, signal value and kit clicks.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(signal_text(
                "live",
                {
                    let state = state.clone();
                    Arc::new(move || {
                        format!(
                            "live: route {}, engine clicks {}, signal {}",
                            AppRoute::ALL
                                .into_iter()
                                .find(|r| r.id() as u8 == state.route.get())
                                .map(|r| r.path())
                                .unwrap_or("/"),
                            state.clicks.get(),
                            state.signal_count.get()
                        )
                    })
                },
                12.0,
                FAINT,
            )),
    ));

    page
}
