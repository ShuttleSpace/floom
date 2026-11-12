//! The components a reader clicks.

use crate::pages::{page, Category, ComponentPage};

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
        .map(|(component, name)| page(component, name, Category::Actions))
        .collect()
}
