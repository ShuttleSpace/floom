use std::cell::Cell;
use std::rc::Rc;

use floem::prelude::*;
use floem::reactive::{create_effect, RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::textarea::TextareaProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdTextareaStyle {
    pub design_system: &'static str,
    pub text_color: Color,
    pub background: Color,
    pub border_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
    pub line_height: f32,
}

pub struct MdTextarea {
    props: TextareaProps,
    value: RwSignal<String>,
    theme: RwSignal<MultiTheme>,
}

impl MdTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            value: RwSignal::new(props.value.clone()),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MdTextareaStyle {
        let theme = self.theme.get();
        let ComponentTokens::Textarea(tokens) = theme.component(ComponentId::Textarea) else {
            unreachable!("ComponentId::Textarea must resolve to TextareaTokens")
        };
        let line_height = tokens.font_size + 6.0;
        let min_height = tokens.min_height + (self.props.rows.saturating_sub(1) as f32) * line_height;
        let disabled = self.props.disabled;
        MdTextareaStyle {
            design_system: "material",
            text_color: if disabled {
                Color {
                    a: 97,
                    ..theme.colors().on_surface
                }
            } else {
                theme.colors().on_surface
            },
            background: if disabled {
                with_alpha(theme.colors().on_surface, 4)
            } else {
                theme.colors().surface
            },
            border_color: if disabled {
                with_alpha(theme.colors().outline, 97)
            } else {
                theme.colors().outline
            },
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
            line_height,
        }
    }

    /// Build a renderable, interactive Material 3 text area (multi-line editor).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value;
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let on_change: Rc<Option<Box<dyn Fn(String)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Textarea) {
            ComponentTokens::Textarea(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let max_width = tokens.max_width;
        let line_height = font_size + 6.0;
        let min_height = tokens.min_height + (self.props.rows.saturating_sub(1) as f32) * line_height;

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
                    .font_size(t.typography().body.medium.font_size)
                    .width_pct(100.0)
                    .min_height(min_height)
                    .max_width(max_width)
            })
            .style(move |s| s.set_disabled(disabled));

        Container::new(input)
            .style(move |s| {
                let t = theme.get();
                let border = if disabled {
                    with_alpha(t.colors().outline, 97)
                } else {
                    t.colors().outline
                };
                let bg = if disabled {
                    with_alpha(t.colors().on_surface, 4)
                } else {
                    t.colors().surface
                };
                s.padding(padding)
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .items_start()
                    .width_pct(100.0)
                    .max_width(max_width)
                    .cursor(CursorStyle::Text)
                    .set_disabled(disabled)
            })
    }
}

impl Default for MdTextarea {
    fn default() -> Self {
        Self::new()
    }
}
