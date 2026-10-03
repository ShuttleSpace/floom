/// Neutral, design-system-agnostic props for the virtual scroller.
///
/// Item content is passed through the widget builder (e.g. `items(Vec<AnyView>)`)
/// rather than these props, because `AnyView` is not `Clone` and props must be.
pub struct VirtualScrollerProps {
    /// Height of a single row in pixels (must be fixed for windowing math).
    pub item_height: f32,
    /// Gap between rows in pixels.
    pub row_gap: f32,
    /// Inner padding in pixels.
    pub padding: f32,
    /// Extra rows rendered above/below the viewport to smooth fast scrolling.
    pub overscan: usize,
    /// Called when the user scrolls within `overscan` of the last item.
    pub on_reach_end: Option<Box<dyn Fn()>>,
}

impl Clone for VirtualScrollerProps {
    fn clone(&self) -> Self {
        Self {
            item_height: self.item_height,
            row_gap: self.row_gap,
            padding: self.padding,
            overscan: self.overscan,
            on_reach_end: None,
        }
    }
}

impl std::fmt::Debug for VirtualScrollerProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VirtualScrollerProps")
            .field("item_height", &self.item_height)
            .field("row_gap", &self.row_gap)
            .field("padding", &self.padding)
            .field("overscan", &self.overscan)
            .field("on_reach_end", &self.on_reach_end.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for VirtualScrollerProps {
    fn default() -> Self {
        Self {
            item_height: 44.0,
            row_gap: 0.0,
            padding: 0.0,
            overscan: 4,
            on_reach_end: None,
        }
    }
}
