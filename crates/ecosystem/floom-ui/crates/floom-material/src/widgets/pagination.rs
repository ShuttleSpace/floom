use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Axis, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::pagination::PaginationProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The previous arrow's glyph.
const PREV: &str = "\u{2039}";
/// The next arrow's glyph.
const NEXT: &str = "\u{203a}";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialPaginationStyle {
    pub design_system: &'static str,
    /// The selected page's fill.
    pub selected_bg: Color,
    /// The selected page's ink.
    pub selected_fg: Color,
    /// The other pages' fill: nothing at all.
    pub unselected_bg: Color,
    /// The other pages' ink.
    pub unselected_fg: Color,
    /// The arrows' ink, when the arrow answers.
    pub arrow_fg: Color,
    /// The arrows' ink, when the arrow cannot answer.
    pub disabled_arrow_fg: Color,
    pub button_size: f32,
    pub gap: f32,
    pub radius: f32,
}

/// Material's pagination.
pub struct MaterialPagination {
    props: PaginationProps,
    theme: RwSignal<MultiTheme>,
    on_change: Option<Callback>,
}

impl MaterialPagination {
    pub fn new() -> Self {
        Self {
            props: PaginationProps::default(),
            theme: theme_signal(),
            on_change: None,
        }
    }

    /// How many pages there are.
    pub fn total(mut self, total: usize) -> Self {
        self.props.total = total;
        self
    }

    /// The page that is chosen. One-based, and clamped to the pages there are.
    pub fn current(mut self, current: usize) -> Self {
        self.props.current = current;
        self
    }

    /// Called with the newly chosen page whenever a control answers.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        let shared: Callback = Arc::new(callback);
        self.props.on_change = Some(Box::new({
            let shared = shared.clone();
            move |index| shared(index)
        }));
        self.on_change = Some(shared);
        self
    }

    pub fn props(&self) -> &PaginationProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialPaginationStyle {
        let theme = self.theme.get();
        let ComponentTokens::Pagination(tokens) = theme.component(ComponentId::Pagination) else {
            unreachable!("ComponentId::Pagination must resolve to PaginationTokens")
        };
        MaterialPaginationStyle {
            design_system: "material",
            selected_bg: theme.colors().primary,
            selected_fg: theme.colors().on_primary,
            unselected_bg: Color::TRANSPARENT,
            unselected_fg: theme.colors().on_surface,
            arrow_fg: theme.colors().on_surface,
            disabled_arrow_fg: with_alpha(theme.colors().on_surface, 38),
            button_size: tokens.button_size,
            gap: tokens.gap,
            radius: theme.radius().md,
        }
    }

    /// Build a Material 3 pagination control (prev / numbered / next).
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let total = self.props.total.max(1);
        let current = self.props.current.clamp(1, total);
        let on_change = self.on_change.clone();

        let button_size = style.button_size;
        let radius = style.radius;

        let mut column = Stack::new(Axis::Vertical);

        {
            let enabled = current > 1;
            let on_change = on_change.clone();
            let ink = if enabled {
                style.arrow_fg
            } else {
                style.disabled_arrow_fg
            };
            column = column.push(move || {
                let on_change = on_change.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || {
                            Box::new(control_box(
                                PREV.to_string(),
                                None,
                                ink,
                                button_size,
                                radius,
                            ))
                        },
                    ))
                    .label("Previous page")
                    .disabled(!enabled)
                    .on_click(move || {
                        if enabled {
                            if let Some(cb) = &on_change {
                                cb(current - 1);
                            }
                        }
                    }),
                )
            });
        }

        for page in 1..=total {
            let selected = page == current;
            let on_change = on_change.clone();
            let (fill, ink) = if selected {
                (style.selected_bg, style.selected_fg)
            } else {
                (style.unselected_bg, style.unselected_fg)
            };
            let label = page.to_string();
            column = column.push(move || {
                let on_change = on_change.clone();
                let label = label.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || {
                            Box::new(control_box(
                                label.clone(),
                                Some(fill),
                                ink,
                                button_size,
                                radius,
                            ))
                        },
                    ))
                    .label(format!("Page {page}"))
                    .on_click(move || {
                        if let Some(cb) = &on_change {
                            cb(page);
                        }
                    }),
                )
            });
        }

        {
            let enabled = current < total;
            let on_change = on_change.clone();
            let ink = if enabled {
                style.arrow_fg
            } else {
                style.disabled_arrow_fg
            };
            column = column.push(move || {
                let on_change = on_change.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || {
                            Box::new(control_box(
                                NEXT.to_string(),
                                None,
                                ink,
                                button_size,
                                radius,
                            ))
                        },
                    ))
                    .label("Next page")
                    .disabled(!enabled)
                    .on_click(move || {
                        if enabled {
                            if let Some(cb) = &on_change {
                                cb(current + 1);
                            }
                        }
                    }),
                )
            });
        }

        Stack::new(Axis::Horizontal)
            .gap(style.gap)
            .align(Align::Start, Align::Center)
            .push(move || Box::new(column.clone()))
    }
}

impl Default for MaterialPagination {
    fn default() -> Self {
        Self::new()
    }
}

fn control_box(
    label: String,
    fill: Option<Color>,
    ink: Color,
    button_size: f32,
    radius: f32,
) -> Surface {
    let ink = to_paint(ink);
    let mut box_ = Surface::new()
        .request(
            LayoutRequest::new()
                .width(button_size)
                .height(button_size)
                .main_align(Align::Center)
                .cross_align(Align::Center),
        )
        .radius(radius);
    if let Some(fill) = fill {
        box_ = box_.background(to_paint(fill));
    }
    box_.push(move || Box::new(Text::new(label.clone()).color(ink)))
}
