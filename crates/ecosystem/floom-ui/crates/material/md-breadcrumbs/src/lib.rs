
use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet, SignalUpdate};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

const PADDING: f32 = 8.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialBreadcrumbsStyle {
    pub design_system: &'static str,
    /// The gap between the items and the dividers, from the token.
    pub gap: f32,
    /// The items' size, from the token.
    pub font_size: f32,
    /// The current item's ink.
    pub on_surface: Color,
    /// The other items' ink.
    pub on_surface_variant: Color,
    /// The divider's ink.
    pub outline_variant: Color,
    pub current_font_weight: FontWeight,
    pub item_font_weight: FontWeight,
    pub divider_font_weight: FontWeight,
    /// The divider's text, from the token.
    pub divider: String,
    pub padding: f32,
}

/// Material's breadcrumbs.
pub struct MaterialBreadcrumbs {
    theme: RwSignal<MultiTheme>,
    items: RwSignal<Vec<String>>,
    on_click: Option<Callback>,
}

impl MaterialBreadcrumbs {
    pub fn new() -> Self {
        Self {
            theme: theme_signal(),
            items: RwSignal::new(Vec::new()),
            on_click: None,
        }
    }

    /// The ordered breadcrumb labels, last item is the current location.
    pub fn items(self, items: Vec<String>) -> Self {
        self.items.set(items);
        self
    }

    /// Fired when a non-current item is clicked, passing its index.
    pub fn on_click(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialBreadcrumbsStyle {
        let theme = self.theme.get();
        let ComponentTokens::Breadcrumbs(tokens) = theme.component(ComponentId::Breadcrumbs) else {
            unreachable!("ComponentId::Breadcrumbs must resolve to BreadcrumbsTokens")
        };
        let c = theme.colors();
        MaterialBreadcrumbsStyle {
            design_system: "material",
            gap: tokens.gap,
            font_size: tokens.font_size,
            on_surface: c.on_surface,
            on_surface_variant: c.on_surface_variant,
            outline_variant: c.outline_variant,
            current_font_weight: FontWeight::Medium,
            item_font_weight: FontWeight::Normal,
            divider_font_weight: FontWeight::Normal,
            divider: tokens.divider,
            padding: PADDING,
        }
    }

    /// The trail: the padded box, and the row of items and dividers inside it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let items = self.items.get();
        let on_click = self.on_click.clone();

        let font_size = style.font_size;
        let count = items.len();

        let mut trail = Stack::new(Axis::Horizontal)
            .gap(style.gap)
            .align(Align::Start, Align::Center)
            .request(LayoutRequest::new().width_pct(100.0));

        for (index, label) in items.iter().enumerate() {
            let is_last = index + 1 == count;
            let ink = to_paint(if is_last {
                style.on_surface
            } else {
                style.on_surface_variant
            });
            let weight = if is_last {
                style.current_font_weight
            } else {
                style.item_font_weight
            };
            let text = label.clone();
            let accessible = label.clone();

            if is_last {
                trail = trail.push(move || {
                    Box::new(
                        Text::new(text.clone())
                            .size(font_size)
                            .weight(weight)
                            .color(ink),
                    )
                });
            } else {
                let on_click = on_click.clone();
                trail = trail.push(move || {
                    let on_click = on_click.clone();
                    let text = text.clone();
                    let accessible = accessible.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new(),
                            move || {
                                Box::new(
                                    Text::new(text.clone())
                                        .size(font_size)
                                        .weight(weight)
                                        .color(ink),
                                )
                            },
                        ))
                        .role(SemanticsRole::Link)
                        .label(accessible)
                        .on_click(move || {
                            if let Some(cb) = &on_click {
                                cb(index);
                            }
                        }),
                    )
                });
            }

            if !is_last {
                let divider = style.divider.clone();
                let divider_ink = to_paint(style.outline_variant);
                let divider_weight = style.divider_font_weight;
                trail = trail.push(move || {
                    Box::new(
                        Text::new(divider.clone())
                            .size(font_size)
                            .weight(divider_weight)
                            .color(divider_ink),
                    )
                });
            }
        }

        Surface::new()
            .request(LayoutRequest::new().width_pct(100.0))
            .padding_horiz(f64::from(style.padding))
            .padding_vert(f64::from(style.padding))
            .push(move || Box::new(trail.clone()))
    }
}

impl Default for MaterialBreadcrumbs {
    fn default() -> Self {
        Self::new()
    }
}
