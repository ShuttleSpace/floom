use std::cell::Cell;
use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::style::CursorStyle;
use floom::reactive::create_effect;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::textarea::TextareaProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentTextareaStyle {
    pub design_system: &'static str,
    pub border_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
}

pub struct FluentTextarea {
    props: TextareaProps,
    value: RwSignal<String>,
    focused: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl FluentTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            value: RwSignal::new(props.value.clone()),
            focused: RwSignal::new(false),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            props,
        }
    }
    pub fn value(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.value.set(value.clone());
        self.props.value = value;
        self
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn rows(mut self, rows: usize) -> Self {
        self.props.rows = rows.max(1);
        self
    }
    pub fn on_change(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &TextareaProps {
        &self.props
    }
    pub fn set_value(&mut self, value: impl Into<String>) {
        let value = value.into();
        self.value.set(value.clone());
        self.props.value = value.clone();
        if let Some(callback) = &self.props.on_change {
            callback(value);
        }
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentTextareaStyle {
        let theme = self.theme.get();
        let ComponentTokens::Textarea(tokens) = theme.component(ComponentId::Textarea) else {
            unreachable!("ComponentId::Textarea must resolve to TextareaTokens")
        };
        FluentTextareaStyle {
            design_system: DESIGN_SYSTEM,
            border_color: if self.props.disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else if self.focused.get() {
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
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value;
        let focused = self.focused;
        let disabled = self.props.disabled;
        let rows = self.props.rows;
        let placeholder = self.props.placeholder.clone();
        let on_change = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Textarea) {
            ComponentTokens::Textarea(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let max_width = tokens.max_width;

        let line_height = font_size + 6.0;
        let total_min = tokens.min_height + (rows.saturating_sub(1) as f32) * line_height;

        let initialized = Rc::new(Cell::new(false));
        create_effect(move |_| {
            let v = value.get();
            if initialized.get() {
                if let Some(cb) = on_change.as_ref() {
                    cb(v);
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
                    .font_size(font_size)
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
                s.min_height(total_min)
                    .max_width(max_width)
                    .padding(padding)
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .items_start()
                    .width_pct(100.0)
                    .cursor(CursorStyle::Text)
                    .focus(|s| s.outline(2.0).outline_color(t.colors().primary.to_peniko()))
            })
            .style(move |s| s.set_disabled(disabled))
    }
}

impl Default for FluentTextarea {
    fn default() -> Self {
        Self::new()
    }
}
