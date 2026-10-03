use std::rc::Rc;

use floem::prelude::*;
use floem_widget_fluent_icon::fluent_icon_svg;
use floem::style::CursorStyle;
use floem::event::EventPropagation;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::checkbox::CheckboxProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

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
    theme: RwSignal<MultiTheme>,
}

impl FluentCheckbox {
    pub fn new(label: impl Into<String>) -> Self {
        let props = CheckboxProps {
            label: label.into(),
            ..Default::default()
        };
        Self {
            checked: RwSignal::new(props.checked),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
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

    /// Build a renderable, interactive Fluent checkbox.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let checked = self.checked;
        let disabled = self.props.disabled;
        let on_change = Rc::new(self.props.on_change);
        let label_text = self.props.label.clone();
        let hover = RwSignal::new(false);

        let tokens = match theme.get().component(ComponentId::Checkbox) {
            ComponentTokens::Checkbox(t) => t,
            _ => unreachable!(),
        };
        let size = tokens.size;
        let radius = tokens.border_radius;

        let mark = svg(move || fluent_icon_svg("check")).style(move |s| {
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
            s.size(size * 0.7, size * 0.7).color(mk.to_peniko())
        });

        let hover_bg = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let pad = 4.0;
            s.absolute()
                .inset_top(0.0)
                .inset_left(0.0)
                .size(size + pad * 2.0, size + pad * 2.0)
                .translate_x(-pad)
                .translate_y(-pad)
                .border_radius(radius + pad)
                .background(with_alpha(t.colors().on_surface, 8).to_peniko())
                .apply_if(!hover.get() || disabled, |s| s.hide())
        });

        let box_ = Container::new(
            Stack::new((hover_bg, mark))
                .style(|s| s.width_full().height_full().items_center().justify_center()),
        )
        .style(move |s| {
            let t = theme.get();
            let is_checked = checked.get();
            let (bg, border) = if is_checked {
                (t.colors().primary, t.colors().primary)
            } else {
                (Color::rgba(0, 0, 0, 0), t.colors().outline)
            };
            let (bg, border) = if disabled {
                (
                    with_alpha(t.colors().on_surface, 12),
                    with_alpha(t.colors().on_surface, 38),
                )
            } else {
                (bg, border)
            };
            s.size(size, size)
                .border_radius(radius)
                .background(bg.to_peniko())
                .border(2.0)
                .border_color(border.to_peniko())
                .items_center()
                .justify_center()
        });

        let label_view = Label::new(label_text).style(move |s| {
            let t = theme.get();
            let c = if disabled {
                with_alpha(t.colors().on_surface, 38)
            } else {
                t.colors().on_surface
            };
            s.color(c.to_peniko())
                .font_size(t.typography().body.medium.font_size)
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

        Stack::horizontal((box_, label_view))
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
            .on_event(listener::PointerEnter, {
                let hover = hover;
                move |_, _| {
                    if !disabled {
                        hover.set(true);
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerLeave, {
                let hover = hover;
                move |_, _| {
                    hover.set(false);
                    EventPropagation::Continue
                }
            })
            .style(move |s| {
                let t = theme.get();
                s.set_disabled(disabled)
                    .keyboard_navigable()
                    .focus(|s| s.outline(2.0).outline_color(t.colors().primary.to_peniko()))
            })
    }
}
