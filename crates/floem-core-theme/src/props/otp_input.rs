pub struct OtpInputProps {
    /// Number of single-character boxes.
    pub length: usize,
    /// When true, entered characters are masked (shown as a dot).
    pub masked: bool,
    /// Whether the inputs are interactive.
    pub disabled: bool,
    /// Called with the full code string whenever any box changes.
    pub on_change: Option<Box<dyn Fn(String)>>,
}

impl Clone for OtpInputProps {
    fn clone(&self) -> Self {
        Self {
            length: self.length,
            masked: self.masked,
            disabled: self.disabled,
            on_change: None,
        }
    }
}

impl std::fmt::Debug for OtpInputProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OtpInputProps")
            .field("length", &self.length)
            .field("masked", &self.masked)
            .field("disabled", &self.disabled)
            .field("on_change", &self.on_change.as_ref().map(|_| "Fn(String)"))
            .finish()
    }
}

impl Default for OtpInputProps {
    fn default() -> Self {
        Self {
            length: 6,
            masked: false,
            disabled: false,
            on_change: None,
        }
    }
}
