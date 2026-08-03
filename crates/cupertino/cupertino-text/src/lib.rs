use floem_core_theme::props::text::TextProps;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTextStyle {
    pub design_system: &'static str,
}

pub struct CupertinoText {
    props: TextProps,
}

impl CupertinoText {
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
    pub fn resolved_style(&self) -> CupertinoTextStyle {
        CupertinoTextStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
