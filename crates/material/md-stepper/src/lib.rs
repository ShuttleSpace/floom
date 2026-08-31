use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, h_stack_from_iter, v_stack, Stack, Label};
use floem::views::svg;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_widget_md_icon::md_icon_svg;
use floem_core_theme::props::stepper::StepperProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::typography::FontWeight;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdStepperStyle {
    pub design_system: &'static str,
    pub step_size: f32,
    pub connector_width: f32,
    pub gap: f32,
    pub label_font_size: f32,
    pub primary: Color,
    pub on_primary: Color,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
}

pub struct MdStepper {
    props: StepperProps,
    theme: RwSignal<MultiTheme>,
    items: RwSignal<Vec<String>>,
    selected: RwSignal<usize>,
    on_change: Option<Box<dyn Fn(usize)>>,
}

impl MdStepper {
    pub fn new() -> Self {
        Self {
            props: StepperProps::default(),
            theme: theme_signal(),
            items: RwSignal::new(Vec::new()),
            selected: RwSignal::new(0),
            on_change: None,
        }
    }

    /// The ordered step labels.
    pub fn items(mut self, items: Vec<String>) -> Self {
        self.items.set(items);
        self
    }

    /// The initially-active step index.
    pub fn selected(mut self, selected: usize) -> Self {
        self.selected.set(selected);
        self
    }

    /// Fired when a step is activated by click (respecting `editable`/`non_linear`).
    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    /// Allow completed/earlier steps to be clicked (navigate backward).
    pub fn editable(mut self, editable: bool) -> Self {
        self.props.editable = editable;
        self
    }

    /// Allow jumping to any step on click (not just the next one).
    pub fn non_linear(mut self, non_linear: bool) -> Self {
        self.props.non_linear = non_linear;
        self
    }

    pub fn resolved_style(&self) -> MdStepperStyle {
        let theme = self.theme.get();
        let ComponentTokens::Stepper(tokens) = theme.component(ComponentId::Stepper) else {
            unreachable!("ComponentId::Stepper must resolve to StepperTokens")
        };
        let c = theme.colors();
        MdStepperStyle {
            design_system: "material",
            step_size: tokens.step_size,
            connector_width: tokens.connector_width,
            gap: tokens.gap,
            label_font_size: tokens.label_font_size,
            primary: c.primary,
            on_primary: c.on_primary,
            surface: c.surface,
            on_surface: c.on_surface,
            on_surface_variant: c.on_surface_variant,
            outline: c.outline,
            outline_variant: c.outline_variant,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let items = self.items;
        let selected = self.selected;
        let on_change: Rc<Option<Box<dyn Fn(usize)>>> = Rc::new(self.on_change);
        let editable = self.props.editable;
        let non_linear = self.props.non_linear;

        let tokens = match theme.get().component(ComponentId::Stepper) {
            ComponentTokens::Stepper(t) => t,
            _ => unreachable!(),
        };
        let step_size = tokens.step_size;
        let connector_width = tokens.connector_width;
        let label_font = tokens.label_font_size;
        let gap = tokens.gap;

        let child = dyn_view(move || {
            let labels = items.get();
            let sel = selected.get();
            let theme = theme;
            let on_change = on_change.clone();
            let n = labels.len();

            let mut kids: Vec<AnyView> = Vec::with_capacity(n * 2);
            for (i, label) in labels.iter().enumerate() {
                let theme_m = theme;
                let sel_m = selected;
                let completed = i < sel;
                let marker = Container::new(if completed {
                    svg(move || md_icon_svg("check"))
                        .style(move |s| {
                            let t = theme_m.get();
                            s.size(step_size * 0.55, step_size * 0.55)
                                .color(t.colors().on_primary.to_peniko())
                        })
                        .into_any()
                } else {
                    Label::new(format!("{}", i + 1)).into_any()
                })
                .style(move |s| {
                    let t = theme_m.get();
                    let done = sel_m.get() > i || sel_m.get() == i;
                    let (bg, fg, bd) = if done {
                        (t.colors().primary, t.colors().on_primary, t.colors().primary)
                    } else {
                        (t.colors().surface, t.colors().on_surface_variant, t.colors().outline)
                    };
                    s.width(step_size)
                        .height(step_size)
                        .border_radius(9999.0)
                        .items_center()
                        .justify_center()
                        .background(bg.to_peniko())
                        .color(fg.to_peniko())
                        .border(1.0)
                        .border_color(bd.to_peniko())
                        .font_size(14.0)
                        .font_weight(FontWeight::Medium.to_floem())
                });

                let theme_l = theme;
                let sel_l = selected;
                let label_view = Label::new(label.clone()).style(move |s| {
                    let t = theme_l.get();
                    let c = if sel_l.get() == i {
                        t.colors().on_surface
                    } else if sel_l.get() > i {
                        t.colors().primary
                    } else {
                        t.colors().on_surface_variant
                    };
                    let w = if sel_l.get() == i {
                        FontWeight::Medium
                    } else {
                        FontWeight::Regular
                    };
                    s.font_size(label_font)
                        .color(c.to_peniko())
                        .font_weight(w.to_floem())
                });

                let node: AnyView = Stack::vertical((marker, label_view))
                    .style(move |s| s.items_center().gap(8.0))
                    .into_any();

                let clickable = editable && i < sel || non_linear;
                let node: AnyView = if clickable {
                    let theme_k = theme;
                    let on_change_k = on_change.clone();
                    let sel_k = selected;
                    node.on_event(listener::Click, move |_, _| {
                        sel_k.set(i);
                        if let Some(cb) = on_change_k.as_ref() {
                            cb(i);
                        }
                        EventPropagation::Continue
                    })
                    .style(move |s| {
                        let _ = theme_k;
                        s.cursor(CursorStyle::Pointer)
                    })
                    .into_any()
                } else {
                    node
                };
                kids.push(node);

                if i < n - 1 {
                    let theme_c = theme;
                    let conn = Container::new(Empty::new()).style(move |s| {
                        let t = theme_c.get();
                        let done = i < sel;
                        s.height(connector_width)
                            .flex_grow(1.0)
                            .min_width(16.0)
                            .background(
                                if done {
                                    t.colors().primary
                                } else {
                                    t.colors().outline_variant
                                }
                                .to_peniko(),
                            )
                    });
                    kids.push(conn.into_any());
                }
            }

            if kids.is_empty() {
                return Empty::new().into_any();
            }
            Stack::horizontal_from_iter(kids)
                .style(move |s| s.items_center().gap(gap).width_pct(100.0))
                .into_any()
        });

        Container::new(child)
            .style(move |s| s.width_pct(100.0).padding_horiz(8.0).padding_vert(12.0))
    }
}

impl Default for MdStepper {
    fn default() -> Self {
        Self::new()
    }
}
