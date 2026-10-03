/// Neutral props shared by every design system's `Form`.
///
/// A `Form` is a grouping surface for fields with an optional validity-gated
/// submit button. Validity and error state are driven by the consumer through
/// the `valid_signal()` / `errors_signal()` accessors (mirrors the
/// `open_signal()` pattern used by `Dialog` / `Menu`).
#[derive(Debug, Clone)]
pub struct FormProps {
    /// Inner padding of the form surface in dp.
    pub pad: f32,
    /// Vertical gap between stacked children in dp.
    pub gap: f32,
    /// Label shown on the submit button.
    pub submit_label: String,
    /// Whether to render the submit button.
    pub show_submit: bool,
}

impl Default for FormProps {
    fn default() -> Self {
        Self {
            pad: 16.0,
            gap: 12.0,
            submit_label: "Submit".to_string(),
            show_submit: true,
        }
    }
}
