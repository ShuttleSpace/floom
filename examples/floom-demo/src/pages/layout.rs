//! The components that arrange the page around the content.

use crate::pages::{Category, ComponentPage, page};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("app", "App"),
    ("app_bar", "AppBar"),
    ("bottom_navigation", "BottomNavigation"),
    ("breadcrumbs", "Breadcrumbs"),
    ("container", "Container"),
    ("divider", "Divider"),
    ("footer", "Footer"),
    ("grid", "Grid"),
    ("hover", "Hover"),
    ("main", "Main"),
    ("navigation_drawer", "NavigationDrawer"),
    ("parallax", "Parallax"),
    ("responsive", "Responsive"),
    ("slide_group", "SlideGroup"),
    ("spacer", "Spacer"),
    ("system_bar", "SystemBar"),
    ("tabs", "Tabs"),
    ("toolbar", "Toolbar"),
    ("window", "Window"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| page(component, name, Category::Layout))
        .collect()
}
