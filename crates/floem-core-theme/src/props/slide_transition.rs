/// Which axis a `SlideTransition` travels along.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SlideDirection {
    /// Slide horizontally (translate on the X axis).
    #[default]
    X,
    /// Slide vertically (translate on the Y axis).
    Y,
}

pub struct SlideTransitionProps {
    pub direction: SlideDirection,
}

impl Clone for SlideTransitionProps {
    fn clone(&self) -> Self {
        Self {
            direction: self.direction,
        }
    }
}

impl std::fmt::Debug for SlideTransitionProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlideTransitionProps")
            .field("direction", &self.direction)
            .finish()
    }
}

impl Default for SlideTransitionProps {
    fn default() -> Self {
        Self {
            direction: SlideDirection::X,
        }
    }
}
