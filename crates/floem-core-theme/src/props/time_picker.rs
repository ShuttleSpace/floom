/// A wall-clock time of day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeValue {
    pub hour: u8,   // 0..=23
    pub minute: u8, // 0..=59
}

impl Default for TimeValue {
    fn default() -> Self {
        Self {
            hour: 0,
            minute: 0,
        }
    }
}

pub struct TimePickerProps {
    /// Currently selected time (`None` = placeholder).
    pub value: Option<TimeValue>,
    /// Text shown when nothing is selected.
    pub placeholder: String,
    /// Whether the picker is interactive.
    pub disabled: bool,
    /// Called with the newly chosen time when committed.
    pub on_change: Option<Box<dyn Fn(TimeValue)>>,
}

impl Clone for TimePickerProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            placeholder: self.placeholder.clone(),
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for TimePickerProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TimePickerProps")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(TimeValue)"))
            .finish()
    }
}

impl Default for TimePickerProps {
    fn default() -> Self {
        Self {
            value: None,
            placeholder: "Select time".to_string(),
            disabled: false,
            on_change: None,
        }
    }
}
