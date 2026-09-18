/// Shared, design-system-agnostic props for the `ListGroup` component.
pub struct ListGroupProps {
    /// Initial collapsed/expanded state.
    pub expanded: bool,
    /// Whether the group's child items are indented (sub-group styling).
    pub inset: bool,
    /// Optional group header text shown next to the expand toggle.
    pub title: Option<String>,
    /// Called with the new expanded state whenever the header is toggled.
    pub on_toggle: Option<Box<dyn Fn(bool)>>,
}

impl Clone for ListGroupProps {
    fn clone(&self) -> Self {
        Self {
            expanded: self.expanded,
            inset: self.inset,
            title: self.title.clone(),
            on_toggle: None,
        }
    }
}

impl std::fmt::Debug for ListGroupProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListGroupProps")
            .field("expanded", &self.expanded)
            .field("inset", &self.inset)
            .field("title", &self.title)
            .field("on_toggle", &self.on_toggle.as_ref().map(|_| "Fn(bool)"))
            .finish()
    }
}

impl Default for ListGroupProps {
    fn default() -> Self {
        Self {
            expanded: false,
            inset: false,
            title: None,
            on_toggle: None,
        }
    }
}
