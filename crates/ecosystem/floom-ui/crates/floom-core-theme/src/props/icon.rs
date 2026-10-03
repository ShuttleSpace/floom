#[derive(Debug, Clone, PartialEq)]
pub struct IconProps {
    pub name: String,
    pub size: Option<f32>,
    pub label: Option<String>,
    pub decorative: bool,
}

impl Default for IconProps {
    fn default() -> Self {
        Self {
            name: String::new(),
            size: None,
            label: None,
            decorative: true,
        }
    }
}
