/// Semantic variant of a banner, controlling its accent colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BannerVariant {
    Info,
    Warning,
    Error,
    Success,
}

/// Neutral props shared by every design system's `Banner`.
pub struct BannerProps {
    pub variant: BannerVariant,
    /// Main banner message.
    pub message: String,
    /// Optional leading material/cupertino/fluent icon name.
    pub icon: Option<String>,
    /// Optional action label rendered as a text button on the right.
    pub action_label: Option<String>,
    /// Optional action callback fired when the action button is clicked.
    pub on_action: Option<Box<dyn Fn()>>,
    /// Restrict the banner to a single line (truncate the message).
    pub single_line: bool,
}

impl Default for BannerProps {
    fn default() -> Self {
        Self {
            variant: BannerVariant::Info,
            message: String::new(),
            icon: None,
            action_label: None,
            on_action: None,
            single_line: false,
        }
    }
}

impl std::fmt::Debug for BannerProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BannerProps")
            .field("variant", &self.variant)
            .field("message", &self.message)
            .field("icon", &self.icon)
            .field("action_label", &self.action_label)
            .field("single_line", &self.single_line)
            .finish()
    }
}

impl Clone for BannerProps {
    fn clone(&self) -> Self {
        // Callbacks are intentionally dropped on clone.
        Self {
            variant: self.variant,
            message: self.message.clone(),
            icon: self.icon.clone(),
            action_label: self.action_label.clone(),
            on_action: None,
            single_line: self.single_line,
        }
    }
}
