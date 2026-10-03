pub struct ExpandTransitionProps {}

impl Clone for ExpandTransitionProps {
    fn clone(&self) -> Self {
        Self {}
    }
}

impl std::fmt::Debug for ExpandTransitionProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExpandTransitionProps").finish()
    }
}

impl Default for ExpandTransitionProps {
    fn default() -> Self {
        Self {}
    }
}
