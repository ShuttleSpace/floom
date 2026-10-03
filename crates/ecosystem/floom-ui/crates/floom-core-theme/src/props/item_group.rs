#[derive(Debug, Clone)]
pub struct ItemGroupProps {
    pub multiple: bool,
    pub mandatory: bool,
}

impl Default for ItemGroupProps {
    fn default() -> Self {
        Self {
            multiple: false,
            mandatory: false,
        }
    }
}
