use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::item_group::ItemGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoItemGroupStyle {
    pub design_system: &'static str,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub primary: Color,
    pub outline: Color,
    pub gap: f32,
    pub pad: f32,
    pub radius: f32,
}

pub struct CupertinoItemGroup {
    props: ItemGroupProps,
    theme: RwSignal<MultiTheme>,
    items: Vec<AnyView>,
    single: RwSignal<usize>,
    multi: RwSignal<Vec<usize>>,
    on_change: Option<Box<dyn Fn(usize)>>,
    on_change_multi: Option<Box<dyn Fn(Vec<usize>)>>,
}

impl CupertinoItemGroup {
    pub fn new() -> Self {
        Self {
            props: ItemGroupProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            items: Vec::new(),
            single: RwSignal::new(0),
            multi: RwSignal::new(Vec::new()),
            on_change: None,
            on_change_multi: None,
        }
    }

    /// Provide the selectable item views. `AnyView` is owned (not `Clone`).
    pub fn items(mut self, items: Vec<AnyView>) -> Self {
        self.items = items;
        self
    }

    pub fn selected(mut self, selected: usize) -> Self {
        self.single = RwSignal::new(selected);
        self.multi = RwSignal::new(vec![selected]);
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

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn on_change_multi(mut self, callback: impl Fn(Vec<usize>) + 'static) -> Self {
        self.on_change_multi = Some(Box::new(callback));
        self
    }

    /// The component owns these signals (mirrors `Dialog::open_signal()`); the consumer
    /// obtains them and reads selection state.
    pub fn selected_signal(&self) -> RwSignal<usize> {
        self.single
    }

    pub fn selected_multi_signal(&self) -> RwSignal<Vec<usize>> {
        self.multi
    }

    pub fn resolved_style(&self) -> CupertinoItemGroupStyle {
        let theme = self.theme.get();
        let ComponentTokens::ItemGroup(tokens) =
            theme.component(ComponentId::ItemGroup)
        else {
            unreachable!("ComponentId::ItemGroup must resolve to ItemGroupTokens")
        };
        CupertinoItemGroupStyle {
            design_system: "cupertino",
            surface_variant: theme.colors().surface_variant,
            on_surface: theme.colors().on_surface,
            primary_container: theme.colors().primary_container,
            on_primary_container: theme.colors().on_primary_container,
            primary: theme.colors().primary,
            outline: theme.colors().outline,
            gap: tokens.gap,
            pad: tokens.pad,
            radius: tokens.radius,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.items;
        let n = items.len();
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.on_change);
        let on_change_multi: Rc<Option<Box<dyn Fn(Vec<usize>)>>> = Rc::new(self.on_change_multi);
        let multiple = self.props.multiple;
        let mandatory = self.props.mandatory;
        let single = self.single;
        let multi = self.multi;

        let tokens = match theme.get().component(ComponentId::ItemGroup) {
            ComponentTokens::ItemGroup(t) => t,
            _ => unreachable!(),
        };
        let gap = tokens.gap;
        let pad = tokens.pad;
        let radius = tokens.radius;

        let rows: Vec<AnyView> = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let single = single;
                let multi = multi.clone();
                let on_change = on_change.clone();
                let on_change_multi = on_change_multi.clone();
                let multiple = multiple;
                let mandatory = mandatory;
                Container::new(item)
                    .style(move |s| {
                        let selected = multi.get().contains(&i);
                        let t = theme.get();
                        let (bg, fg, border) = if selected {
                            (
                                t.colors().primary_container.to_peniko(),
                                t.colors().on_primary_container.to_peniko(),
                                t.colors().primary.to_peniko(),
                            )
                        } else {
                            (
                                t.colors().surface_variant.to_peniko(),
                                t.colors().on_surface.to_peniko(),
                                t.colors().outline.to_peniko(),
                            )
                        };
                        s.padding_horiz(pad)
                            .padding_vert(pad / 2.0)
                            .border_radius(radius)
                            .border(1.0)
                            .border_color(border)
                            .background(bg)
                            .color(fg)
                            .cursor(CursorStyle::Pointer)
                    })
                    .on_event(listener::Click, move |_, _| {
                        if multiple {
                            let mut cur = multi.get();
                            if let Some(pos) = cur.iter().position(|x| *x == i) {
                                if !(mandatory && cur.len() == 1) {
                                    cur.remove(pos);
                                }
                            } else {
                                cur.push(i);
                            }
                            multi.set(cur.clone());
                            single.set(i);
                            if let Some(cb) = on_change_multi.as_ref() {
                                cb(cur);
                            }
                        } else {
                            let cur = single.get();
                            if !(mandatory && cur == i) {
                                single.set(i);
                                multi.set(vec![i]);
                                if let Some(cb) = on_change.as_ref() {
                                    cb(i);
                                }
                            }
                        }
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        Stack::vertical_from_iter(rows).style(move |s| s.gap(gap).width_pct(100.0).apply_if(n == 0, |s| s.hide()))
    }
}

impl Default for CupertinoItemGroup {
    fn default() -> Self {
        Self::new()
    }
}
