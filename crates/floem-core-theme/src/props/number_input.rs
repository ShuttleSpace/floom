/// Shared, design-system-agnostic props for the `NumberInput` component.
pub struct NumberInputProps {
    /// Initial numeric value.
    pub value: i64,
    /// Optional inclusive lower bound (clamped).
    pub min: Option<i64>,
    /// Optional inclusive upper bound (clamped).
    pub max: Option<i64>,
    /// Increment/decrement step.
    pub step: i64,
    /// Optional floating label.
    pub label: Option<String>,
    /// Disables interaction when true.
    pub disabled: bool,
    /// Called with the new value whenever it changes.
    pub on_change: Option<Box<dyn Fn(i64)>>,
}

impl Clone for NumberInputProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            min: self.min,
            max: self.max,
            step: self.step,
            label: self.label.clone(),
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for NumberInputProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NumberInputProps")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("step", &self.step)
            .field("label", &self.label)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(i64)"))
            .finish()
    }
}

impl Default for NumberInputProps {
    fn default() -> Self {
        Self {
            value: 0,
            min: None,
            max: None,
            step: 1,
            label: None,
            disabled: false,
            on_change: None,
        }
    }
}
