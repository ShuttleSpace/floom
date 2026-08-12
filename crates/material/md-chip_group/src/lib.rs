use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{Container, Decorators, h_stack_from_iter};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::chip_group::ChipGroupProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

pub struct MdChipGroup {
    props: ChipGroupProps,
    labels: Vec<String>,
    theme: RwSignal<MultiTheme>,
}

impl MdChipGroup {
    pub fn new() -> Self {
        Self {
            props: ChipGroupProps::default(),
            labels: Vec::new(),
            theme: theme_signal(),
        }
    }

    pub fn chip(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(Vec<usize>) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let labels = self.labels;
        let multiple = self.props.multiple;
        let mandatory = self.props.mandatory;
        let on_change = Rc::new(self.props.on_change);
        let n = labels.len();

        let tokens = match theme.get().component(ComponentId::ChipGroup) {
            ComponentTokens::ChipGroup(t) => t,
            _ => unreachable!(),
        };

        let selected: RwSignal<Vec<bool>> = RwSignal::new(vec![false; n]);
        let tokens_final = tokens.clone();

        let chip_views: Vec<AnyView> = labels
            .into_iter()
            .enumerate()
            .map(move |(i, label)| {
                let theme = theme;
                let tk = tokens.clone();
                let selected = selected;
                let on_change = on_change.clone();
                let multiple = multiple;
                let mandatory = mandatory;

                Container::new(text(label))
                    .style(move |s| {
                        let t = theme.get();
                        let sel = selected.get()[i];
                        let (bg, fg) = if sel {
                            (
                                t.colors().secondary_container,
                                t.colors().on_secondary_container,
                            )
                        } else {
                            (t.colors().surface_variant, t.colors().on_surface)
                        };
                        s.height(32.0)
                            .padding_horiz(12.0)
                            .gap(6.0)
                            .border_radius(16.0)
                            .background(bg.to_peniko())
                            .color(fg.to_peniko())
                            .font_size(13.0)
                            .items_center()
                            .justify_center()
                            .cursor(CursorStyle::Pointer)
                    })
                    .on_event(listener::Click, {
                        let selected = selected;
                        let on_change = on_change.clone();
                        move |_, _| {
                            let cur = selected.get()[i];
                            let mut all = selected.get();
                            if multiple {
                                all[i] = !cur;
                            } else if !(cur && mandatory) {
                                for v in all.iter_mut() {
                                    *v = false;
                                }
                                all[i] = !cur;
                            }
                            selected.set(all);
                            let idxs: Vec<usize> =
                                (0..n).filter(|&k| selected.get()[k]).collect();
                            if let Some(cb) = on_change.as_ref() {
                                cb(idxs);
                            }
                            EventPropagation::Continue
                        }
                    })
                    .into_any()
            })
            .collect();

        h_stack_from_iter(chip_views)
            .style(move |s| s.gap(tokens_final.gap).items_center())
    }
}

impl Default for MdChipGroup {
    fn default() -> Self {
        Self::new()
    }
}
