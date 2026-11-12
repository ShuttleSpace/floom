//! The components a reader reads.

use crate::pages::{Category, ComponentPage, page};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("avatar", "Avatar"),
    ("badge", "Badge"),
    ("calendar", "Calendar"),
    ("card", "Card"),
    ("carousel", "Carousel"),
    ("chip", "Chip"),
    ("code", "Code"),
    ("data_iterator", "DataIterator"),
    ("datatable", "DataTable"),
    ("icon", "Icon"),
    ("image", "Image"),
    ("kbd", "Kbd"),
    ("list", "List"),
    ("list_item", "ListItem"),
    ("pagination", "Pagination"),
    ("sparkline", "Sparkline"),
    ("table", "Table"),
    ("text", "Text"),
    ("timeline", "Timeline"),
    ("treeview", "Treeview"),
    ("virtual_scroller", "VirtualScroller"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| page(component, name, Category::DataDisplay))
        .collect()
}
