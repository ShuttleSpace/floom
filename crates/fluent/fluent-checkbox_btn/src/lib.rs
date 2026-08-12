use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, h_stack, label, text};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::checkbox_btn::CheckboxBtnProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

pub struct FluentCheckboxBtn {
    props: CheckboxBtnProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentCheckboxBtn {
    pub fn new() -> Self {
        Self {
            props: CheckboxBtnProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.props.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self
    }

    pub fn on_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let checked: RwSignal<bool> = RwSignal::new(self.props.checked);
        let disabled = self.props.disabled;
        let label_text = self.props.label.clone();
        let on_change = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::CheckboxBtn) {
            ComponentTokens::CheckboxBtn(t) => t,
            _ => unreachable!(),
        };
        let tk = tokens.clone();

        let mark = label(move || {
            if checked.get() {
                "✓".to_string()
            } else {
                String::new()
            }
        })
        .style(move |s| {
            let t = theme.get();
            let mk = if checked.get() {
                t.colors().on_primary
            } else {
                Color::rgba(0, 0, 0, 0)
            };
            let mk = if disabled {
                with_alpha(t.colors().on_surface, 38)
            } else {
                mk
            };
            s.color(mk.to_peniko())
                .font_size(tk.font_size * 1.2)
                .font_weight(FontWeight::Bold.to_floem())
        });

        let label_view = match label_text {
            Some(txt) => text(txt)
                .style(move |s| {
                    let t = theme.get();
                    let c = if disabled {
                        with_alpha(t.colors().on_surface, 38)
                    } else {
                        t.colors().on_surface
                    };
                    s.color(c.to_peniko()).font_size(tk.font_size)
                })
                .into_any(),
            None => Empty::new().into_any(),
        };

        let toggle = Rc::new(move || {
            if !disabled {
                let next = !checked.get();
                checked.set(next);
                if let Some(cb) = on_change.as_ref() {
                    cb(next);
                }
            }
        });

        h_stack((mark, label_view))
            .style(move |s| {
                let t = theme.get();
                let is_checked = checked.get();
                let (bg, border) = if is_checked {
                    (t.colors().primary, t.colors().primary)
                } else {
                    (t.colors().surface_variant, t.colors().outline)
                };
                let (bg, border) = if disabled {
                    (
                        with_alpha(t.colors().on_surface, 12),
                        with_alpha(t.colors().on_surface, 38),
                    )
                } else {
                    (bg, border)
                };
                s.height(tk.height)
                    .padding_horiz(tk.padding)
                    .gap(tk.gap)
                    .border_radius(tk.radius)
                    .background(bg.to_peniko())
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled)
                    .keyboard_navigable()
            })
            .on_event(listener::Click, {
                let toggle = toggle.clone();
                move |_, _| {
                    toggle();
                    EventPropagation::Continue
                }
            })
            .on_event(listener::KeyDown, move |_, e: &KeyboardEvent| {
                if e.key == Key::Named(NamedKey::Enter)
                    || e.key == Key::Character(" ".to_string())
                {
                    toggle();
                }
                EventPropagation::Continue
            })
    }
}

impl Default for FluentCheckboxBtn {
    fn default() -> Self {
        Self::new()
    }
}
