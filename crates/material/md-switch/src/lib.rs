use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::switch::SwitchProps;
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSwitchStyle {
    pub track_color: Color,
    pub thumb_color: Color,
    pub width: f32,
    pub height: f32,
    pub thumb_size: f32,
    pub thumb_offset: f32,
}

pub struct MdSwitch {
    props: SwitchProps,
    checked: RwSignal<bool>,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdSwitch {
    pub fn new() -> Self {
        let props = SwitchProps::default();
        Self {
            checked: RwSignal::new(props.checked),
            theme: theme_signal(),
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

    pub fn props(&self) -> &SwitchProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdSwitchStyle {
        let theme = self.theme.get();
        let ComponentTokens::Switch(tokens) = theme.component(ComponentId::Switch) else {
            unreachable!("ComponentId::Switch must resolve to SwitchTokens")
        };
        let checked = self.is_checked();
        MdSwitchStyle {
            track_color: if checked {
                theme.colors().primary
            } else {
                theme.colors().surface_variant
            },
            thumb_color: if checked {
                theme.colors().on_primary
            } else {
                theme.colors().outline
            },
            width: tokens.width,
            height: tokens.height,
            thumb_size: tokens.thumb_size,
            thumb_offset: if checked {
                tokens.width - tokens.height
            } else {
                0.0
            },
        }
    }
}

impl Default for MdSwitch {
    fn default() -> Self {
        Self::new()
    }
}
