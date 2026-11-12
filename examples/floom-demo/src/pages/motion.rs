//! The components that move, progress, and provide context to their children.

use crate::pages::{Category, ComponentPage, page};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("date_adapter_provider", "DateAdapterProvider"),
    ("defaults_provider", "DefaultsProvider"),
    ("display_provider", "DisplayProvider"),
    ("expand_transition", "ExpandTransition"),
    ("fade_transition", "FadeTransition"),
    ("icons_provider", "IconsProvider"),
    ("infinite_scroller", "InfiniteScroller"),
    ("locale_provider", "LocaleProvider"),
    ("progress_circular", "ProgressCircular"),
    ("progress_linear", "ProgressLinear"),
    ("pull_to_refresh", "PullToRefresh"),
    ("scale_transition", "ScaleTransition"),
    ("skeleton_loader", "SkeletonLoader"),
    ("slide_transition", "SlideTransition"),
    ("theme_provider", "ThemeProvider"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| page(component, name, Category::Motion))
        .collect()
}
