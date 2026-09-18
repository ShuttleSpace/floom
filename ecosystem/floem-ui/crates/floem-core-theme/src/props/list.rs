use crate::props::menu::MenuItem;

pub struct ListProps {
    /// Clickable list items. Reuses `MenuItem` so the row contract matches `Menu`.
    pub items: Vec<MenuItem>,
}

impl Clone for ListProps {
    fn clone(&self) -> Self {
        Self {
            items: self.items.clone(),
        }
    }
}

impl std::fmt::Debug for ListProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListProps")
            .field("items", &self.items)
            .finish()
    }
}

impl Default for ListProps {
    fn default() -> Self {
        Self {
            items: Vec::new(),
        }
    }
}
