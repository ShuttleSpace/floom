use floom_legacy_reactive::RwSignal;

pub struct SliderProps {
    /// Current value.
    pub value: f32,
    /// Minimum value.
    pub min: f32,
    /// Maximum value.
    pub max: f32,
    /// Step increment.
    pub step: f32,
    /// Whether the slider is interactive.
    pub disabled: bool,
    /// Called with the new value whenever the thumb is dragged.
    pub on_change: Option<Box<dyn Fn(f32)>>,
    /// Optional external signal the slider reads from and writes to directly.
    /// When set, the slider never needs to be rebuilt to reflect value changes
    /// (eliminates the drag "flash-back" when the host rebuilds on every change).
    pub value_signal: Option<RwSignal<f32>>,
}

impl Clone for SliderProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
            disabled: self.disabled,
            on_change: None,
            value_signal: self.value_signal,
        }
    }
}

impl std::fmt::Debug for SliderProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SliderProps")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("step", &self.step)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(f32)"))
            .finish()
    }
}

impl Default for SliderProps {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            disabled: false,
            on_change: None,
            value_signal: None,
        }
    }
}
