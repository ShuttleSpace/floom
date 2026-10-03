use std::rc::Rc;

use floem::prelude::*;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::style::CursorStyle;
use floem::event::EventPropagation;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::switch::SwitchProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
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
    theme: RwSignal<MultiTheme>,
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
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

    /// Build a renderable, interactive Material 3 switch.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let checked = self.checked;
        let disabled = self.props.disabled;
        let on_change = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Switch) {
            ComponentTokens::Switch(t) => t,
            _ => unreachable!(),
        };
        let width = tokens.width;
        let height = tokens.height;
        let thumb = tokens.thumb_size;
        let radius = height / 2.0;

        let thumb_view = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let c = if disabled {
                with_alpha(t.colors().outline, 38)
            } else if checked.get() {
                t.colors().on_primary
            } else {
                t.colors().outline
            };
            s.size(thumb, thumb)
                .border_radius(thumb / 2.0)
                .background(c.to_peniko())
                .box_shadow_blur(1.0)
                .box_shadow_spread(0.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 40).to_peniko())
        });

        let track = Container::new(thumb_view)
            .style(move |s| {
                let t = theme.get();
                let c = if disabled {
                    with_alpha(t.colors().on_surface, 12)
                } else if checked.get() {
                    t.colors().primary
                } else {
                    t.colors().surface_variant
                };
                s.width(width)
                    .height(height)
                    .border_radius(radius)
                    .background(c.to_peniko())
                    .items_center()
                    .apply_if(checked.get(), |s| s.justify_end())
                    .apply_if(!checked.get(), |s| s.justify_start())
                    .padding(2.0)
                    .cursor(CursorStyle::Pointer)
            })
            .style(move |s| s.set_disabled(disabled).keyboard_navigable());

        let toggle = Rc::new(move || {
            if !disabled {
                let next = !checked.get();
                checked.set(next);
                if let Some(cb) = on_change.as_ref() {
                    cb(next);
                }
            }
        });

        track
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
    }
}

impl Default for MdSwitch {
    fn default() -> Self {
        Self::new()
    }
}
