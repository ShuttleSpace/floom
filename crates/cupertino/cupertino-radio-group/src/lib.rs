use std::rc::Rc;

use floem::event::EventPropagation;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::Decorators;
use floem::{
    prelude::*,
    reactive::RwSignal,
};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::radio_group::RadioGroupProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoRadioGroupStyle {
    pub design_system: &'static str,
    pub selected_color: Color,
    pub ring_color: Color,
    pub label_color: Color,
    pub radio_size: f32,
    pub item_height: f32,
    pub gap: f32,
}

pub struct CupertinoRadioGroup {
    props: RadioGroupProps,
    theme: RwSignal<MultiTheme>,
    selected: RwSignal<Option<usize>>,
}

impl CupertinoRadioGroup {
    pub fn new() -> Self {
        let props = RadioGroupProps::default();
        let selected = props.selected;
        Self {
            props,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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
    pub fn resolved_style(&self) -> CupertinoRadioGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::RadioGroup(tokens) = theme.component(ComponentId::RadioGroup) else {
            unreachable!("ComponentId::RadioGroup must resolve to RadioGroupTokens")
        };
        CupertinoRadioGroupStyle {
            design_system: DESIGN_SYSTEM,
            selected_color: theme.colors().primary,
            ring_color: theme.colors().outline,
            label_color: theme.colors().on_surface,
            radio_size: tokens.radio_size,
            item_height: tokens.item_height,
            gap: tokens.gap,
        }
    }

    /// Build a renderable, interactive Cupertino radio group.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let selected = self.selected;
        let on_change = Rc::new(self.props.on_change);
        let disabled = self.props.disabled;
        let options = self.props.options.clone();

        let tokens = match theme.get().component(ComponentId::RadioGroup) {
            ComponentTokens::RadioGroup(t) => t,
            _ => unreachable!(),
        };
        let radio_size = tokens.radio_size;
        let item_height = tokens.item_height;
        let gap = tokens.gap;

        let rows: Vec<AnyView> = options
            .into_iter()
            .enumerate()
            .map(|(i, label)| {
                let selected = selected;
                let on_change = on_change.clone();
                let theme = theme;

                let indicator = Container::new(Empty::new())
                    .style(move |s| {
                        let t = theme.get();
                        s.size(radio_size * 0.45, radio_size * 0.45)
                            .background(t.colors().primary.to_peniko())
                            .border_radius(t.radius().full)
                            .apply_if(selected.get() != Some(i), |s| s.hide())
                    });

                let radio = Container::new(indicator)
                    .style(move |s| {
                        let t = theme.get();
                        s.size(radio_size, radio_size)
                            .border(2.0)
                            .border_color(t.colors().outline.to_peniko())
                            .border_radius(t.radius().full)
                            .items_center()
                            .justify_center()
                    });

                let row = h_stack((radio, text(label)))
                    .style(move |s| {
                        s.height(item_height)
                            .gap(gap)
                            .items_center()
                            .cursor(CursorStyle::Pointer)
                            .apply_if(disabled, |s| s.set_disabled(true))
                    })
                    .on_event(listener::Click, move |_, _| {
                        if disabled {
                            return EventPropagation::Continue;
                        }
                        selected.set(Some(i));
                        if let Some(cb) = on_change.as_ref() {
                            cb(i);
                        }
                        EventPropagation::Continue
                    });

                row.into_any()
            })
            .collect();

        Container::new(list(rows)).style(move |s| {
            s.flex_col().gap(gap)
        })
    }
}

impl Default for CupertinoRadioGroup {
    fn default() -> Self {
        Self::new()
    }
}
