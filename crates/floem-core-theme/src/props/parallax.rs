pub struct ParallaxProps {
    pub factor: f32,
}

impl Clone for ParallaxProps {
    fn clone(&self) -> Self {
        Self { factor: self.factor }
    }
}

impl std::fmt::Debug for ParallaxProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParallaxProps")
            .field("factor", &self.factor)
            .finish()
    }
}

impl Default for ParallaxProps {
    fn default() -> Self {
        Self { factor: 0.3 }
    }
}
