use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::{create_effect, RwSignal, SignalGet, SignalUpdate};
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::IntoView;
use floom::context::{LayoutChanged, LayoutChangedListener};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::select::SelectProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;
use floom_widget_md_icon::MdIcon;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSelectStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
    pub min_width: f32,
    pub item_height: f32,
}

pub struct MdSelect {
    props: SelectProps,
    selected: RwSignal<Option<usize>>,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl MdSelect {
    pub fn new() -> Self {
        let props = SelectProps::default();
        Self {
            selected: RwSignal::new(props.selected),
            open: RwSignal::new(false),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MdSelectStyle {
        let theme = self.theme.get();
        let ComponentTokens::Select(tokens) = theme.component(ComponentId::Select) else {
            unreachable!("ComponentId::Select must resolve to SelectTokens")
        };
        MdSelectStyle {
            design_system: "material",
            surface: theme.colors().surface,
            on_surface: theme.colors().on_surface,
            on_surface_variant: theme.colors().on_surface_variant,
            outline: theme.colors().outline,
            primary: theme.colors().primary,
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
        }
    }

    /// Build a renderable, interactive Material 3 select (dropdown).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let options = self.props.options.clone();
        let placeholder = self.props.placeholder.clone();
        let selected = self.selected;
        let open = self.open;
        let on_change: Rc<Option<Box<dyn Fn(Option<usize>)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Select) {
            ComponentTokens::Select(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let min_width = tokens.min_width;
        let item_height = tokens.item_height;
        let icon_size = tokens.icon_size;

        let origin = RwSignal::new((0.0f64, 0.0f64));
        let trigger_h = RwSignal::new(0.0f32);

        let label_options = options.clone();
        let label = Label::derived(move || {
            let idx = selected.get();
            match idx {
                Some(i) if i < label_options.len() => label_options[i].clone(),
                _ => placeholder.clone(),
            }
        })
        .style(move |s| {
            let t = theme.get();
            let is_ph = selected.get().is_none();
            let c = if disabled {
                with_alpha(t.colors().on_surface, 97)
            } else if is_ph {
                t.colors().on_surface_variant
            } else {
                t.colors().on_surface
            };
            s.color(c.to_peniko())
                .font_size(t.typography().body.medium.font_size)
        });

        let chevron = MdIcon::new("expand_more")
            .size(icon_size)
            .view()
            .style(move |s| {
                let t = theme.get();
                s.color(
                    if disabled {
                        with_alpha(t.colors().on_surface, 97)
                    } else {
                        t.colors().on_surface_variant
                    }
                    .to_peniko(),
                )
            });

        let trigger = Container::new(Stack::horizontal((label, chevron)).style(|s| s.items_center().gap(8.0)))
            .style(move |s| {
                let t = theme.get();
                let border = if disabled {
                    with_alpha(t.colors().outline, 97)
                } else {
                    t.colors().outline
                };
                s.height(height)
                    .min_width(min_width)
                    .padding_horiz(padding_horizontal)
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .border_radius(radius)
                    .background(t.colors().surface.to_peniko())
                    .items_center()
                    .justify_between()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled)
            })
            .on_event(listener::Click, {
                let open = open;
                move |_, _| {
                    if !disabled {
                        open.set(!open.get());
                    }
                    EventPropagation::Continue
                }
            });

        let rows: Vec<AnyView> = options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let opt = opt.clone();
                let selected = selected;
                let open = open;
                let on_change = on_change.clone();
                Label::new(opt)
                    .style(move |s| {
                        let t = theme.get();
                        let is_sel = selected.get() == Some(i);
                        s.width_pct(100.0)
                            .padding_horiz(12.0)
                            .height(item_height)
                            .items_center()
                            .color(t.colors().on_surface.to_peniko())
                            .font_size(t.typography().body.medium.font_size)
                            .background(if is_sel {
                                with_alpha(t.colors().primary, 24).to_peniko()
                            } else {
                                Color::rgba(0, 0, 0, 0).to_peniko()
                            })
                            .cursor(CursorStyle::Pointer)
                            .hover(|s| {
                                let t = theme.get();
                                s.background(with_alpha(t.colors().on_surface, 8).to_peniko())
                            })
                    })
                    .on_event(listener::Click, move |_, _| {
                        selected.set(Some(i));
                        open.set(false);
                        if let Some(cb) = on_change.as_ref() {
                            cb(Some(i));
                        }
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
                .border(1.0)
                .border_color(with_alpha(t.colors().outline, 120).to_peniko())
                .box_shadow_blur(8.0)
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 20).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        });
        let dropdown = Overlay::new(panel);

        Container::new(Stack::vertical((trigger, dropdown)))
            .style(|s| s.items_start())
            .on_event(LayoutChangedListener, move |_cx, layout: &LayoutChanged| {
                origin.set((layout.new_window_origin.x, layout.new_window_origin.y));
                trigger_h.set(layout.new_box.size().height as f32);
                EventPropagation::Continue
            })
    }
}

impl Default for MdSelect {
    fn default() -> Self {
        Self::new()
    }
}
