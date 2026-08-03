use floem_core_theme::props::card::CardProps;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentCardStyle {
    pub design_system: &'static str,
}

pub struct FluentCard {
    props: CardProps,
}

impl FluentCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
        }
    }
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }
    pub fn props(&self) -> &CardProps {
        &self.props
    }
    pub fn resolved_style(&self) -> FluentCardStyle {
        FluentCardStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for FluentCard {
    fn default() -> Self {
        Self::new()
    }
}
