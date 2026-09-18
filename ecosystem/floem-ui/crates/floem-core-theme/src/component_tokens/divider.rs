#[derive(Debug, Clone, PartialEq)]
pub struct DividerTokens {
    pub thickness: f32,
}

impl Default for DividerTokens {
    fn default() -> Self {
        Self { thickness: 1.0 }
    }
}
