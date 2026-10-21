//! Cupertino skeleton loader, ported to the Floom 2.0 widget API.
//!
//! A skeleton loader is the placeholder shown while content is loading: a
//! grey block, or a stack of grey lines, in the shape the content will take.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for each block, a vertical `Stack`
//! for a column of them, and a horizontal `Stack` for the avatar and its
//! lines. Nothing here is a shape the engine does not already have.
//!
//! ## What this kit answers
//!
//! * the blocks' fill: the token's `color`;
//! * a block's corner radius: the token's `corner_radius`, and a card's: the
//!   token's `card_radius`;
//! * a text line's height: the token's `text_height`; a circle's and the
//!   avatar's diameter: the token's `avatar_size`; the gap between the
//!   avatar and its lines: the token's `gap`.
//!
//! ## The theme it reads
//!
//! Cupertino's theme is a *snapshot*: the kit takes a `MultiTheme` for
//! Cupertino when it is built and reads its tokens and colours from it.
//! The Material kit reads the app-wide signal instead, so a change of theme
//! re-themes the placeholder with everything else.
//!
//! ## What the three kits disagree about
//!
//! Only the theme they read: the tokens, the colours and the literals above
//! are the same in all three, so the three kits are the same code under three
//! names, and what differs between them is the palette the theme hands out.
//!
//! ## The shimmer that is not there
//!
//! v1's `animated` prop asked for a shimmer, and v1's own comment said the
//! build had no animation frame hook, so nothing shimmered: the placeholder
//! was a steady block. 2.0 has no shimmer either, and this port says so
//! plainly rather than pretending: **the placeholder never shimmers.** The
//! `animated` prop and its setter are kept, because v1's props carried them,
//! and they change nothing about what is drawn.
//!
//! ## The style v1 under-reported
//!
//! v1's style reported the fill and the outer width and nothing else, though
//! the paint used six more numbers: the two corner radii, the line height,
//! the avatar's diameter and the gap. The port's style reports all of them,
//! because a style that does not say what the widget draws is not the widget's
//! output.
//!
//! ## The width the bars had no base for
//!
//! v1's text bars asked for `width_pct(100)` inside a column that shrank to
//! fit -- v1's `items_start()` on the outer box -- so the percentage had no
//! definite box to be a percentage of. The port gives the column the same
//! percentage of the outer box, so the bars are as wide as the box, which is
//! what v1's own numbers asked for.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The list.** v1 built its columns with `list(..)`, which is a column
//!   *and* a selection widget: the arrow keys moved a selection through the
//!   lines, `Enter` accepted it, and the framework's default theme gave each
//!   line a padding, a radius and a hover and selected background. 2.0's
//!   `Stack` is a column and nothing else. Nothing in a skeleton was ever
//!   clickable, so what is lost is a keyboard selection over grey blocks --
//!   and the default theme's dressing of a list item, which belonged to v1's
//!   app theme rather than to this kit.
//! * **The accessors.** v1's skeleton loaders had no `props()`, `theme()` or
//!   `with_theme()`. The port gives them all three, because a kit that cannot
//!   be given a theme cannot be tested against one.

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::skeleton_loader::{SkeletonLoaderProps, SkeletonType};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;

/// A circle's radius: 9999 is a circle whatever the size. A v1 literal.
const CIRCLE_RADIUS: f32 = 9999.0;
/// A rectangle's default width. A v1 literal.
const RECT_WIDTH: f32 = 160.0;
/// A card's default width. A v1 literal.
const CARD_WIDTH: f32 = 200.0;
/// A card's default height. A v1 literal.
const CARD_HEIGHT: f32 = 120.0;
/// The default outer width of a skeleton that is neither a card nor a
/// rectangle. A v1 literal.
const DEFAULT_OUTER_WIDTH: f32 = 240.0;

/// The skeleton's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoSkeletonStyle {
    pub design_system: &'static str,
    /// The placeholder's fill.
    pub color: Color,
    /// The outer box's width.
    pub outer_width: f32,
    /// The corner radius of a rectangle and of a text line.
    pub corner_radius: f32,
    /// The height of one text line.
    pub text_height: f32,
    /// The diameter of a circle and of an avatar.
    pub avatar_size: f32,
    /// The gap between the avatar and its lines, and between two lines.
    pub gap: f32,
    /// The corner radius of a card.
    pub card_radius: f32,
}

/// Cupertino's skeleton loader.
pub struct CupertinoSkeletonLoader {
    props: SkeletonLoaderProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSkeletonLoader {
    pub fn new() -> Self {
        Self {
            props: SkeletonLoaderProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// The shape of the placeholder.
    pub fn skeleton_type(mut self, skeleton_type: SkeletonType) -> Self {
        self.props.skeleton_type = skeleton_type;
        self
    }

    /// An explicit width, in place of the type's default.
    pub fn width(mut self, width: f32) -> Self {
        self.props.width = Some(width);
        self
    }

    /// An explicit height, in place of the type's default.
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// How many text lines.
    pub fn lines(mut self, lines: usize) -> Self {
        self.props.lines = lines;
        self
    }

    /// Whether the placeholder should shimmer.
    ///
    /// Kept for parity with v1's props. Nothing shimmers: see the module's
    /// note on the shimmer that is not there.
    pub fn animated(mut self, animated: bool) -> Self {
        self.props.animated = animated;
        self
    }

    pub fn props(&self) -> &SkeletonLoaderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoSkeletonStyle {
        let theme = self.theme.get();
        let ComponentTokens::SkeletonLoader(tokens) = theme.component(ComponentId::SkeletonLoader)
        else {
            unreachable!("ComponentId::SkeletonLoader must resolve to SkeletonTokens")
        };
        CupertinoSkeletonStyle {
            design_system: "cupertino",
            color: tokens.color,
            outer_width: outer_width(&self.props),
            corner_radius: tokens.corner_radius,
            text_height: tokens.text_height,
            avatar_size: tokens.avatar_size,
            gap: tokens.gap,
            card_radius: tokens.card_radius,
        }
    }

    /// Build a static skeleton placeholder.
    ///
    /// `animated` is accepted for API parity; this Floom build has no animation
    /// frame hook, so the placeholder is rendered as a steady block.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let props = self.props;
        Surface::new()
            .request(
                LayoutRequest::new()
                    .width(style.outer_width)
                    // v1's `.items_start()`.
                    .cross_align(Align::Start),
            )
            .push(move || content_of(&props, &style))
    }
}

impl Default for CupertinoSkeletonLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// The outer box's width: the caller's, or the type's own default.
fn outer_width(props: &SkeletonLoaderProps) -> f32 {
    match props.skeleton_type {
        SkeletonType::Card => props.width.unwrap_or(CARD_WIDTH),
        SkeletonType::Rect => props.width.unwrap_or(RECT_WIDTH),
        _ => props.width.unwrap_or(DEFAULT_OUTER_WIDTH),
    }
}

/// The placeholder itself: the block, or the column of blocks.
fn content_of(props: &SkeletonLoaderProps, style: &CupertinoSkeletonStyle) -> Box<dyn WidgetAny> {
    let fill = to_paint(style.color);
    match props.skeleton_type {
        SkeletonType::Circle => {
            let size = props.width.or(props.height).unwrap_or(style.avatar_size);
            Box::new(
                Surface::new()
                    .request(LayoutRequest::new().width(size).height(size))
                    .background(fill)
                    .radius(CIRCLE_RADIUS),
            )
        }
        SkeletonType::Rect => {
            let width = props.width.unwrap_or(RECT_WIDTH);
            let height = props.height.unwrap_or(style.text_height * 2.0);
            Box::new(
                Surface::new()
                    .request(LayoutRequest::new().width(width).height(height))
                    .background(fill)
                    .radius(style.corner_radius),
            )
        }
        SkeletonType::Card => {
            let width = props.width.unwrap_or(CARD_WIDTH);
            let height = props.height.unwrap_or(CARD_HEIGHT);
            Box::new(
                Surface::new()
                    .request(LayoutRequest::new().width(width).height(height))
                    .background(fill)
                    .radius(style.card_radius),
            )
        }
        SkeletonType::Text => {
            // v1's `list(bars)`: a column of lines.
            let lines = props.lines.max(1);
            let mut column = Stack::new(Axis::Vertical)
                // The bars are a percentage of this column, so the column
                // needs a definite width: see the module's note.
                .request(LayoutRequest::new().width_pct(100.0));
            for _ in 0..lines {
                column = column.push({
                    let fill = fill;
                    let text_height = style.text_height;
                    let corner_radius = style.corner_radius;
                    move || {
                        Box::new(
                            Surface::new()
                                .request(
                                    LayoutRequest::new()
                                        .width_pct(100.0)
                                        .height(text_height),
                                )
                                .background(fill)
                                .radius(corner_radius),
                        )
                    }
                });
            }
            Box::new(column)
        }
        SkeletonType::Avatar => {
            let circle = Surface::new()
                .request(
                    LayoutRequest::new()
                        .width(style.avatar_size)
                        .height(style.avatar_size),
                )
                .background(fill)
                .radius(CIRCLE_RADIUS);
            // v1: at most three lines beside the avatar.
            let lines = props.lines.max(1).min(3);
            let mut column = Stack::new(Axis::Vertical).gap(style.gap);
            for _ in 0..lines {
                column = column.push({
                    let fill = fill;
                    let text_height = style.text_height;
                    let corner_radius = style.corner_radius;
                    move || {
                        Box::new(
                            Surface::new()
                                .request(
                                    LayoutRequest::new()
                                        .width_pct(100.0)
                                        .height(text_height),
                                )
                                .background(fill)
                                .radius(corner_radius),
                        )
                    }
                });
            }
            // v1's `Stack::horizontal((circle, right))`: the circle, the lines
            // beside it, and the gap between them.
            Box::new(
                Stack::new(Axis::Horizontal)
                    .gap(style.gap)
                    // v1's `.items_center()`.
                    .align(Align::Start, Align::Center)
                    .request(LayoutRequest::new().width_pct(100.0))
                    .push(move || Box::new(circle.clone()))
                    // The column takes the room the circle left, so the bars
                    // have a definite width to be a percentage of.
                    .push_layout(LayoutRequest::new().grow(1.0), move || {
                        Box::new(column.clone())
                    }),
            )
        }
    }
}

