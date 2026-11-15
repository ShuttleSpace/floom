//! Fluent skeleton loader, ported to the Floom 2.0 widget API.

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::skeleton_loader::{SkeletonLoaderProps, SkeletonType};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const CIRCLE_RADIUS: f32 = 9999.0;
const RECT_WIDTH: f32 = 160.0;
const CARD_WIDTH: f32 = 200.0;
const CARD_HEIGHT: f32 = 120.0;
const DEFAULT_OUTER_WIDTH: f32 = 240.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSkeletonStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub outer_width: f32,
    pub corner_radius: f32,
    pub text_height: f32,
    pub avatar_size: f32,
    pub gap: f32,
    pub card_radius: f32,
}

pub struct FluentSkeletonLoader {
    props: SkeletonLoaderProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentSkeletonLoader {
    pub fn new() -> Self {
        Self {
            props: SkeletonLoaderProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn skeleton_type(mut self, skeleton_type: SkeletonType) -> Self {
        self.props.skeleton_type = skeleton_type;
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.props.width = Some(width);
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    pub fn lines(mut self, lines: usize) -> Self {
        self.props.lines = lines;
        self
    }

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

    pub fn resolved_style(&self) -> FluentSkeletonStyle {
        let theme = self.theme.get();
        let ComponentTokens::SkeletonLoader(tokens) = theme.component(ComponentId::SkeletonLoader)
        else {
            unreachable!("ComponentId::SkeletonLoader must resolve to SkeletonTokens")
        };
        FluentSkeletonStyle {
            design_system: "fluent",
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

impl Default for FluentSkeletonLoader {
    fn default() -> Self {
        Self::new()
    }
}

fn outer_width(props: &SkeletonLoaderProps) -> f32 {
    match props.skeleton_type {
        SkeletonType::Card => props.width.unwrap_or(CARD_WIDTH),
        SkeletonType::Rect => props.width.unwrap_or(RECT_WIDTH),
        _ => props.width.unwrap_or(DEFAULT_OUTER_WIDTH),
    }
}

fn content_of(props: &SkeletonLoaderProps, style: &FluentSkeletonStyle) -> Box<dyn WidgetAny> {
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
