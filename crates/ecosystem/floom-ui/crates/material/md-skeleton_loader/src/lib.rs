
use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::skeleton_loader::{SkeletonLoaderProps, SkeletonType};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSkeletonStyle {
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

/// Material's skeleton loader.
pub struct MaterialSkeletonLoader {
    props: SkeletonLoaderProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialSkeletonLoader {
    pub fn new() -> Self {
        Self {
            props: SkeletonLoaderProps::default(),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MaterialSkeletonStyle {
        let theme = self.theme.get();
        let ComponentTokens::SkeletonLoader(tokens) = theme.component(ComponentId::SkeletonLoader)
        else {
            unreachable!("ComponentId::SkeletonLoader must resolve to SkeletonTokens")
        };
        MaterialSkeletonStyle {
            design_system: "material",
            color: tokens.color,
            outer_width: outer_width(&self.props),
            corner_radius: tokens.corner_radius,
            text_height: tokens.text_height,
            avatar_size: tokens.avatar_size,
            gap: tokens.gap,
            card_radius: tokens.card_radius,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let props = self.props;
        Surface::new()
            .request(
                LayoutRequest::new()
                    .width(style.outer_width)
                    .cross_align(Align::Start),
            )
            .push(move || content_of(&props, &style))
    }
}

impl Default for MaterialSkeletonLoader {
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
fn content_of(props: &SkeletonLoaderProps, style: &MaterialSkeletonStyle) -> Box<dyn WidgetAny> {
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
            let lines = props.lines.max(1);
            let mut column = Stack::new(Axis::Vertical)
                .request(LayoutRequest::new().width_pct(100.0));
            for _ in 0..lines {
                column = column.push({
                    let fill = fill;
                    let text_height = style.text_height;
                    let corner_radius = style.corner_radius;
                    move || {
                        Box::new(
                            Surface::new()
                                .request(LayoutRequest::new().width_pct(100.0).height(text_height))
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
                                .request(LayoutRequest::new().width_pct(100.0).height(text_height))
                                .background(fill)
                                .radius(corner_radius),
                        )
                    }
                });
            }
            Box::new(
                Stack::new(Axis::Horizontal)
                    .gap(style.gap)
                    .align(Align::Start, Align::Center)
                    .request(LayoutRequest::new().width_pct(100.0))
                    .push(move || Box::new(circle.clone()))
                    .push_layout(LayoutRequest::new().grow(1.0), move || {
                        Box::new(column.clone())
                    }),
            )
        }
    }
}
