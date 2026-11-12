//! The components that tell a reader something happened.
//!
//! Every component here exists in all three design systems with the same
//! builder methods, so a sample is written once and shown three times.
//!
//! Most of these are overlays: they sit above the page and they are not
//! there until something reveals them. An overlay that is not revealed paints
//! nothing, so each sample here reveals the one it shows and says so in its
//! note -- otherwise "it painted" would mean nothing.

use floom::floom_core::WidgetAny;
use floom::floom_types::FontWeight;
use floom::floom_widgets::Text;
use floom::floom_widgets::text_input::shared_flag;
use floom_core_theme::props::alert::AlertVariant;
use floom_core_theme::props::banner::BannerVariant;
use floom_core_theme::props::sheet::SheetSide;

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::compare_children;
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::{compare_row, label};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("alert", "Alert"),
    ("banner", "Banner"),
    ("dialog", "Dialog"),
    ("empty_state", "EmptyState"),
    ("menu", "Menu"),
    ("overlay", "Overlay"),
    ("sheet", "Sheet"),
    ("snackbar", "Snackbar"),
    ("tooltip", "Tooltip"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| match *component {
            "alert" => alert_page(),
            "banner" => banner_page(),
            "dialog" => dialog_page(),
            "empty_state" => empty_state_page(),
            "menu" => menu_page(),
            "overlay" => overlay_page(),
            "sheet" => sheet_page(),
            "snackbar" => snackbar_page(),
            "tooltip" => tooltip_page(),
            _ => page(component, name, Category::Feedback),
        })
        .collect()
}

/// A page in this category: an introduction, its samples and its API.
fn feedback_page(
    component: &'static str,
    name: &'static str,
    intro: &'static str,
    samples: Vec<Sample>,
    api: ApiDoc,
) -> ComponentPage {
    let mut page = page(component, name, Category::Feedback);
    page.intro = intro;
    page.samples = samples;
    page.api = api;
    page
}

// -- Alert ----------------------------------------------------------------

/// The Alert page.
fn alert_page() -> ComponentPage {
    feedback_page(
        "alert",
        "Alert",
        "A message about something that happened, in the page.",
        vec![
            Sample {
                title: "With a title",
                note: None,
                build: alert_title,
            },
            Sample {
                title: "Dismissible",
                note: Some("A dismissible alert has a close button the reader can press."),
                build: alert_dismissible,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_dismiss",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader dismisses the alert.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The alert, with a title and a message.
fn alert_title() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Alert,
        ds::cupertino::Alert,
        ds::fluent::Alert,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .variant(AlertVariant::Info)
                .title("Heads up")
                .message("Something happened that you should know about.")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The alert, dismissible.
fn alert_dismissible() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Alert,
        ds::cupertino::Alert,
        ds::fluent::Alert,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .variant(AlertVariant::Warning)
                .title("Your session is ending")
                .message("Save your work before it closes.")
                .dismissible(true)
                .on_dismiss(|| {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Banner ---------------------------------------------------------------

/// The Banner page.
fn banner_page() -> ComponentPage {
    feedback_page(
        "banner",
        "Banner",
        "A message across the top of the page, with a way to act on it.",
        vec![
            Sample {
                title: "A message",
                note: None,
                build: banner_message,
            },
            Sample {
                title: "With an action",
                note: Some("The action is a button on the right of the banner."),
                build: banner_action,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_action",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader presses the banner's action.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The banner, with a message.
fn banner_message() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Banner,
        ds::cupertino::Banner,
        ds::fluent::Banner,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .variant(BannerVariant::Info)
                .message("A new version of the gallery is ready.")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The banner, with an action.
fn banner_action() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Banner,
        ds::cupertino::Banner,
        ds::fluent::Banner,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .variant(BannerVariant::Warning)
                .message("Your session is ending soon.")
                .action("Save now")
                .on_action(|| {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Dialog ---------------------------------------------------------------

/// The Dialog page.
fn dialog_page() -> ComponentPage {
    feedback_page(
        "dialog",
        "Dialog",
        "A modal card: something the reader has to answer before they go on.",
        vec![
            Sample {
                title: "Open",
                note: Some("A dialog paints nothing until it is opened; this one is open."),
                build: dialog_open,
            },
            Sample {
                title: "Open, with an action",
                note: Some("The action is the button at the bottom of the dialog."),
                build: dialog_action,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_close",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the dialog closes.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Dialog",
                    "Dialog",
                    "Build the engine dialog this component renders.",
                ),
                ApiRow::new(
                    "open_flag",
                    "&self -> floom_widgets::SharedFlag",
                    "SharedFlag",
                    "The flag that says whether the dialog is open; an outside trigger can drive it.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The dialog, open.
fn dialog_open() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Dialog,
        ds::cupertino::Dialog,
        ds::fluent::Dialog,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .open(true)
                .title("Delete this?")
                .content("The page will be gone, and so will anything on it.")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The dialog, open, with an action.
fn dialog_action() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Dialog,
        ds::cupertino::Dialog,
        ds::fluent::Dialog,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .open(true)
                .title("Save your changes?")
                .content("You have unsaved work.")
                .action("Save", || {})
                .on_close(|| {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- EmptyState -----------------------------------------------------------

/// The EmptyState page.
fn empty_state_page() -> ComponentPage {
    feedback_page(
        "empty_state",
        "EmptyState",
        "What a page shows when there is nothing in it yet.",
        vec![
            Sample {
                title: "A title and a description",
                note: None,
                build: empty_state_plain,
            },
            Sample {
                title: "With an icon",
                note: Some("The icon says what the empty thing would have been."),
                build: empty_state_icon,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The empty state, with a title and a description.
fn empty_state_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::EmptyState,
        ds::cupertino::EmptyState,
        ds::fluent::EmptyState,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .title("Nothing here yet")
                .description("Anything you add will show up on this page.")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The empty state, with an icon.
fn empty_state_icon() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::EmptyState,
        ds::cupertino::EmptyState,
        ds::fluent::EmptyState,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .title("Nothing here yet")
                .description("Anything you add will show up on this page.")
                .icon("folder")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Menu -----------------------------------------------------------------

/// The Menu page.
fn menu_page() -> ComponentPage {
    feedback_page(
        "menu",
        "Menu",
        "A list of choices that drops over the page.",
        vec![
            Sample {
                title: "Open",
                note: Some("A menu paints nothing until it is opened; this one is open."),
                build: menu_open,
            },
            Sample {
                title: "Open, with more items",
                note: Some("Each item carries the callback that runs when it is picked."),
                build: menu_many,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_select",
                "impl Fn() + Send + Sync",
                "()",
                "Called when an item is picked.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Menu",
                    "Menu",
                    "Build the engine menu this component renders.",
                ),
                ApiRow::new(
                    "open_flag",
                    "&self -> floom_widgets::SharedFlag",
                    "SharedFlag",
                    "The flag that says whether the menu is open; an outside trigger can drive it.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The menu, open.
fn menu_open() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Menu,
        ds::cupertino::Menu,
        ds::fluent::Menu,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .open(true)
                .item("Cut", || {})
                .item("Copy", || {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The menu, open, with more items.
fn menu_many() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Menu,
        ds::cupertino::Menu,
        ds::fluent::Menu,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .open(true)
                .item("Cut", || {})
                .item("Copy", || {})
                .item("Paste", || {})
                .item("Delete", || {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Overlay --------------------------------------------------------------

/// The Overlay page.
fn overlay_page() -> ComponentPage {
    feedback_page(
        "overlay",
        "Overlay",
        "The layer that dims what is behind it.",
        vec![
            Sample {
                title: "Shown",
                note: Some("An overlay paints nothing until it is shown; this one is."),
                build: overlay_shown,
            },
            Sample {
                title: "Shown, with a click",
                note: Some("The click is what a press outside the content dismisses it with."),
                build: overlay_click,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader presses the overlay itself.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Dialog",
                    "Dialog",
                    "Build the engine dialog this component renders.",
                ),
                ApiRow::new(
                    "show_signal",
                    "&self -> floom_widgets::SharedFlag",
                    "SharedFlag",
                    "The flag that says whether the overlay is shown; an outside trigger can drive it.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The overlay, shown.
fn overlay_shown() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Overlay,
        ds::cupertino::Overlay,
        ds::fluent::Overlay,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .show(true)
                .child(|| Box::new(Text::new("Behind the overlay")))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The overlay, shown, with a click.
fn overlay_click() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Overlay,
        ds::cupertino::Overlay,
        ds::fluent::Overlay,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .show(true)
                .child(|| Box::new(Text::new("Behind the overlay")))
                .on_click(|| {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Sheet ----------------------------------------------------------------

/// The Sheet page.
fn sheet_page() -> ComponentPage {
    feedback_page(
        "sheet",
        "Sheet",
        "A panel that slides in from an edge of the screen.",
        vec![
            Sample {
                title: "Open",
                note: Some("A sheet paints nothing until it is opened; this one is open."),
                build: sheet_open,
            },
            Sample {
                title: "Open, from the side",
                note: Some("The side is the edge the sheet slides in from."),
                build: sheet_side,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Dialog",
                    "Dialog",
                    "Build the engine dialog this component renders.",
                ),
                ApiRow::new(
                    "open_flag",
                    "&self -> floom_widgets::SharedFlag",
                    "SharedFlag",
                    "The flag that says whether the sheet is open; an outside trigger can drive it.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The sheet, open.
fn sheet_open() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Sheet,
        ds::cupertino::Sheet,
        ds::fluent::Sheet,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .open(true)
                .child(label(
                    "content",
                    "A panel that slides up from the bottom.",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The sheet, open, from the side.
fn sheet_side() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Sheet,
        ds::cupertino::Sheet,
        ds::fluent::Sheet,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .open(true)
                .side(SheetSide::End)
                .elevated(true)
                .child(label(
                    "content",
                    "A panel that slides in from the end.",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Snackbar -------------------------------------------------------------

/// The Snackbar page.
fn snackbar_page() -> ComponentPage {
    feedback_page(
        "snackbar",
        "Snackbar",
        "A small message that slides in at the bottom of the screen.",
        vec![
            Sample {
                title: "Shown",
                note: Some("A snackbar paints nothing until its flag is set; this one's is set."),
                build: snackbar_shown,
            },
            Sample {
                title: "Shown, with an action",
                note: Some("The action is the button at the end of the message."),
                build: snackbar_action,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_action",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader presses the snackbar's action.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Overlay",
                    "Overlay",
                    "Build the engine overlay this component renders.",
                ),
                ApiRow::new(
                    "open_flag",
                    "&self -> floom_widgets::SharedFlag",
                    "SharedFlag",
                    "The flag that says whether the snackbar is showing; an outside trigger can drive it.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The snackbar, shown.
fn snackbar_shown() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Snackbar,
        ds::cupertino::Snackbar,
        ds::fluent::Snackbar,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .with_open_flag(shared_flag(true))
                .message("Saved.")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The snackbar, shown, with an action.
fn snackbar_action() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Snackbar,
        ds::cupertino::Snackbar,
        ds::fluent::Snackbar,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .with_open_flag(shared_flag(true))
                .message("The page was deleted.")
                .action_label(Some("Undo".to_string()))
                .on_action(|| {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Tooltip --------------------------------------------------------------

/// The Tooltip page.
fn tooltip_page() -> ComponentPage {
    feedback_page(
        "tooltip",
        "Tooltip",
        "A small bubble that appears when the reader points at a control.",
        vec![
            Sample {
                title: "Wrapping a control",
                note: Some(
                    "The bubble appears when the reader points at the control; the control itself is what you see here.",
                ),
                build: tooltip_control,
            },
            Sample {
                title: "A different bubble",
                note: Some("The text is what the bubble says once it is up."),
                build: tooltip_text,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Tooltip",
                "Tooltip",
                "Build the engine tooltip this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The tooltip, wrapping a control.
fn tooltip_control() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Tooltip,
        ds::cupertino::Tooltip,
        ds::fluent::Tooltip,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .text("Save the page")
                .child(label(
                    "control",
                    "Hover me",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The tooltip, with a different bubble.
fn tooltip_text() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Tooltip,
        ds::cupertino::Tooltip,
        ds::fluent::Tooltip,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .text("Three design systems, side by side")
                .child(label(
                    "control",
                    "Hover me",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}
