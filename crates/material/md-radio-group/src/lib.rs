use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::Decorators;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::radio_group::RadioGroupProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdRadioGroupStyle {
    pub design_system: &'static str,
    pub selected_color: Color,
    pub unselected_color: Color,
    pub radio_size: f32,
    pub item_height: f32,
    pub gap: f32,
}

pub struct MdRadioGroup {
    props: RadioGroupProps,
    theme: RwSignal<MultiTheme>,
}

impl MdRadioGroup {
    pub fn new() -> Self {
        Self {
            props: RadioGroupProps::default(),
            theme: theme_signal(),
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdRadioGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::RadioGroup(tokens) = theme.component(ComponentId::RadioGroup) else {
            unreachable!("ComponentId::RadioGroup must resolve to RadioGroupTokens")
        };
        MdRadioGroupStyle {
            design_system: "material",
            selected_color: theme.colors().primary,
            unselected_color: theme.colors().outline,
            radio_size: tokens.radio_size,
            item_height: tokens.item_height,
            gap: tokens.gap,
        }
    }

    /// Build a renderable, interactive Material 3 radio group.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let on_change: Option<Rc<Box<dyn Fn(usize)>>> = self.props.on_change.map(Rc::new);
        let disabled = RwSignal::new(self.props.disabled);
        let selected = RwSignal::new(self.props.selected);

        let tokens = match theme.get().component(ComponentId::RadioGroup) {
            ComponentTokens::RadioGroup(t) => t,
            _ => unreachable!(),
        };
        let radio_size = tokens.radio_size;
        let item_height = tokens.item_height;
        let gap = tokens.gap;

        let rows: Vec<AnyView> = self
            .props
            .options
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let i = i;
                let label = label.clone();
                let theme = theme;
                let selected = selected;
                let on_change = on_change.clone();
                let disabled = disabled;
                let radio_size = radio_size;
                let item_height = item_height;

                let is_selected = move || selected.get() == Some(i);

                let indicator = Container::new(
                    Container::new(Empty::new()).style(move |s| {
                        let t = theme.get();
                        s.size(radio_size * 0.5, radio_size * 0.5)
                            .border_radius(9999.0)
                            .background(t.colors().primary.to_peniko())
                            .apply_if(!is_selected(), |s| s.hide())
                    }),
                )
                .style(move |s| {
                    let t = theme.get();
                    let c = if is_selected() {
                        t.colors().primary
                    } else {
                        t.colors().outline
                    };
                    s.size(radio_size, radio_size)
                        .border_radius(9999.0)
                        .border(2.0)
                        .border_color(c.to_peniko())
                        .items_center()
                        .justify_center()
                });

                h_stack((indicator, text(label)))
                    .style(move |s| {
                        s.height(item_height)
                            .items_center()
                            .gap(8.0)
                            .cursor(CursorStyle::Pointer)
                            .set_disabled(disabled.get())
                    })
                    .on_event(listener::Click, move |_, _| {
                        if !disabled.get() {
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

        list(rows).style(move |s| s.gap(gap))
    }
}

impl Default for MdRadioGroup {
    fn default() -> Self {
        Self::new()
    }
}
