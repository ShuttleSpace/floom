pub struct RangeSliderProps {
    /// Lower bound of the selected range.
    pub low: f32,
    /// Upper bound of the selected range.
    pub high: f32,
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
    /// Step increment.
    pub step: f32,
    /// Whether the slider is interactive.
    pub disabled: bool,
    /// Called with the new (low, high) pair whenever either thumb is dragged.
    pub on_change: Option<Box<dyn Fn((f32, f32))>>,
}

impl Clone for RangeSliderProps {
    fn clone(&self) -> Self {
        Self {
            low: self.low,
            high: self.high,
            min: self.min,
            max: self.max,
            step: self.step,
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for RangeSliderProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RangeSliderProps")
            .field("low", &self.low)
            .field("high", &self.high)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("step", &self.step)
            .field("disabled", &self.disabled)
            .field(
                "on_change",
                &self.on_change.as_ref().map(|_| "Fn((f32, f32))"),
            )
            .finish()
    }
}

impl Default for RangeSliderProps {
    fn default() -> Self {
        Self {
            low: 25.0,
            high: 75.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            disabled: false,
            on_change: None,
        }
    }
}
