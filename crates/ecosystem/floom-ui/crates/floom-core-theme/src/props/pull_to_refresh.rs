/// Shared, design-system-agnostic props for the `PullToRefresh` component.
pub struct PullToRefreshProps {
    /// Whether a refresh is currently in flight (shows the spinner).
    pub loading: bool,
    /// Drag distance (dp) that triggers a refresh.
    pub threshold: f32,
    /// Invoked when the pull passes `threshold`.
    pub on_refresh: Option<Box<dyn Fn()>>,
}

impl Clone for PullToRefreshProps {
    fn clone(&self) -> Self {
        Self {
            loading: self.loading,
            threshold: self.threshold,
            on_refresh: None,
        }
    }
}

impl std::fmt::Debug for PullToRefreshProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PullToRefreshProps")
            .field("loading", &self.loading)
            .field("threshold", &self.threshold)
            .field("on_refresh", &self.on_refresh.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for PullToRefreshProps {
    fn default() -> Self {
        Self {
            loading: false,
            threshold: 80.0,
            on_refresh: None,
        }
    }
}
