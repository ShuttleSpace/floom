use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::Decorators;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::radio_group::RadioGroupProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentRadioGroupStyle {
    pub design_system: &'static str,
    pub indicator_color: Color,
    pub label_color: Color,
    pub radio_size: f32,
    pub item_height: f32,
    pub gap: f32,
}

pub struct FluentRadioGroup {
    props: RadioGroupProps,
    theme: RwSignal<MultiTheme>,
    selected: RwSignal<Option<usize>>,
}

impl FluentRadioGroup {
    pub fn new() -> Self {
        let props = RadioGroupProps::default();
        let selected = props.selected;
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            props,
            selected: RwSignal::new(selected),
        }
    }
    pub fn option(mut self, option: impl Into<String>) -> Self {
        self.props.options.push(option.into());
        self
    }
    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }
    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.props.selected = selected;
        self.selected.set(selected);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &RadioGroupProps {
        &self.props
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentRadioGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::RadioGroup(tk) = theme.component(ComponentId::RadioGroup) else {
            unreachable!("ComponentId::RadioGroup must resolve to RadioGroupTokens")
        };
        FluentRadioGroupStyle {
            design_system: DESIGN_SYSTEM,
            indicator_color: theme.colors().primary,
            label_color: theme.colors().on_surface,
            radio_size: tk.radio_size,
            item_height: tk.item_height,
            gap: tk.gap,
        }
    }

    /// Build a renderable, interactive Fluent radio group.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let disabled = self.props.disabled;
        let on_change = Rc::new(self.props.on_change);
        let options = self.props.options;

        let rows: Vec<AnyView> = options
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let theme = theme;
                let selected = selected;
                let on_change = on_change.clone();
                let disabled = disabled;
                let label = label.clone();

                let indicator = Container::new(
                    Empty::new().style(move |s| {
                        let t = theme.get();
                        let ComponentTokens::RadioGroup(tk) =
                            t.component(ComponentId::RadioGroup) else { unreachable!() };
                        let sel = selected.get() == Some(i);
                        s.width(tk.radio_size * 0.5)
                            .height(tk.radio_size * 0.5)
                            .background(t.colors().primary.to_peniko())
                            .border_radius(t.radius().full)
                            .apply_if(!sel, |s| s.hide())
                    }),
                )
                .style(move |s| {
                    let t = theme.get();
                    let ComponentTokens::RadioGroup(tk) =
                        t.component(ComponentId::RadioGroup) else { unreachable!() };
                    let sel = selected.get() == Some(i);
                    s.width(tk.radio_size)
                        .height(tk.radio_size)
                        .border_radius(t.radius().full)
                        .border(2.0)
                        .border_color(
                            (if sel {
                                t.colors().primary
                            } else {
                                t.colors().outline
                            })
                            .to_peniko(),
                        )
                        .items_center()
                        .justify_center()
                });

                let label_view = text(label).style(move |s| {
                    let t = theme.get();
                    let sel = selected.get() == Some(i);
                    s.color(
                        (if sel {
                            t.colors().primary
                        } else {
                            t.colors().on_surface
                        })
                        .to_peniko(),
                    )
                    .font_size(t.typography().body.medium.font_size)
                    .font_weight(
                        (if sel {
                            FontWeight::SemiBold
                        } else {
                            FontWeight::Medium
                        })
                        .to_floem(),
                    )
                });

                h_stack((indicator, label_view))
                    .style(move |s| {
                        let ComponentTokens::RadioGroup(tk) =
                            theme.get().component(ComponentId::RadioGroup) else { unreachable!() };
                        s.height(tk.item_height)
                            .items_center()
                            .gap(8.0)
                            .cursor(if disabled {
                                CursorStyle::Default
                            } else {
                                CursorStyle::Pointer
                            })
                            .apply_if(disabled, |s| s.set_disabled(true))
                    })
                    .on_event(listener::Click, move |_, _| {
                        if !disabled {
                            selected.set(Some(i));
                            if let Some(cb) = on_change.as_ref() {
                                cb(i);
                            }
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        Container::new(list(rows)).style(move |s| {
            let ComponentTokens::RadioGroup(tk) =
                theme.get().component(ComponentId::RadioGroup) else { unreachable!() };
            s.flex_col().gap(tk.gap)
        })
    }
}

impl Default for FluentRadioGroup {
    fn default() -> Self {
        Self::new()
    }
}
