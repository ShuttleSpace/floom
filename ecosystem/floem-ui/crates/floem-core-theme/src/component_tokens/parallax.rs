#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParallaxTokens {
    pub factor: f32,
}

impl Default for ParallaxTokens {
    fn default() -> Self {
        Self { factor: 0.3 }
    }
}
