pub struct RatingProps {
    /// Current rating value (may be fractional; clamped to 0..=max).
    pub value: f32,
    /// Number of star slots.
    pub max: usize,
    /// When true, taps do not change the value.
    pub readonly: bool,
    /// Called with the new value when a star is tapped (ignored when readonly).
    pub on_change: Option<Box<dyn Fn(f32)>>,
}

impl Clone for RatingProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            max: self.max,
            readonly: self.readonly,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for RatingProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RatingProps")
            .field("value", &self.value)
            .field("max", &self.max)
            .field("readonly", &self.readonly)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(f32)"))
            .finish()
    }
}

impl Default for RatingProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            max: 5,
            readonly: false,
            on_change: None,
        }
    }
}
