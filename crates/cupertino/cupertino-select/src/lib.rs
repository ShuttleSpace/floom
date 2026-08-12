use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::IntoView;
use floem::reactive::RwSignal;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::select::SelectProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoSelectStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub item_height: f32,
}

pub struct CupertinoSelect {
    props: SelectProps,
    selected: RwSignal<Option<usize>>,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSelect {
    pub fn new() -> Self {
        let props = SelectProps::default();
        Self {
            selected: RwSignal::new(props.selected),
            open: RwSignal::new(false),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
        }
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
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }
    pub fn on_change(mut self, callback: impl Fn(Option<usize>) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &SelectProps {
        &self.props
    }
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoSelectStyle {
        let theme = self.theme.get();
        let ComponentTokens::Select(tokens) = theme.component(ComponentId::Select) else {
            unreachable!("ComponentId::Select must resolve to SelectTokens")
        };
        CupertinoSelectStyle {
            design_system: DESIGN_SYSTEM,
            background: theme.colors().surface,
            foreground: theme.colors().on_surface,
            border_color: theme.colors().outline,
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
        }
    }

    /// Build a renderable Cupertino select with a dropdown of options.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let selected = self.selected;
        let on_change = Rc::new(self.props.on_change);
        let disabled = self.props.disabled;
        let options = self.props.options.clone();
        let options_for_label = options.clone();
        let options_for_style = options.clone();
        let placeholder = self.props.placeholder.clone();

        let tokens = match theme.get().component(ComponentId::Select) {
            ComponentTokens::Select(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let min_width = tokens.min_width;
        let item_height = tokens.item_height;

        let label_text = label(move || {
            let idx = selected.get();
            match idx {
                Some(i) if i < options_for_label.len() => options_for_label[i].clone(),
                _ => placeholder.clone(),
            }
        })
        .style(move |s| {
            let t = theme.get();
            let idx = selected.get();
            let is_placeholder = idx.map(|i| i >= options_for_style.len()).unwrap_or(true);
            let c = if disabled {
                with_alpha(t.colors().on_surface, 97)
            } else if is_placeholder {
                with_alpha(t.colors().on_surface, 140)
            } else {
                t.colors().on_surface
            };
            s.color(c.to_peniko())
                .font_size(t.typography().body.medium.font_size)
                .flex_basis(1.0)
                .flex_grow(1.0)
        });

        let chevron = text("▾").style(move |s| {
            let t = theme.get();
            s.color(with_alpha(t.colors().on_surface, 140).to_peniko())
                .font_size(12.0)
        });

        let trigger = Container::new(
            h_stack((label_text, chevron)).style(|s| s.items_center().justify_between().gap(8.0)),
        )
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
                t.colors().surface
            };
            s.height(height)
                .min_width(min_width)
                .padding_horiz(padding_horizontal)
                .border(1.0)
                .border_color(border.to_peniko())
                .border_radius(radius)
                .background(bg.to_peniko())
                .items_center()
                .cursor(CursorStyle::Pointer)
                .apply_if(disabled, |s| s.set_disabled(true).cursor(CursorStyle::Default))
                .hover(move |s| {
                    let t = theme.get();
                    s.border_color(with_alpha(t.colors().primary, 160).to_peniko())
                })
        })
        .on_event(listener::Click, move |_, _| {
            if !disabled {
                open.set(!open.get());
            }
            EventPropagation::Continue
        });

        let rows: Vec<AnyView> = options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let on_select = on_change.clone();
                let sel_sig = selected;
                let open_sig = open;
                let theme_row = theme;
                let opt = opt.clone();
                text(opt)
                    .style(move |s| {
                        let t = theme_row.get();
                        let is_sel = sel_sig.get() == Some(i);
                        s.width_pct(100.0)
                            .padding_horiz(12.0)
                            .height(item_height)
                            .items_center()
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(t.typography().body.medium.font_size)
                            .background(
                                if is_sel {
                                    with_alpha(t.colors().primary, 24).to_peniko()
                                } else {
                                    Color::rgba(0, 0, 0, 0).to_peniko()
                                },
                            )
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                let t = theme_row.get();
                                s.background(with_alpha(t.colors().primary, 12).to_peniko())
                            })
                    })
                    .on_event(listener::Click, move |_, _| {
                        sel_sig.set(Some(i));
                        if let Some(cb) = on_select.as_ref() {
                            cb(Some(i));
                        }
                        open_sig.set(false);
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        let dropdown = Container::new(list(rows)).style(move |s| {
            let t = theme.get();
            s.min_width(min_width)
                .margin_top(4.0)
                .padding(4.0)
                .background(t.colors().surface.to_peniko())
                .border_radius(radius)
                .box_shadow_blur(12.0)
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 24).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        });

        v_stack((trigger, dropdown)).style(|s| s.items_start())
    }
}

impl Default for CupertinoSelect {
    fn default() -> Self {
        Self::new()
    }
}
