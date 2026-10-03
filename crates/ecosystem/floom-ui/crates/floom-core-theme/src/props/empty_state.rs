pub struct EmptyStateProps {
    /// Optional icon glyph/text rendered above the title (empty = no icon).
    pub icon: String,
    /// Headline text.
    pub title: String,
    /// Supporting description text.
    pub description: String,
}

impl Clone for EmptyStateProps {
    fn clone(&self) -> Self {
        Self {
            icon: self.icon.clone(),
            title: self.title.clone(),
            description: self.description.clone(),
        }
    }
}

impl std::fmt::Debug for EmptyStateProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmptyStateProps")
            .field("icon", &self.icon)
            .field("title", &self.title)
            .field("description", &self.description)
            .finish()
    }
}

impl Default for EmptyStateProps {
    fn default() -> Self {
        Self {
            icon: String::new(),
            title: "No data".to_string(),
            description: String::new(),
        }
    }
}
