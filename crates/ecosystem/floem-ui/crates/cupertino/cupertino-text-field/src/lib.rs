use std::cell::Cell;
use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate, create_effect};
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::text_field::TextFieldProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTextFieldStyle {
    pub design_system: &'static str,
    pub border_color: Color,
    pub text_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
}

pub struct CupertinoTextField {
    props: TextFieldProps,
    value: RwSignal<String>,
    focused: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTextField {
    pub fn new() -> Self {
        let props = TextFieldProps::default();
        Self {
            value: RwSignal::new(props.value.clone()),
            focused: RwSignal::new(false),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
        }
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }
    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        let value = self.clamp_value(value.into());
        self.value.set(value.clone());
        self.props.value = value;
        self
    }
    pub fn password(mut self, is_password: bool) -> Self {
        self.props.is_password = is_password;
        self
    }
    pub fn max_length(mut self, max_length: usize) -> Self {
        self.props.max_length = Some(max_length);
        let value = self.clamp_value(self.props.value.clone());
        self.value.set(value.clone());
        self.props.value = value;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &TextFieldProps {
        &self.props
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        let value = self.clamp_value(value.into());
        self.value.set(value.clone());
        self.props.value = value.clone();
        if let Some(callback) = &self.props.on_change {
            callback(&value);
        }
    }

    pub fn value(&self) -> String {
        self.value.get()
    }

    pub fn set_focused(&self, focused: bool) {
        self.focused.set(focused);
    }

    pub fn is_focused(&self) -> bool {
        self.focused.get()
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoTextFieldStyle {
        let theme = self.theme.get();
        let ComponentTokens::TextField(tokens) = theme.component(ComponentId::TextField) else {
            unreachable!("ComponentId::TextField must resolve to TextFieldTokens")
        };
        CupertinoTextFieldStyle {
            design_system: DESIGN_SYSTEM,
            border_color: if self.props.disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else if self.is_focused() {
                theme.colors().primary
            } else {
                theme.colors().outline
            },
            text_color: if self.props.disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else {
                theme.colors().on_surface
            },
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
        }
    }

    fn clamp_value(&self, value: String) -> String {
        match self.props.max_length {
            Some(max_length) => value.chars().take(max_length).collect(),
            None => value,
        }
    }

    /// Build a renderable, interactive Cupertino rounded text field.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value;
        let focused = self.focused;
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let on_change = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::TextField) {
            ComponentTokens::TextField(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;

        // Fire the consumer callback whenever the buffer changes (after init).
        let initialized = Rc::new(Cell::new(false));
        create_effect(move |_| {
            let v = value.get();
            if initialized.get() {
                if let Some(cb) = on_change.as_ref() {
                    cb(&v);
                }
            } else {
                initialized.set(true);
            }
        });

        let input = TextInput::new(value)
            .placeholder(&placeholder)
            .style(move |s| {
                let t = theme.get();
                let c = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().on_surface
                };
                s.color(c.to_peniko())
                    .font_size(t.typography().body.medium.font_size)
                    .width_pct(100.0)
            })
            .on_event(listener::FocusGained, move |_, _| {
                focused.set(true);
                EventPropagation::Continue
            })
            .on_event(listener::FocusLost, move |_, _| {
                focused.set(false);
                EventPropagation::Continue
            })
            .style(move |s| s.set_disabled(disabled));

        Container::new(input)
            .style(move |s| {
                let t = theme.get();
                let border = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else if focused.get() {
                    t.colors().primary
                } else {
                    t.colors().outline
                };
                let bg = if disabled {
                    with_alpha(t.colors().on_surface, 4)
                } else {
                    t.colors().surface
                };
                s.height(height)
                    .padding_horiz(padding_horizontal)
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .items_center()
                    .width_pct(100.0)
                    .cursor(CursorStyle::Text)
                    .hover(|s| s.border_color(with_alpha(t.colors().primary, 120).to_peniko()))
            })
            .style(move |s| s.set_disabled(disabled))
    }
}

impl Default for CupertinoTextField {
    fn default() -> Self {
        Self::new()
    }
}
