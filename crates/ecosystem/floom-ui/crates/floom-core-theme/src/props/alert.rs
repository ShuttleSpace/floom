#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertVariant {
    Info,
    Success,
    Warning,
    Error,
}

pub struct AlertProps {
    pub variant: AlertVariant,
    pub title: String,
    pub message: String,
    pub dismissible: bool,
    pub on_dismiss: Option<Box<dyn Fn()>>,
}

impl Clone for AlertProps {
    fn clone(&self) -> Self {
        Self {
            variant: self.variant,
            title: self.title.clone(),
            message: self.message.clone(),
            dismissible: self.dismissible,
            on_dismiss: None,
        }
    }
}

impl std::fmt::Debug for AlertProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlertProps")
            .field("variant", &self.variant)
            .field("title", &self.title)
            .field("message", &self.message)
            .field("dismissible", &self.dismissible)
            .field("on_dismiss", &self.on_dismiss.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for AlertProps {
    fn default() -> Self {
        Self {
            variant: AlertVariant::Info,
            title: String::new(),
            message: String::new(),
            dismissible: false,
            on_dismiss: None,
        }
    }
}
