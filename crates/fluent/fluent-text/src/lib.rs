use floem_core_theme::props::text::TextProps;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentTextStyle {
    pub design_system: &'static str,
}

pub struct FluentText {
    props: TextProps,
}

impl FluentText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..Default::default()
            },
        }
    }
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.props.max_lines = Some(max_lines);
        self
    }
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.props.is_selectable = selectable;
        self
    }
    pub fn props(&self) -> &TextProps {
        &self.props
    }
    pub fn resolved_style(&self) -> FluentTextStyle {
        FluentTextStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
