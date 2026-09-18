pub struct TabsProps {
    /// Tab labels, in display order.
    pub tabs: Vec<String>,
    /// Index of the currently active tab.
    pub selected: usize,
    /// Called with the selected index when the active tab changes.
    pub on_change: Option<Box<dyn Fn(usize)>>,
}

impl Clone for TabsProps {
    fn clone(&self) -> Self {
        Self {
            tabs: self.tabs.clone(),
            selected: self.selected,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for TabsProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TabsProps")
            .field("tabs", &self.tabs)
            .field("selected", &self.selected)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(usize)"))
            .finish()
    }
}

impl Default for TabsProps {
    fn default() -> Self {
        Self {
            tabs: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }
}
