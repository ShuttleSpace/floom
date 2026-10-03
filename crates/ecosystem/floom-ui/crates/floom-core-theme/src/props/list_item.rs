pub struct ListItemProps {
    /// Primary line of text.
    pub title: String,
    /// Optional secondary line of text.
    pub subtitle: Option<String>,
    /// Whether the item ignores clicks.
    pub disabled: bool,
    /// Navigation styling (extra trailing affordance space + hover affordance).
    pub nav: bool,
    /// Clicked callback (ignored when `disabled`).
    pub on_click: Option<Box<dyn Fn()>>,
}

impl Clone for ListItemProps {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            subtitle: self.subtitle.clone(),
            disabled: self.disabled,
            nav: self.nav,
            on_click: None,
        }
    }
}

impl std::fmt::Debug for ListItemProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListItemProps")
            .field("title", &self.title)
            .field("subtitle", &self.subtitle)
            .field("disabled", &self.disabled)
            .field("nav", &self.nav)
            .field("on_click", &self.on_click.as_ref().map(|_| "Fn()"))
            .finish()
    }
}

impl Default for ListItemProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            subtitle: None,
            disabled: false,
            nav: false,
            on_click: None,
        }
    }
}
