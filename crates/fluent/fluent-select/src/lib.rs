use std::rc::Rc;

use floem::context::{LayoutChanged, LayoutChangedListener};
use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::IntoView;
use floem::reactive::create_effect;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::select::SelectProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentSelectStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub item_height: f32,
}

pub struct FluentSelect {
    props: SelectProps,
    theme: RwSignal<MultiTheme>,
    selected: RwSignal<Option<usize>>,
    open: RwSignal<bool>,
}

impl FluentSelect {
    pub fn new() -> Self {
        let props = SelectProps::default();
        Self {
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            selected: RwSignal::new(props.selected),
            open: RwSignal::new(false),
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
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentSelectStyle {
        let theme = self.theme.get();
        let ComponentTokens::Select(tok) = theme.component(ComponentId::Select) else {
            unreachable!("ComponentId::Select must resolve to SelectTokens")
        };
        FluentSelectStyle {
            design_system: DESIGN_SYSTEM,
            background: theme.colors().surface,
            text_color: theme.colors().on_surface,
            border_color: theme.colors().outline,
            height: tok.height,
            padding_horizontal: tok.padding_horizontal,
            border_radius: tok.border_radius,
            min_width: tok.min_width,
            item_height: tok.item_height,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let selected = self.selected;
        let disabled = self.props.disabled;
        let options = self.props.options.clone();
        let placeholder = self.props.placeholder.clone();
        let on_change = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Select) {
            ComponentTokens::Select(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let min_width = tokens.min_width;
        let item_height = tokens.item_height;

        let origin = RwSignal::new((0.0f64, 0.0f64));
        let trigger_h = RwSignal::new(0.0f32);

        let rows: Vec<AnyView> = options
            .clone()
            .into_iter()
            .enumerate()
            .map(|(i, opt)| {
                let theme_row = theme;
                let selected = selected;
                let open = open;
                let on_change = on_change.clone();
                text(opt)
                    .style(move |s| {
                        let th = theme_row;
                        let t = th.get();
                        let active = selected.get() == Some(i);
                        s.width_pct(100.0)
                            .height(item_height)
                            .padding_horiz(12.0)
                            .items_center()
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(t.typography().body.medium.font_size)
                            .background(
                                if active {
                                    with_alpha(t.colors().primary, 24).to_peniko()
                                } else {
                                    Color::rgba(0, 0, 0, 0).to_peniko()
                                },
                            )
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                let t = th.get();
                                s.background(with_alpha(t.colors().on_surface, 8).to_peniko())
                            })
                    })
                    .on_event(listener::Click, move |_, _| {
                        selected.set(Some(i));
                        if let Some(cb) = on_change.as_ref() {
                            cb(Some(i));
                        }
                        open.set(false);
                        EventPropagation::Continue
                    })
                    .into_any()
            })
            .collect();

        let panel = Container::new(list(rows)).style(move |s| {
            let t = theme.get();
            let (ox, oy) = origin.get();
            let h = trigger_h.get() as f64;
            s.absolute()
                .inset_top(oy + h)
                .inset_left(ox)
                .min_width(min_width)
                .padding(4.0)
                .background(t.colors().surface.to_peniko())
                .border_radius(radius)
                .box_shadow_blur(12.0)
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(BLACK, 20).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        });

        let dropdown = Overlay::new(panel);

        let trigger_label = label(move || match selected.get() {
            Some(i) => options
                .get(i)
                .cloned()
                .unwrap_or_else(|| placeholder.clone()),
            None => placeholder.clone(),
        });

        let chevron = text("▾").style(move |s| {
            let t = theme.get();
            s.color(with_alpha(t.colors().on_surface, 160).to_peniko())
                .font_size(12.0)
        });

        let trigger = Container::new(
            h_stack((trigger_label, chevron)).style(|s| s.items_center().justify_between().gap(8.0)),
        )
        .style(move |s| {
            let t = theme.get();
            let border = if disabled {
                with_alpha(t.colors().on_surface, 38)
            } else {
                t.colors().outline
            };
            s.height(height)
                .padding_horiz(padding_horizontal)
                .min_width(min_width)
                .border(1.0)
                .border_color(border.to_peniko())
                .border_radius(radius)
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
                .items_center()
                .cursor(if disabled {
                    CursorStyle::Default
                } else {
                    CursorStyle::Pointer
                })
                .apply_if(disabled, |s| s.set_disabled(true))
        })
        .on_event(listener::Click, move |_, _| {
            if !disabled {
                open.set(!open.get());
            }
            EventPropagation::Continue
        });

        Container::new(v_stack((trigger, dropdown)))
            .style(|s| s.items_start())
            .on_event(LayoutChangedListener, move |_cx, layout: &LayoutChanged| {
                origin.set((layout.new_window_origin.x, layout.new_window_origin.y));
                trigger_h.set(layout.new_box.size().height as f32);
                EventPropagation::Continue
            })
    }
}

impl Default for FluentSelect {
    fn default() -> Self {
        Self::new()
    }
}
