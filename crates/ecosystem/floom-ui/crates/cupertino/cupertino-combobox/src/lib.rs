use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom_widget_cupertino_icon::cupertino_icon_svg;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::IntoView;
use floom::context::{LayoutChanged, LayoutChangedListener};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::combobox::ComboboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoComboboxStyle {
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

pub struct CupertinoCombobox {
    props: ComboboxProps,
    selected: RwSignal<Option<usize>>,
    open: RwSignal<bool>,
    query: RwSignal<String>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoCombobox {
    pub fn new() -> Self {
        let props = ComboboxProps::default();
        let initial_query = props
            .selected
            .and_then(|i| props.options.get(i).cloned())
            .unwrap_or_default();
        Self {
            selected: RwSignal::new(props.selected),
            open: RwSignal::new(false),
            query: RwSignal::new(initial_query),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
        }
    }

    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options.clone();
        if let Some(i) = self.props.selected {
            if let Some(t) = options.get(i) {
                self.query.set(t.clone());
            }
        }
        self
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.props.selected = selected;
        self.selected.set(selected);
        if let Some(i) = selected {
            if let Some(t) = self.props.options.get(i) {
                self.query.set(t.clone());
            }
        }
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

    pub fn resolved_style(&self) -> CupertinoComboboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Combobox(tokens) = theme.component(ComponentId::Combobox) else {
            unreachable!("ComponentId::Combobox must resolve to ComboboxTokens")
        };
        CupertinoComboboxStyle {
            design_system: "cupertino",
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

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let options = self.props.options.clone();
        let placeholder = self.props.placeholder.clone();
        let selected = self.selected;
        let open = self.open;
        let query = self.query;
        let on_change: Rc<Option<Box<dyn Fn(Option<usize>)>>> = Rc::new(self.props.on_change);

        let tokens = match theme.get().component(ComponentId::Combobox) {
            ComponentTokens::Combobox(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let padding_horizontal = tokens.padding_horizontal;
        let radius = tokens.border_radius;
        let min_width = tokens.min_width;
        let item_height = tokens.item_height;

        let origin = RwSignal::new((0.0f64, 0.0f64));
        let trigger_h = RwSignal::new(0.0f32);

        let input = TextInput::new(query)
            .placeholder(&placeholder)
            .style(move |s| {
                let t = theme.get();
                let c = if disabled {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().on_surface
                };
                s.color(c.to_peniko())
                    .font_size(t.typography().body.medium.font_size)
                    .width_pct(100.0)
            })
            .on_event(listener::FocusGained, move |_, _| {
                if !disabled {
                    open.set(true);
                }
                EventPropagation::Continue
            })
            .style(move |s| s.set_disabled(disabled));

        let chevron = svg(move || cupertino_icon_svg("expand_more"))
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
                .size(14.0, 14.0)
                .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, move |_, _| {
                if !disabled {
                    open.set(!open.get());
                }
                EventPropagation::Continue
            });

        let trigger = Container::new(Stack::horizontal((input, chevron)).style(|s| s.items_center().gap(8.0)))
            .style(move |s| {
                let t = theme.get();
                let border = if disabled {
                    with_alpha(t.colors().outline, 97)
                } else if open.get() {
                    t.colors().primary
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
                    .cursor(CursorStyle::Text)
                    .set_disabled(disabled)
            });

        let rows: Vec<AnyView> = options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let opt = opt.clone();
                let opt_style = opt.clone();
                let opt_event = opt.clone();
                let query = query;
                let selected = selected;
                let open = open;
                let on_change = on_change.clone();
                Label::new(opt)
                    .style(move |s| {
                        let q = query.get().to_lowercase();
                        let matches = q.is_empty() || opt_style.to_lowercase().contains(&q);
                        let is_sel = selected.get() == Some(i);
                        s.width_pct(100.0)
                            .padding_horiz(12.0)
                            .height(item_height)
                            .items_center()
                            .color(theme.get().colors().on_surface.to_peniko())
                            .font_size(theme.get().typography().body.medium.font_size)
                            .background(if is_sel {
                                with_alpha(theme.get().colors().primary, 24).to_peniko()
                            } else {
                                Color::rgba(0, 0, 0, 0).to_peniko()
                            })
                            .cursor(CursorStyle::Pointer)
                            .hover(move |s| {
                                s.background(
                                    with_alpha(theme.get().colors().on_surface, 8).to_peniko(),
                                )
                            })
                            .apply_if(!matches, |s| s.hide())
                    })
                    .on_event(listener::Click, move |_, _| {
                        selected.set(Some(i));
                        query.set(opt_event.clone());
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

impl Default for CupertinoCombobox {
    fn default() -> Self {
        Self::new()
    }
}
