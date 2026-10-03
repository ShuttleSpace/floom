use std::rc::Rc;

use floom::prelude::*;
use floom::reactive::{RwSignal, SignalGet, create_effect};
use floom::style::CursorStyle;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::textarea::TextareaProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

/// Estimated line height used to derive min-height from `rows`.
const LINE_HEIGHT: f32 = 20.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTextareaStyle {
    pub design_system: &'static str,
    pub border_color: Color,
    pub text_color: Color,
    pub background: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
}

pub struct CupertinoTextarea {
    props: TextareaProps,
    value: RwSignal<String>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoTextarea {
    pub fn new() -> Self {
        let props = TextareaProps::default();
        Self {
            value: RwSignal::new(props.value.clone()),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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
    pub fn value_signal(&self) -> RwSignal<String> {
        self.value
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoTextareaStyle {
        let theme = self.theme.get();
        let ComponentTokens::Textarea(tokens) = theme.component(ComponentId::Textarea) else {
            unreachable!("ComponentId::Textarea must resolve to TextareaTokens")
        };
        CupertinoTextareaStyle {
            design_system: DESIGN_SYSTEM,
            border_color: if self.props.disabled {
                with_alpha(theme.colors().on_surface, 38)
            } else {
                theme.colors().outline
            },
            text_color: theme.colors().on_surface,
            background: if self.props.disabled {
                with_alpha(theme.colors().on_surface, 4)
            } else {
                theme.colors().surface_variant
            },
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
        }
    }

    /// Build a renderable, editable Cupertino textarea.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.value;
        let disabled = self.props.disabled;
        let placeholder = self.props.placeholder.clone();
        let rows = self.props.rows;
        let on_change = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Textarea) {
            ComponentTokens::Textarea(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = tokens.border_radius;
        let min_height = tokens.min_height + (rows - 1) as f32 * LINE_HEIGHT;
        let font_size = tokens.font_size;
        let max_width = tokens.max_width;

        // Fire the consumer callback whenever the buffer changes (after init).
        let initialized = Rc::new(std::cell::Cell::new(false));
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
                    with_alpha(t.colors().on_surface, 97)
                } else {
                    t.colors().on_surface
                };
                s.color(c.to_peniko())
                    .font_size(font_size)
                    .width_pct(100.0)
            })
            .style(move |s| s.set_disabled(disabled));

        Container::new(input)
            .style(move |s| {
                let t = theme.get();
                let border = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().outline
                };
                let bg = if disabled {
                    with_alpha(t.colors().on_surface, 4)
                } else {
                    t.colors().surface_variant
                };
                s.padding(padding)
                    .min_height(min_height)
                    .max_width(max_width)
                    .width_pct(100.0)
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .items_start()
                    .cursor(CursorStyle::Text)
                    .hover(move |s| {
                        let t = theme.get();
                        s.border_color(with_alpha(t.colors().primary, 140).to_peniko())
                    })
            })
            .style(move |s| s.set_disabled(disabled))
    }
}

impl Default for CupertinoTextarea {
    fn default() -> Self {
        Self::new()
    }
}
