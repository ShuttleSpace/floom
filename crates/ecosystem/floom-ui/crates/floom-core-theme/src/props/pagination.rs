pub struct PaginationProps {
    /// Total number of pages (>= 1).
    pub total: usize,
    /// Current page (1-based, clamped to 1..=total).
    pub current: usize,
    /// Called with the newly selected page (1-based) when a control is tapped.
    pub on_change: Option<Box<dyn Fn(usize)>>,
}

impl Clone for PaginationProps {
    fn clone(&self) -> Self {
        Self {
            total: self.total,
            current: self.current,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for PaginationProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaginationProps")
            .field("total", &self.total)
            .field("current", &self.current)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(usize)"))
            .finish()
    }
}

impl Default for PaginationProps {
    fn default() -> Self {
        Self {
            total: 1,
            current: 1,
            on_change: None,
        }
    }
}
