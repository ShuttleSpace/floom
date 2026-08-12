use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::radio::RadioProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdRadioStyle {
    pub selected_color: Color,
    pub unselected_color: Color,
    pub size: f32,
    pub border_width: f32,
    pub label_font_size: f32,
}

pub struct MdRadio {
    props: RadioProps,
    checked: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl MdRadio {
    pub fn new(label: impl Into<String>) -> Self {
        let props = RadioProps {
            label: label.into(),
            ..Default::default()
        };
        Self {
            checked: RwSignal::new(props.checked),
            theme: theme_signal(),
            props,
        }
    }

    /// Grouping value (informational; the host wires mutual exclusion).
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        self
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

    pub fn is_checked(&self) -> bool {
        self.checked.get()
    }

    pub fn props(&self) -> &RadioProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdRadioStyle {
        let theme = self.theme.get();
        let ComponentTokens::Radio(tokens) = theme.component(ComponentId::Radio) else {
            unreachable!("ComponentId::Radio must resolve to RadioTokens")
        };
        let checked = self.is_checked();
        MdRadioStyle {
            selected_color: if checked {
                theme.colors().primary
            } else {
                theme.colors().outline
            },
            unselected_color: theme.colors().outline,
            size: tokens.size,
            border_width: tokens.border_width,
            label_font_size: tokens.label_font_size,
        }
    }

    /// Build a renderable, interactive Material 3 radio button.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let checked = self.checked;
        let disabled = self.props.disabled;
        let on_change = Rc::new(self.props.on_change);
        let label_text = self.props.label.clone();

        let tokens = match theme.get().component(ComponentId::Radio) {
            ComponentTokens::Radio(t) => t,
            _ => unreachable!(),
        };
        let size = tokens.size;
        let bw = tokens.border_width;
        let label_fs = tokens.label_font_size;

        let dot = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            s.size(size * 0.5, size * 0.5)
                .border_radius(9999.0)
                .background(t.colors().primary.to_peniko())
                .apply_if(!checked.get(), |s| s.hide())
        });

        let circle = Container::new(dot).style(move |s| {
            let t = theme.get();
            let c = if checked.get() {
                t.colors().primary
            } else {
                t.colors().outline
            };
            let c = if disabled {
                with_alpha(t.colors().on_surface, 38)
            } else {
                c
            };
            s.size(size, size)
                .border_radius(9999.0)
                .border(bw)
                .border_color(c.to_peniko())
                .items_center()
                .justify_center()
        });

        let label_view = text(label_text).style(move |s| {
            let t = theme.get();
            let c = if disabled {
                with_alpha(t.colors().on_surface, 38)
            } else {
                t.colors().on_surface
            };
            s.color(c.to_peniko()).font_size(label_fs)
        });

        let toggle = Rc::new(move || {
            if !disabled {
                let next = !checked.get();
                checked.set(next);
                if let Some(cb) = on_change.as_ref() {
                    cb(next);
                }
            }
        });

        h_stack((circle, label_view))
            .style(|s| s.items_center().gap(8.0).cursor(CursorStyle::Pointer))
            .on_event(listener::Click, {
                let toggle = toggle.clone();
                move |_, _| {
                    toggle();
                    EventPropagation::Continue
                }
            })
            .on_event(listener::KeyDown, move |_, e: &KeyboardEvent| {
                if e.key == Key::Named(NamedKey::Enter) || e.key == Key::Character(" ".to_string()) {
                    toggle();
                }
                EventPropagation::Continue
            })
            .style(move |s| s.set_disabled(disabled).keyboard_navigable())
    }
}

impl Default for MdRadio {
    fn default() -> Self {
        Self::new(String::new())
    }
}
