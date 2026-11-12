//! The components that tell a reader something happened.

use crate::pages::{page, Category, ComponentPage};

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
        .map(|(component, name)| page(component, name, Category::Feedback))
        .collect()
}
