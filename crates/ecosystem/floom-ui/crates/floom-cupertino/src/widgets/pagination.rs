//! Cupertino pagination, ported to the Floom 2.0 widget API.
//!

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

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// The previous arrow's glyph.
const PREV: &str = "\u{2039}";
/// The next arrow's glyph.
const NEXT: &str = "\u{203a}";

/// The pagination's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoPaginationStyle {
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

/// Cupertino's pagination.
pub struct CupertinoPagination {
    props: PaginationProps,
    theme: RwSignal<MultiTheme>,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// v1's props held a `Box<dyn Fn(usize)>`, which cannot cross a slot
    /// boundary: 2.0's slots are shared between threads.
    on_change: Option<Callback>,
}

impl CupertinoPagination {
    pub fn new() -> Self {
        Self {
            props: PaginationProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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
        // Stored twice, from one closure: the props keep v1's
        // `Box<dyn Fn(usize)>` and the engine's slot gets the shareable one.
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

    pub fn resolved_style(&self) -> CupertinoPaginationStyle {
        let theme = self.theme.get();
        let ComponentTokens::Pagination(tokens) = theme.component(ComponentId::Pagination) else {
            unreachable!("ComponentId::Pagination must resolve to PaginationTokens")
        };
        CupertinoPaginationStyle {
            design_system: "cupertino",
            selected_bg: theme.colors().primary,
            selected_fg: theme.colors().on_primary,
            // v1's unselected page: a transparent fill.
            unselected_bg: Color::TRANSPARENT,
            unselected_fg: theme.colors().on_surface,
            arrow_fg: theme.colors().on_surface,
            // v1's disabled arrow: the same ink at alpha 38.
            disabled_arrow_fg: with_alpha(theme.colors().on_surface, 38),
            button_size: tokens.button_size,
            gap: tokens.gap,
            radius: theme.radius().md,
        }
    }

    /// Build a Cupertino pagination control (prev / numbered / next).
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        // v1 clamped the page to the pages there are, and kept at least one.
        let total = self.props.total.max(1);
        let current = self.props.current.clamp(1, total);
        let on_change = self.on_change.clone();

        let button_size = style.button_size;
        let radius = style.radius;

        // v1's `list(controls)`: a column. See the module's note on the column
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
                // The slot is a replayed `Fn`, so each control it holds gets a
                // copy of the callback rather than a borrow of the one
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
                    // v1's `set_disabled(true)`.
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
                    // v1's `set_disabled(true)`.
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

        // v1's container: a row that held the column, with the gap that had
        Stack::new(Axis::Horizontal)
            .gap(style.gap)
            // v1's `items_center()`.
            .align(Align::Start, Align::Center)
            .push(move || Box::new(column.clone()))
    }
}

impl Default for CupertinoPagination {
    fn default() -> Self {
        Self::new()
    }
}

/// One control: the square box, and the label inside it.
///
/// `fill` is `None` for the arrows -- v1 set a colour on them, which in v1
/// was the *text* colour, and left their box transparent.
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
                // v1's `items_center()` and `justify_center()`.
                .main_align(Align::Center)
                .cross_align(Align::Center),
        )
        .radius(radius);
    if let Some(fill) = fill {
        box_ = box_.background(to_paint(fill));
    }
    box_.push(move || Box::new(Text::new(label.clone()).color(ink)))
}
