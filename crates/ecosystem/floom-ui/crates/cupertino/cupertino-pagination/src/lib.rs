//! Cupertino pagination, ported to the Floom 2.0 widget API.
//!
//! A pagination control is a set of page buttons: a previous arrow, one button
//! per page, and a next arrow.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Pressable`] around a [`Surface`] for each
//! control, a vertical `Stack` for the column they are laid out in, and
//! [`Text`] for the labels. Nothing here is a shape the engine does not
//! already have.
//!
//! ## What this kit answers
//!
//! * the selected page's fill: `primary`, and its ink: `on_primary`;
//! * the other pages' ink: `on_surface`, and their fill: nothing at all;
//! * the arrows' ink: `on_surface`, and the ink of an arrow that cannot
//!   answer: `on_surface` at alpha 38;
//! * the buttons' size: the token's `button_size`; their radius: the theme's
//!   `md`; the gap: the token's `gap`.
//!
//! ## The theme it reads
//!
//! Cupertino's theme is a *snapshot*: the kit takes a `MultiTheme` for
//! Cupertino when it is built and reads its tokens and colours from it.
//! The Material kit reads the app-wide signal instead, so a change of theme
//! re-themes the control with everything else.
//!
//! ## What the three kits disagree about
//!
//! Only the theme they read: the tokens, the colours and the literals above
//! are the same in all three, so the three kits are the same code under three
//! names, and what differs between them is the palette the theme hands out.
//!
//! ## The column v1 drew
//!
//! v1 built the controls with `list(controls)` and put that in a container
//! with `flex_row()`. `list(..)` is a *column* -- so the page buttons were
//! stacked **vertically**, not laid out side by side, and that is what v1
//! showed. The port reproduces the column, because v1 is the specification,
//! and the row around it is reproduced too, so the structure is the same.
//!
//! ## The gap that separated nothing
//!
//! v1's `gap(gap)` was written on the container that held the column, and
//! that container had exactly one child -- so the gap had nothing to
//! separate, and the buttons sat against each other. The port writes the same
//! gap on the same container, and it is as dead here as it was there.
//!
//! ## The style v1 under-reported
//!
//! v1's style reported the selected page's fill and ink, the other pages' ink
//! and the three sizes. The paint used three more colours: the unselected
//! page's fill, which is nothing at all, and the arrows' two inks. The port's
//! style reports all of them.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The list's selection.** v1's `list(..)` was a column *and* a
//!   selection widget: the arrow keys moved a selection through the controls,
//!   `Enter` accepted it, and the framework's default theme gave each control
//!   a padding, a radius and a hover and selected background. 2.0's `Stack`
//!   is a column and nothing else. The controls are still `Pressable`s, so
//!   they are still activatable by keyboard -- which the list's selection was
//!   not.
//! * **The pointer cursor.** v1 set `CursorStyle::Pointer` on the controls and
//!   `CursorStyle::Default` on a disabled arrow. 2.0's paint context has no
//!   cursor vocabulary.
//! * **The callback's shape.** v1's `PaginationProps::on_change` is
//!   `Option<Box<dyn Fn(usize)>>`, which cannot cross a `Send + Sync` slot
//!   boundary. The kit keeps its own `Arc<dyn Fn(usize) + Send + Sync>` and
//!   stores a v1-shaped copy in the props, so `props()` still reports one.
//! * **The accessors.** v1's kits did not agree on them: Material's pagination
//!   had no `props()`, `theme()` or `with_theme()`, and the other two had all
//!   three. The port gives all three kits the same three, because a kit that
//!   cannot be given a theme cannot be tested against one.
//!
//! ## The one thing the port adds
//!
//! v1 published nothing to assistive technology. 2.0's `Pressable` does, so
//! the port names each control: the arrows are "Previous page" and "Next
//! page", and a page button is "Page 3". That is a fix rather than a
//! translation, and it is named here rather than slipped in.

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
            unselected_bg: Color::rgba(0, 0, 0, 0),
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
        // v1 drew.
        let mut column = Stack::new(Axis::Vertical);

        // v1's previous arrow.
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
                // outside.
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

        // v1's page numbers.
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

        // v1's next arrow.
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
        // one child to separate and so separated nothing.
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
