/// A simple calendar date (no timezone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateValue {
    pub year: i32,
    pub month: u32, // 1..=12
    pub day: u32,   // 1..=31
}

impl Default for DateValue {
    fn default() -> Self {
        Self {
            year: 1970,
            month: 1,
            day: 1,
        }
    }
}

pub struct DatePickerProps {
    /// Currently selected date (`None` = placeholder).
    pub value: Option<DateValue>,
    /// Text shown when nothing is selected.
    pub placeholder: String,
    /// Whether the picker is interactive.
    pub disabled: bool,
    /// Called with the newly chosen date when a day is picked.
    pub on_change: Option<Box<dyn Fn(DateValue)>>,
}

impl Clone for DatePickerProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            placeholder: self.placeholder.clone(),
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for DatePickerProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatePickerProps")
            .field("value", &self.value)
            .field("placeholder", &self.placeholder)
            .field("disabled", &self.disabled)
            .field(
                "on_change",
                &self.on_change.as_ref().map(|_| "Fn(DateValue)"),
            )
            .finish()
    }
}

impl Default for DatePickerProps {
    fn default() -> Self {
        Self {
            value: None,
            placeholder: "Select date".to_string(),
            disabled: false,
            on_change: None,
        }
    }
}
