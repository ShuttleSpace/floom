pub struct HoverProps {
    pub on_hover_change: Option<Box<dyn Fn(bool)>>,
}

impl Clone for HoverProps {
    fn clone(&self) -> Self {
        Self {
            on_hover_change: None,
        }
    }
}

impl std::fmt::Debug for HoverProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HoverProps")
            .field(
                "on_hover_change",
                &self.on_hover_change.as_ref().map(|_| "Fn(bool)"),
            )
            .finish()
    }
}

impl Default for HoverProps {
    fn default() -> Self {
        Self {
            on_hover_change: None,
        }
    }
}
