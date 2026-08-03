use floem_core_theme::props::card::CardProps;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoCardStyle {
    pub design_system: &'static str,
}

pub struct CupertinoCard {
    props: CardProps,
}

impl CupertinoCard {
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
    pub fn resolved_style(&self) -> CupertinoCardStyle {
        CupertinoCardStyle {
            design_system: DESIGN_SYSTEM,
        }
    }
}
impl Default for CupertinoCard {
    fn default() -> Self {
        Self::new()
    }
}
