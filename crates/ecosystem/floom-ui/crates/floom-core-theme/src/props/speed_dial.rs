#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeedDialDirection {
    /// Actions stack above the FAB.
    #[default]
    Up,
    /// Actions stack below the FAB.
    Down,
    /// Actions stack to the left of the FAB.
    Left,
    /// Actions stack to the right of the FAB.
    Right,
}

/// Shared, design-system-agnostic props for the `SpeedDial` component.
pub struct SpeedDialProps {
    /// Direction the action buttons fan out.
    pub direction: SpeedDialDirection,
    /// Initial open state of the dial.
    pub open: bool,
    /// SVG markup for the FAB's icon (defaults to a plus when empty).
    pub fab_icon: String,
    /// Whether the whole dial is non-interactive.
    pub disabled: bool,
    /// Called with the action index when an action button is activated.
    pub on_action: Option<Box<dyn Fn(usize)>>,
}

impl Clone for SpeedDialProps {
    fn clone(&self) -> Self {
        Self {
            direction: self.direction,
            open: self.open,
            fab_icon: self.fab_icon.clone(),
            disabled: self.disabled,
            on_action: None,
        }
    }
}

impl std::fmt::Debug for SpeedDialProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeedDialProps")
            .field("direction", &self.direction)
            .field("open", &self.open)
            .field("fab_icon", &self.fab_icon)
            .field("disabled", &self.disabled)
            .field("on_action", &self.on_action.as_ref().map(|_| "Fn(usize)"))
            .finish()
    }
}

impl Default for SpeedDialProps {
    fn default() -> Self {
        Self {
            direction: SpeedDialDirection::Up,
            open: false,
            fab_icon: String::new(),
            disabled: false,
            on_action: None,
        }
    }
}
