//! The components a reader clicks.
//!
//! Every component here exists in all three design systems with the same
//! builder methods, so a sample is written once and shown three times. A
//! sample that shows several states of one component puts them side by side
//! in a column; a sample that shows one component is one widget in a column.

use floom::floom_core::{Axis, WidgetAny};
use floom::floom_types::FontWeight;
use floom::floom_widgets::{Stack, Text};
use floom_core_theme::props::button::{ButtonSize, ButtonVariant};
use floom_core_theme::props::icon_button::IconButtonVariant;
use floom_core_theme::props::speed_dial::SpeedDialDirection;

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::{compare_children, compare_with, ds_icon, themed_widget};
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::{compare_row, label, widget};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("button", "Button"),
    ("button_group", "ButtonGroup"),
    ("checkbox_btn", "CheckboxBtn"),
    ("chip_group", "ChipGroup"),
    ("confirm_edit", "ConfirmEdit"),
    ("counter", "Counter"),
    ("expansion_panels", "ExpansionPanels"),
    ("floating_action_button", "FloatingActionButton"),
    ("icon_button", "IconButton"),
    ("item_group", "ItemGroup"),
    ("list_group", "ListGroup"),
    ("list_subheader", "ListSubheader"),
    ("radio", "Radio"),
    ("radio-group", "RadioGroup"),
    ("rating", "Rating"),
    ("speed_dial", "SpeedDial"),
    ("stepper", "Stepper"),
    ("switch", "Switch"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| match *component {
            "button" => button_page(),
            "button_group" => button_group_page(),
            "checkbox_btn" => checkbox_btn_page(),
            "chip_group" => chip_group_page(),
            "confirm_edit" => confirm_edit_page(),
            "counter" => counter_page(),
            "expansion_panels" => expansion_panels_page(),
            "floating_action_button" => floating_action_button_page(),
            "icon_button" => icon_button_page(),
            "item_group" => item_group_page(),
            "list_group" => list_group_page(),
            "list_subheader" => list_subheader_page(),
            "radio" => radio_page(),
            "radio-group" => radio_group_page(),
            "rating" => rating_page(),
            "speed_dial" => speed_dial_page(),
            "stepper" => stepper_page(),
            "switch" => switch_page(),
            _ => page(component, name, Category::Actions),
        })
        .collect()
}

/// A page in this category: an introduction, its samples and its API.
fn actions_page(
    component: &'static str,
    name: &'static str,
    intro: &'static str,
    samples: Vec<Sample>,
    api: ApiDoc,
) -> ComponentPage {
    let mut page = page(component, name, Category::Actions);
    page.intro = intro;
    page.samples = samples;
    page.api = api;
    page
}

// -- Button ---------------------------------------------------------------

/// The Button page.
fn button_page() -> ComponentPage {
    actions_page(
        "button",
        "Button",
        "A button: a label, a variant, a size, and the states a reader can put it in.",
        vec![
            Sample {
                title: "Variants",
                note: None,
                build: button_variants,
            },
            Sample {
                title: "Sizes and states",
                note: Some("A disabled button still paints; it just stops responding."),
                build: button_states,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the button is activated.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "&self -> floom_widgets::Button",
                    "Button",
                    "Build the engine button this component renders.",
                ),
                ApiRow::new(
                    "resolve",
                    "&self -> floom_widgets::Button",
                    "Button",
                    "Build the engine button, resolving the theme's tokens first.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The button's five variants, side by side.
fn button_variants() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Button,
        ds::cupertino::Button,
        ds::fluent::Button,
        |b, theme| {
            Stack::new(Axis::Horizontal)
                .gap(8.0)
                .child(widget(
                    "primary",
                    themed_widget!(b, theme, |c| c
                        .label("Primary")
                        .variant(ButtonVariant::Primary)
                        .view()),
                ))
                .child(widget(
                    "secondary",
                    themed_widget!(b, theme, |c| c
                        .label("Secondary")
                        .variant(ButtonVariant::Secondary)
                        .view()),
                ))
                .child(widget(
                    "danger",
                    themed_widget!(b, theme, |c| c
                        .label("Danger")
                        .variant(ButtonVariant::Danger)
                        .view()),
                ))
                .child(widget(
                    "ghost",
                    themed_widget!(b, theme, |c| c
                        .label("Ghost")
                        .variant(ButtonVariant::Ghost)
                        .view()),
                ))
                .child(widget(
                    "text",
                    themed_widget!(b, theme, |c| c
                        .label("Text")
                        .variant(ButtonVariant::Text)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The button's three sizes, plus its disabled and loading states.
fn button_states() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Button,
        ds::cupertino::Button,
        ds::fluent::Button,
        |b, theme| {
            Stack::new(Axis::Horizontal)
                .gap(8.0)
                .child(widget(
                    "small",
                    themed_widget!(b, theme, |c| c
                        .label("Small")
                        .size(ButtonSize::Small)
                        .view()),
                ))
                .child(widget(
                    "medium",
                    themed_widget!(b, theme, |c| c
                        .label("Medium")
                        .size(ButtonSize::Medium)
                        .view()),
                ))
                .child(widget(
                    "large",
                    themed_widget!(b, theme, |c| c
                        .label("Large")
                        .size(ButtonSize::Large)
                        .view()),
                ))
                .child(widget(
                    "disabled",
                    themed_widget!(b, theme, |c| c.label("Disabled").disabled(true).view()),
                ))
                .child(widget(
                    "loading",
                    themed_widget!(b, theme, |c| c.label("Loading").loading(true).view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ButtonGroup ----------------------------------------------------------

/// The ButtonGroup page.
fn button_group_page() -> ComponentPage {
    actions_page(
        "button_group",
        "ButtonGroup",
        "A row or column of buttons, optionally joined so they read as one control.",
        vec![
            Sample {
                title: "A row of buttons",
                note: None,
                build: button_group_row,
            },
            Sample {
                title: "Joined and stacked",
                note: Some("A joined group shares its edges; a vertical one stacks."),
                build: button_group_joined,
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

/// Three buttons in a group.
fn button_group_row() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_with!(
        ds::material::ButtonGroup,
        ds::cupertino::ButtonGroup,
        ds::fluent::ButtonGroup,
        ds::material::Button,
        ds::cupertino::Button,
        ds::fluent::Button,
        |b, btn, theme| {
            b::new()
                .with_theme(theme.clone())
                .child(themed_widget!(btn, theme, |c| c.label("One").view()))
                .child(themed_widget!(btn, theme, |c| c.label("Two").view()))
                .child(themed_widget!(btn, theme, |c| c.label("Three").view()))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// A joined group and a vertical one.
fn button_group_joined() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_with!(
        ds::material::ButtonGroup,
        ds::cupertino::ButtonGroup,
        ds::fluent::ButtonGroup,
        ds::material::Button,
        ds::cupertino::Button,
        ds::fluent::Button,
        |b, btn, theme| {
            Stack::new(Axis::Vertical)
                .gap(12.0)
                .child(widget(
                    "joined",
                    themed_widget!(b, theme, |c| c
                        .joined(true)
                        .child(themed_widget!(btn, theme, |c| c.label("Left").view()))
                        .child(themed_widget!(btn, theme, |c| c.label("Right").view()))
                        .view()),
                ))
                .child(widget(
                    "vertical",
                    themed_widget!(b, theme, |c| c
                        .vertical(true)
                        .child(themed_widget!(btn, theme, |c| c.label("Up").view()))
                        .child(themed_widget!(btn, theme, |c| c.label("Down").view()))
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- CheckboxBtn ----------------------------------------------------------

/// The CheckboxBtn page.
fn checkbox_btn_page() -> ComponentPage {
    actions_page(
        "checkbox_btn",
        "CheckboxBtn",
        "A checkbox with a label beside it: the thing a reader ticks.",
        vec![
            Sample {
                title: "Unticked and ticked",
                note: None,
                build: checkbox_btn_states,
            },
            Sample {
                title: "Disabled",
                note: Some("A disabled checkbox keeps its tick; it just stops responding."),
                build: checkbox_btn_disabled,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(bool) + Send + Sync",
                "bool",
                "Called with the new ticked state when the reader changes it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The checkbox, unticked and ticked.
fn checkbox_btn_states() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::CheckboxBtn,
        ds::cupertino::CheckboxBtn,
        ds::fluent::CheckboxBtn,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "unticked",
                    themed_widget!(b, theme, |c| c.label("Send me a receipt").view()),
                ))
                .child(widget(
                    "ticked",
                    themed_widget!(b, theme, |c| c
                        .label("Send me a receipt")
                        .checked(true)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The checkbox, disabled.
fn checkbox_btn_disabled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::CheckboxBtn,
        ds::cupertino::CheckboxBtn,
        ds::fluent::CheckboxBtn,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "unticked",
                    themed_widget!(b, theme, |c| c.label("Unavailable").disabled(true).view()),
                ))
                .child(widget(
                    "ticked",
                    themed_widget!(b, theme, |c| c
                        .label("Unavailable")
                        .checked(true)
                        .disabled(true)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ChipGroup ------------------------------------------------------------

/// The ChipGroup page.
fn chip_group_page() -> ComponentPage {
    actions_page(
        "chip_group",
        "ChipGroup",
        "A set of chips the reader picks from: one at a time, or several.",
        vec![
            Sample {
                title: "One at a time",
                note: None,
                build: chip_group_single,
            },
            Sample {
                title: "Several at once",
                note: Some("A mandatory multiple group always keeps one chip chosen."),
                build: chip_group_multiple,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(Vec<usize>) + Send + Sync",
                "Vec<usize>",
                "Called with the indices of the chosen chips when they change.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::SelectGroup",
                "SelectGroup",
                "Build the engine select group this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The chip group, single selection.
fn chip_group_single() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ChipGroup,
        ds::cupertino::ChipGroup,
        ds::fluent::ChipGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .chip("Rust")
                .chip("Floom")
                .chip("Widgets")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The chip group, multiple selection.
fn chip_group_multiple() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ChipGroup,
        ds::cupertino::ChipGroup,
        ds::fluent::ChipGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .chip("Rust")
                .chip("Floom")
                .chip("Widgets")
                .multiple(true)
                .mandatory(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ConfirmEdit ----------------------------------------------------------

/// The ConfirmEdit page.
fn confirm_edit_page() -> ComponentPage {
    actions_page(
        "confirm_edit",
        "ConfirmEdit",
        "A value the reader can edit in place, and confirm when they are done.",
        vec![
            Sample {
                title: "Read only",
                note: None,
                build: confirm_edit_read_only,
            },
            Sample {
                title: "With a value",
                note: Some("The value is the text the reader starts from."),
                build: confirm_edit_value,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_save",
                "impl Fn(String) + Send + Sync",
                "String",
                "Called with the edited text when the reader saves.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::ConfirmEdit",
                    "ConfirmEdit",
                    "Build the engine confirm-edit this component renders.",
                ),
                ApiRow::new(
                    "is_editing",
                    "&self -> bool",
                    "false",
                    "Whether the reader is editing right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The confirm-edit, showing its child.
fn confirm_edit_read_only() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ConfirmEdit,
        ds::cupertino::ConfirmEdit,
        ds::fluent::ConfirmEdit,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .child(label(
                    "value",
                    "Floom component gallery",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The confirm-edit, with a value to edit.
fn confirm_edit_value() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ConfirmEdit,
        ds::cupertino::ConfirmEdit,
        ds::fluent::ConfirmEdit,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value("Floom component gallery")
                .child(label(
                    "value",
                    "Floom component gallery",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Counter --------------------------------------------------------------

/// The Counter page.
fn counter_page() -> ComponentPage {
    actions_page(
        "counter",
        "Counter",
        "A number the reader steps up and down, within a range.",
        vec![
            Sample {
                title: "From zero to ten",
                note: None,
                build: counter_default,
            },
            Sample {
                title: "A wider range",
                note: Some("The step is how far one press moves the number."),
                build: counter_range,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(i64) + Send + Sync",
                "i64",
                "Called with the new value when the reader steps it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Counter",
                "Counter",
                "Build the engine counter this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The counter, in its ordinary range.
fn counter_default() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Counter,
        ds::cupertino::Counter,
        ds::fluent::Counter,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(0)
                .min(0)
                .max(10)
                .step(1)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The counter, in a wider range.
fn counter_range() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Counter,
        ds::cupertino::Counter,
        ds::fluent::Counter,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(0)
                .min(-5)
                .max(5)
                .step(5)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ExpansionPanels ------------------------------------------------------

/// The ExpansionPanels page.
fn expansion_panels_page() -> ComponentPage {
    actions_page(
        "expansion_panels",
        "ExpansionPanels",
        "A set of panels the reader opens to see more.",
        vec![
            Sample {
                title: "One panel",
                note: None,
                build: expansion_panels_one,
            },
            Sample {
                title: "Several panels",
                note: Some("A multiple group lets more than one panel stand open."),
                build: expansion_panels_many,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize, bool) + Send + Sync",
                "(usize, bool)",
                "Called with the panel's index and whether it opened when it changes.",
            )],
            methods: vec![ApiRow::new(
                "resolve",
                "&self -> floom_ui_composites::Accordion",
                "Accordion",
                "Build the engine accordion this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// One panel, closed.
fn expansion_panels_one() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ExpansionPanels,
        ds::cupertino::ExpansionPanels,
        ds::fluent::ExpansionPanels,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .panel("What is this?", || {
                    Box::new(Text::new(
                        "A gallery of every component, in three design systems.",
                    ))
                })
                .resolve()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// Two panels, more than one open at a time.
fn expansion_panels_many() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ExpansionPanels,
        ds::cupertino::ExpansionPanels,
        ds::fluent::ExpansionPanels,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .multiple(true)
                .panel("First", || {
                    Box::new(Text::new("The first panel's contents."))
                })
                .panel("Second", || {
                    Box::new(Text::new("The second panel's contents."))
                })
                .resolve()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- FloatingActionButton -------------------------------------------------

/// The FloatingActionButton page.
fn floating_action_button_page() -> ComponentPage {
    actions_page(
        "floating_action_button",
        "FloatingActionButton",
        "The round button that floats over the page: one action, always in reach.",
        vec![
            Sample {
                title: "With a glyph",
                note: None,
                build: fab_icon_only,
            },
            Sample {
                title: "Extended",
                note: Some("An extended button carries a label as well as a glyph."),
                build: fab_extended,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the button is activated.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Pressable",
                "Pressable",
                "Build the engine pressable this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The floating action button, with a glyph.
fn fab_icon_only() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_with!(
        ds::material::FloatingActionButton,
        ds::cupertino::FloatingActionButton,
        ds::fluent::FloatingActionButton,
        ds::material::Icon,
        ds::cupertino::Icon,
        ds::fluent::Icon,
        |b, i, theme| {
            b::new()
                .with_theme(theme.clone())
                .child(ds_icon!(i, theme, "add"))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The floating action button, extended with a label.
fn fab_extended() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_with!(
        ds::material::FloatingActionButton,
        ds::cupertino::FloatingActionButton,
        ds::fluent::FloatingActionButton,
        ds::material::Icon,
        ds::cupertino::Icon,
        ds::fluent::Icon,
        |b, i, theme| {
            b::new()
                .with_theme(theme.clone())
                .child(ds_icon!(i, theme, "add"))
                .label("Create")
                .extended(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- IconButton -----------------------------------------------------------

/// The IconButton page.
fn icon_button_page() -> ComponentPage {
    actions_page(
        "icon_button",
        "IconButton",
        "A button that is nothing but an icon: the glyph names the action.",
        vec![
            Sample {
                title: "Plain",
                note: None,
                build: icon_button_plain,
            },
            Sample {
                title: "Contained and sized",
                note: Some("A contained button draws a filled shape behind the glyph."),
                build: icon_button_contained,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the button is activated.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::IconButton",
                "IconButton",
                "Build the engine icon button this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The icon button, plain.
fn icon_button_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::IconButton,
        ds::cupertino::IconButton,
        ds::fluent::IconButton,
        |b, theme| {
            Stack::new(Axis::Horizontal)
                .gap(12.0)
                .child(widget(
                    "close",
                    themed_widget!(b, theme, "close", |c| c.view()),
                ))
                .child(widget("add", themed_widget!(b, theme, "add", |c| c.view())))
                .child(widget(
                    "delete",
                    themed_widget!(b, theme, "delete", |c| c.view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The icon button, contained and at an explicit size.
fn icon_button_contained() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::IconButton,
        ds::cupertino::IconButton,
        ds::fluent::IconButton,
        |b, theme| {
            Stack::new(Axis::Horizontal)
                .gap(12.0)
                .child(widget(
                    "contained",
                    themed_widget!(b, theme, "settings", |c| c
                        .variant(IconButtonVariant::Contained)
                        .view()),
                ))
                .child(widget(
                    "outlined",
                    themed_widget!(b, theme, "settings", |c| c
                        .variant(IconButtonVariant::Outlined)
                        .view()),
                ))
                .child(widget(
                    "large",
                    themed_widget!(b, theme, "settings", |c| c
                        .size(48.0)
                        .variant(IconButtonVariant::Contained)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ItemGroup ------------------------------------------------------------

/// The ItemGroup page.
fn item_group_page() -> ComponentPage {
    actions_page(
        "item_group",
        "ItemGroup",
        "A group of rows the reader picks from -- the settings-list shape.",
        vec![
            Sample {
                title: "Labelled rows",
                note: None,
                build: item_group_rows,
            },
            Sample {
                title: "Several rows at once",
                note: Some("A multiple group lets more than one row be chosen."),
                build: item_group_multiple,
            },
        ],
        ApiDoc {
            events: vec![
                ApiRow::new(
                    "on_change_single",
                    "impl Fn(usize) + Send + Sync",
                    "usize",
                    "Called with the chosen row's index in a single-selection group.",
                ),
                ApiRow::new(
                    "on_change_multi",
                    "impl Fn(Vec<usize>) + Send + Sync",
                    "Vec<usize>",
                    "Called with the chosen rows' indices in a multiple-selection group.",
                ),
            ],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::SelectGroup",
                    "SelectGroup",
                    "Build the engine select group this component renders.",
                ),
                ApiRow::new(
                    "selected",
                    "&SelectGroup -> Option<usize>",
                    "None",
                    "Which row is chosen, in a single-selection group.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The item group, with labelled rows.
fn item_group_rows() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ItemGroup,
        ds::cupertino::ItemGroup,
        ds::fluent::ItemGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .row("Notifications", || {
                    Box::new(Text::new("Email and push, weekly."))
                })
                .row("Privacy", || {
                    Box::new(Text::new("What this device may share."))
                })
                .view(None::<fn(usize)>, None::<fn(Vec<usize>)>)
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The item group, with more than one row chosen.
fn item_group_multiple() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ItemGroup,
        ds::cupertino::ItemGroup,
        ds::fluent::ItemGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .multiple(true)
                .row("Rust", || Box::new(Text::new("The language.")))
                .row("Floom", || Box::new(Text::new("The toolkit.")))
                .view(None::<fn(usize)>, None::<fn(Vec<usize>)>)
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ListGroup ------------------------------------------------------------

/// The ListGroup page.
fn list_group_page() -> ComponentPage {
    actions_page(
        "list_group",
        "ListGroup",
        "A titled group of rows that folds open and shut.",
        vec![
            Sample {
                title: "A titled group",
                note: None,
                build: list_group_titled,
            },
            Sample {
                title: "Open and inset",
                note: Some("An inset group sits inside the page's margin."),
                build: list_group_open,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_toggle",
                "impl Fn(bool) + Send + Sync",
                "bool",
                "Called with whether the group is open when it changes.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Accordion",
                "Accordion",
                "Build the engine accordion this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The list group, with a title and two items.
fn list_group_titled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ListGroup,
        ds::cupertino::ListGroup,
        ds::fluent::ListGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .title("Favourites")
                .item("Rust")
                .item("Floom")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The list group, open and inset.
fn list_group_open() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ListGroup,
        ds::cupertino::ListGroup,
        ds::fluent::ListGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .title("Favourites")
                .item("Rust")
                .item("Floom")
                .expanded(true)
                .inset(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ListSubheader --------------------------------------------------------

/// The ListSubheader page.
fn list_subheader_page() -> ComponentPage {
    actions_page(
        "list_subheader",
        "ListSubheader",
        "The heading that separates one part of a list from the next.",
        vec![
            Sample {
                title: "Plain",
                note: None,
                build: list_subheader_plain,
            },
            Sample {
                title: "Inset",
                note: Some("An inset heading lines up with inset rows."),
                build: list_subheader_inset,
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

/// The list subheader, plain.
fn list_subheader_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ListSubheader,
        ds::cupertino::ListSubheader,
        ds::fluent::ListSubheader,
        |b, theme| b::new("Section").with_theme(theme).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The list subheader, inset.
fn list_subheader_inset() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ListSubheader,
        ds::cupertino::ListSubheader,
        ds::fluent::ListSubheader,
        |b, theme| b::new("Section").with_theme(theme).inset(true).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Radio ----------------------------------------------------------------

/// The Radio page.
fn radio_page() -> ComponentPage {
    actions_page(
        "radio",
        "Radio",
        "One choice in a set: the reader ticks the one they want.",
        vec![
            Sample {
                title: "Unticked and ticked",
                note: None,
                build: radio_states,
            },
            Sample {
                title: "Disabled",
                note: Some("A disabled radio keeps its tick; it just stops responding."),
                build: radio_disabled,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(bool)",
                "bool",
                "Called with the new ticked state when the reader changes it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Radio",
                "Radio",
                "Build the engine radio this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The radio, unticked and ticked.
fn radio_states() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Radio,
        ds::cupertino::Radio,
        ds::fluent::Radio,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "unticked",
                    themed_widget!(b, theme, "Standard delivery", |c| c.view()),
                ))
                .child(widget(
                    "ticked",
                    themed_widget!(b, theme, "Express delivery", |c| c.checked(true).view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The radio, disabled.
fn radio_disabled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Radio,
        ds::cupertino::Radio,
        ds::fluent::Radio,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "unticked",
                    themed_widget!(b, theme, "Overnight delivery", |c| c.disabled(true).view()),
                ))
                .child(widget(
                    "ticked",
                    themed_widget!(b, theme, "Overnight delivery", |c| c
                        .checked(true)
                        .disabled(true)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- RadioGroup -----------------------------------------------------------

/// The RadioGroup page.
fn radio_group_page() -> ComponentPage {
    actions_page(
        "radio-group",
        "RadioGroup",
        "A set of radios the reader picks exactly one of.",
        vec![
            Sample {
                title: "Three options",
                note: None,
                build: radio_group_options,
            },
            Sample {
                title: "One chosen, and disabled",
                note: Some("A disabled group keeps its choice; it just stops responding."),
                build: radio_group_disabled,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the chosen option's index when it changes.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::SelectGroup",
                "SelectGroup",
                "Build the engine select group this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The radio group, with three options.
fn radio_group_options() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::RadioGroup,
        ds::cupertino::RadioGroup,
        ds::fluent::RadioGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Material".to_string(),
                    "Cupertino".to_string(),
                    "Fluent".to_string(),
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The radio group, with one option chosen and the whole group disabled.
fn radio_group_disabled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::RadioGroup,
        ds::cupertino::RadioGroup,
        ds::fluent::RadioGroup,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Material".to_string(),
                    "Cupertino".to_string(),
                    "Fluent".to_string(),
                ])
                .selected(1)
                .disabled(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Rating ---------------------------------------------------------------

/// The Rating page.
fn rating_page() -> ComponentPage {
    actions_page(
        "rating",
        "Rating",
        "A row of stars the reader sets: how many of them.",
        vec![
            Sample {
                title: "Three of five",
                note: None,
                build: rating_default,
            },
            Sample {
                title: "Read only",
                note: Some("A read-only rating shows a value without letting it change."),
                build: rating_readonly,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(f32) + Send + Sync",
                "f32",
                "Called with the new rating when the reader sets it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The rating, three of five.
fn rating_default() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Rating,
        ds::cupertino::Rating,
        ds::fluent::Rating,
        |b, theme| b::new().with_theme(theme).value(3.0).max(5).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The rating, read only.
fn rating_readonly() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Rating,
        ds::cupertino::Rating,
        ds::fluent::Rating,
        |b, theme| b::new()
            .with_theme(theme)
            .value(4.0)
            .max(10)
            .readonly(true)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- SpeedDial ------------------------------------------------------------

/// The SpeedDial page.
fn speed_dial_page() -> ComponentPage {
    actions_page(
        "speed_dial",
        "SpeedDial",
        "A button that fans out into several actions when it is pressed.",
        vec![
            Sample {
                title: "Closed",
                note: None,
                build: speed_dial_closed,
            },
            Sample {
                title: "Open",
                note: Some("The direction decides which way the actions fan out."),
                build: speed_dial_open,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_action",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the action's index when the reader picks one.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The speed dial, closed.
fn speed_dial_closed() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SpeedDial,
        ds::cupertino::SpeedDial,
        ds::fluent::SpeedDial,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .fab_icon("add")
                .action("edit", Some("Edit".to_string()))
                .action("delete", Some("Delete".to_string()))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The speed dial, open.
fn speed_dial_open() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SpeedDial,
        ds::cupertino::SpeedDial,
        ds::fluent::SpeedDial,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .fab_icon("add")
                .action("edit", Some("Edit".to_string()))
                .action("delete", Some("Delete".to_string()))
                .direction(SpeedDialDirection::Up)
                .open(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Stepper --------------------------------------------------------------

/// The Stepper page.
fn stepper_page() -> ComponentPage {
    actions_page(
        "stepper",
        "Stepper",
        "A row of steps the reader is at: where they are, and what is left.",
        vec![
            Sample {
                title: "Three steps",
                note: None,
                build: stepper_three,
            },
            Sample {
                title: "Editable and non-linear",
                note: Some("A non-linear stepper lets the reader jump to any step."),
                build: stepper_editable,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the chosen step's index when it changes.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Stepper",
                "Stepper",
                "Build the engine stepper this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The stepper, with three steps.
fn stepper_three() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Stepper,
        ds::cupertino::Stepper,
        ds::fluent::Stepper,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .items(vec![
                    "Choose".to_string(),
                    "Review".to_string(),
                    "Done".to_string(),
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The stepper, editable and non-linear.
fn stepper_editable() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Stepper,
        ds::cupertino::Stepper,
        ds::fluent::Stepper,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .items(vec![
                    "Choose".to_string(),
                    "Review".to_string(),
                    "Done".to_string(),
                ])
                .selected(1)
                .editable(true)
                .non_linear(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Switch ---------------------------------------------------------------

/// The Switch page.
fn switch_page() -> ComponentPage {
    actions_page(
        "switch",
        "Switch",
        "A switch that is on or off: the setting itself, not a choice.",
        vec![
            Sample {
                title: "Off and on",
                note: None,
                build: switch_states,
            },
            Sample {
                title: "Disabled",
                note: Some("A disabled switch keeps its position; it just stops responding."),
                build: switch_disabled,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(bool)",
                "bool",
                "Called with the new position when the reader flips it.",
            )],
            methods: vec![ApiRow::new(
                "switch",
                "&self -> floom_widgets::Switch",
                "Switch",
                "Build the engine switch this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The switch, off and on.
fn switch_states() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Switch,
        ds::cupertino::Switch,
        ds::fluent::Switch,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget("off", themed_widget!(b, theme, |c| c.switch())))
                .child(widget(
                    "on",
                    themed_widget!(b, theme, |c| c.checked(true).switch()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The switch, disabled.
fn switch_disabled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Switch,
        ds::cupertino::Switch,
        ds::fluent::Switch,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "off",
                    themed_widget!(b, theme, |c| c.disabled(true).switch()),
                ))
                .child(widget(
                    "on",
                    themed_widget!(b, theme, |c| c.checked(true).disabled(true).switch()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}
