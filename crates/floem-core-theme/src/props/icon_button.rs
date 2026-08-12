#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconButtonVariant {
    /// Plain icon on a transparent background.
    #[default]
    Text,
    /// Filled circular / rounded button with a solid background.
    Contained,
    /// Surface button with a thin outlined border.
    Outlined,
}

/// Shared, design-system-agnostic props for the `IconButton` component.
pub struct IconButtonProps {
    /// SVG markup (full `<svg>…</svg>` string) rendered as the button's icon.
    pub icon: String,
    /// Explicit icon/button size in px; falls back to token size when `None`.
    pub size: Option<f32>,
    /// Visual variant (text / contained / outlined).
    pub variant: IconButtonVariant,
    /// Whether the button is non-interactive.
    pub disabled: bool,
    /// Called (without arguments) when the button is activated.
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Clone for IconButtonProps {
    fn clone(&self) -> Self {
        Self {
            icon: self.icon.clone(),
            size: self.size,
            variant: self.variant,
            disabled: self.disabled,
            on_click: None,
        }
    }
}

impl std::fmt::Debug for IconButtonProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconButtonProps")
            .field("icon", &self.icon)
            .field("size", &self.size)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("on_click", &self.on_click.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for IconButtonProps {
    fn default() -> Self {
        Self {
            icon: String::new(),
            size: None,
            variant: IconButtonVariant::Text,
            disabled: false,
            on_click: None,
        }
    }
}
