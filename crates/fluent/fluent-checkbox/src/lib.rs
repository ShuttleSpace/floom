use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::checkbox::CheckboxProps;
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentCheckboxStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub mark_color: Color,
    pub outline_color: Color,
    pub size: f32,
    pub border_radius: f32,
}

pub struct FluentCheckbox {
    props: CheckboxProps,
    checked: RwSignal<bool>,
    theme: RwSignal<FluentTheme>,
}

impl FluentCheckbox {
    pub fn new(label: impl Into<String>) -> Self {
        let props = CheckboxProps {
            label: label.into(),
            ..Default::default()
        };
        Self {
            checked: RwSignal::new(props.checked),
            theme: RwSignal::new(FluentTheme::new()),
            props,
        }
    }
    pub fn checked(mut self, checked: bool) -> Self {
        self.props.checked = checked;
        self.checked.set(checked);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &CheckboxProps {
        &self.props
    }
    pub fn toggle(&mut self) {
        if self.props.disabled {
            return;
        }
        let next = !self.checked.get();
        self.checked.set(next);
        self.props.checked = next;
        if let Some(callback) = &self.props.on_change {
            callback(next);
        }
    }
    pub fn is_checked(&self) -> bool {
        self.checked.get()
    }
    pub fn theme(&self) -> RwSignal<FluentTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: FluentTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentCheckboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Checkbox(tokens) = theme.component(ComponentId::Checkbox) else {
            unreachable!("ComponentId::Checkbox must resolve to CheckboxTokens")
        };
        let checked = self.is_checked();
        FluentCheckboxStyle {
            design_system: DESIGN_SYSTEM,
            container_color: if checked {
                theme.colors().primary
            } else {
                Color::rgba(0, 0, 0, 0)
            },
            mark_color: if checked {
                theme.colors().on_primary
            } else {
                Color::rgba(0, 0, 0, 0)
            },
            outline_color: if self.props.disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else {
                theme.colors().outline
            },
            size: tokens.size,
            border_radius: tokens.border_radius,
        }
    }
}
