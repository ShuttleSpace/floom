pub struct OverlayProps {
    /// Scrim opacity, 0.0 (transparent) .. 1.0 (opaque).
    pub opacity: f32,
    /// Fired when the backdrop (scrim) is clicked — typically wired to dismiss.
    pub on_click: Option<Box<dyn Fn()>>,
}
impl Clone for OverlayProps {
    fn clone(&self) -> Self {
        Self {
            opacity: self.opacity,
            on_click: None,
        }
    }
}
impl std::fmt::Debug for OverlayProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayProps")
            .field("opacity", &self.opacity)
            .field("on_click", &self.on_click.as_ref().map(|_| "Fn()"))
            .finish()
    }
}
impl Default for OverlayProps {
    fn default() -> Self {
        Self {
            opacity: 0.5,
            on_click: None,
        }
    }
}
