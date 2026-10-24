#[derive(Debug, Clone, PartialEq)]
pub struct TimelineItem {
    /// Heading shown next to the timeline dot.
    pub title: String,
    /// Supporting text shown under the title.
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct TimelineProps {
    /// Ordered list of events, top to bottom.
    pub items: Vec<TimelineItem>,
}

impl Default for TimelineProps {
    fn default() -> Self {
        Self { items: Vec::new() }
    }
}
