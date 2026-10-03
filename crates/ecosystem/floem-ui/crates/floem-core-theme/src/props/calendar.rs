/// A simple calendar date (no timezone).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u32, // 1..=12
    pub day: u32,   // 1..=31
}

impl Default for CalendarDate {
    fn default() -> Self {
        Self {
            year: 2026,
            month: 8,
            day: 1,
        }
    }
}

/// Neutral, design-system-agnostic props for the calendar.
pub struct CalendarProps {
    /// Currently selected date.
    pub value: CalendarDate,
    /// Called with the newly picked date when a day is clicked.
    pub on_change: Option<Box<dyn Fn(CalendarDate)>>,
    /// Render a leading week-number column.
    pub show_week_numbers: bool,
}

impl Clone for CalendarProps {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            on_change: None,
            show_week_numbers: self.show_week_numbers,
        }
    }
}

impl std::fmt::Debug for CalendarProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CalendarProps")
            .field("value", &self.value)
            .field("show_week_numbers", &self.show_week_numbers)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(CalendarDate)"))
            .finish()
    }
}

impl Default for CalendarProps {
    fn default() -> Self {
        Self {
            value: CalendarDate::default(),
            on_change: None,
            show_week_numbers: false,
        }
    }
}
