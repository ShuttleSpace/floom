//! The components a reader clicks.

use floom::floom_core::WidgetAny;

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::compare_children;
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::compare_row;

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
        .map(|(component, name)| {
            if *component == "button" {
                let mut page = page(component, name, Category::Actions);
                page.intro =
                    "The button: a label, a variant, a size, and the states a reader can put it in.";
                page.samples = vec![Sample {
                    title: "Variants",
                    note: None,
                    build: button_variants,
                }];
                page.api = ApiDoc {
                    methods: vec![ApiRow::new(
                        "view",
                        "() -> floom_widgets::Button",
                        "",
                        "Build the engine button this component renders.",
                    )],
                    ..Default::default()
                };
                page
            } else {
                page(component, name, Category::Actions)
            }
        })
        .collect()
}

/// A button in its three design systems.
fn button_variants() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Button,
        ds::cupertino::Button,
        ds::fluent::Button,
        |b, theme| b::new().with_theme(theme).label("Click me").view()
    );
    Box::new(compare_row((m, c, f)))
}
