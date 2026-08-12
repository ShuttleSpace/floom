pub struct FadeTransitionProps {}

impl Clone for FadeTransitionProps {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl std::fmt::Debug for FadeTransitionProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FadeTransitionProps").finish()
    }
}

impl Default for FadeTransitionProps {
    fn default() -> Self {
        Self {}
    }
}
