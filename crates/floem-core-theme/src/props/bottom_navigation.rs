pub struct BottomNavigationProps {
    /// Labels for each navigation destination, in order.
    pub labels: Vec<String>,
    /// Index of the currently selected destination.
    pub selected: usize,
    /// Called with the newly selected index when a destination is tapped.
    pub on_change: Option<Box<dyn Fn(usize)>>,
}

impl Clone for BottomNavigationProps {
    fn clone(&self) -> Self {
        Self {
            labels: self.labels.clone(),
            selected: self.selected,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for BottomNavigationProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BottomNavigationProps")
            .field("labels", &self.labels)
            .field("selected", &self.selected)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(usize)"))
            .finish()
    }
}

impl Default for BottomNavigationProps {
    fn default() -> Self {
        Self {
            labels: Vec::new(),
            selected: 0,
            on_change: None,
        }
    }
}
