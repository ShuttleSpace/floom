pub struct InfiniteScrollerProps {
    pub threshold: f32,
    pub on_load_more: Option<Box<dyn Fn()>>,
}

impl Clone for InfiniteScrollerProps {
    fn clone(&self) -> Self {
        Self {
            threshold: self.threshold,
            on_load_more: None,
        }
    }
}

impl std::fmt::Debug for InfiniteScrollerProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InfiniteScrollerProps")
            .field("threshold", &self.threshold)
            .field("on_load_more", &self.on_load_more.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for InfiniteScrollerProps {
    fn default() -> Self {
        Self {
            threshold: 80.0,
            on_load_more: None,
        }
    }
}
