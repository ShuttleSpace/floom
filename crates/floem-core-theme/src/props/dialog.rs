pub struct DialogProps {
    pub title: String,
    pub content: String,
    pub open: bool,
    pub on_close: Option<Box<dyn Fn()>>,
}
impl Clone for DialogProps {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            content: self.content.clone(),
            open: self.open,
            on_close: None,
        }
    }
}
impl std::fmt::Debug for DialogProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DialogProps")
            .field("title", &self.title)
            .field("content", &self.content)
            .field("open", &self.open)
            .field("on_close", &self.on_close.as_ref().map(|_| "Fn()"))
            .finish()
    }
}
impl Default for DialogProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            content: String::new(),
            open: false,
            on_close: None,
        }
    }
}
