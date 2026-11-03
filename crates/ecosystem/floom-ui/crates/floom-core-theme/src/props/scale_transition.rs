pub struct ScaleTransitionProps {}

impl Clone for ScaleTransitionProps {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl std::fmt::Debug for ScaleTransitionProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScaleTransitionProps").finish()
    }
}

impl Default for ScaleTransitionProps {
    fn default() -> Self {
        Self {}
    }
}
