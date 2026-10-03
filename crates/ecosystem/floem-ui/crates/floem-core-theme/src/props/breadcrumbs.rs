/// Neutral props shared by every design system's `Breadcrumbs`.
///
/// A `Breadcrumbs` renders a horizontal trail of items separated by a divider.
/// The last item is the current location and is non-clickable; earlier items
/// are clickable and fire `on_click(index)` so the consumer can navigate.
#[derive(Debug, Clone)]
pub struct BreadcrumbsProps {
    /// Gap between items and dividers in dp.
    pub gap: f32,
    /// Divider string rendered between items.
    pub divider: String,
    /// Item font size in dp.
    pub font_size: f32,
}

impl Default for BreadcrumbsProps {
    fn default() -> Self {
        Self {
            gap: 8.0,
            divider: "/".to_string(),
            font_size: 14.0,
        }
    }
}
