#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonSize {
    Small,
    Medium,
    Large,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Text,
}

pub struct ButtonProps {
    pub label: String,
    pub size: ButtonSize,
    pub variant: ButtonVariant,
    pub disabled: bool,
    pub loading: bool,
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Clone for ButtonProps {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            size: self.size,
            variant: self.variant,
            disabled: self.disabled,
            loading: self.loading,
            on_click: None,
        }
    }
}

impl std::fmt::Debug for ButtonProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButtonProps")
            .field("label", &self.label)
            .field("size", &self.size)
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .field("loading", &self.loading)
            .field("on_click", &self.on_click.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            size: ButtonSize::Medium,
            variant: ButtonVariant::Primary,
            disabled: false,
            loading: false,
            on_click: None,
        }
    }
}
