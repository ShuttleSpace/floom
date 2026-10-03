pub struct MenuItem {
    pub label: String,
    pub on_select: Option<Box<dyn Fn()>>,
}
impl Clone for MenuItem {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            on_select: None,
        }
    }
}
impl std::fmt::Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem")
            .field("label", &self.label)
            .field("on_select", &self.on_select.as_ref().map(|_| "Fn()"))
            .finish()
    }
}
pub struct MenuProps {
    pub items: Vec<MenuItem>,
    pub open: bool,
}
impl Clone for MenuProps {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
            open: self.open,
        }
    }
}
impl std::fmt::Debug for MenuProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuProps")
            .field("items", &self.items)
            .field("open", &self.open)
            .finish()
    }
}
impl Default for MenuProps {
    fn default() -> Self {
        Self {
            items: vec![],
            open: false,
        }
    }
}
