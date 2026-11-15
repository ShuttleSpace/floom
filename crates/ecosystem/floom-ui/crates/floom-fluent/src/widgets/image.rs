//! Fluent image, ported to the Floom 2.0 widget API.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, ResourceId, Size};
use floom_widgets::{Align, Image, ImageFit, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::image::ImageProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const PLACEHOLDER_FONT_SIZE: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentImageStyle {
    pub design_system: &'static str,
    pub corner_radius: f32,
    pub placeholder_color: Color,
    pub alt_color: Color,
    pub placeholder_font_size: f32,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub rounded: bool,
    pub cover: bool,
}

pub struct FluentImage {
    props: ImageProps,
    theme: RwSignal<MultiTheme>,
    resource: Option<ResourceId>,
    intrinsic: Size,
}

impl FluentImage {
    pub fn new() -> Self {
        Self {
            props: ImageProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            resource: None,
            intrinsic: Size::ZERO,
        }
    }

    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.props.src = src.into();
        self
    }

    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.props.alt = alt.into();
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

    pub fn rounded(mut self, rounded: bool) -> Self {
        self.props.rounded = rounded;
        self
    }

    pub fn cover(mut self, cover: bool) -> Self {
        self.props.cover = cover;
        self
    }

    pub fn resource(mut self, resource: ResourceId) -> Self {
        self.resource = Some(resource);
        self
    }

    pub fn intrinsic_size(mut self, size: Size) -> Self {
        self.intrinsic = size;
        self
    }

    pub fn props(&self) -> &ImageProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentImageStyle {
        let theme = self.theme.get();
        let ComponentTokens::Image(tokens) = theme.component(ComponentId::Image) else {
            unreachable!("ComponentId::Image must resolve to ImageTokens")
        };
        FluentImageStyle {
            design_system: "fluent",
            corner_radius: tokens.corner_radius,
            placeholder_color: tokens.placeholder_color,
            alt_color: with_alpha(theme.colors().on_surface_variant, 180),
            placeholder_font_size: PLACEHOLDER_FONT_SIZE,
            width: self.props.width,
            height: self.props.height,
            rounded: self.props.rounded,
            cover: self.props.cover,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();

        let mut request = LayoutRequest::new()
            .main_align(Align::Center)
            .cross_align(Align::Center);
        if let Some(width) = style.width {
            request = request.width(width);
        }
        if let Some(height) = style.height {
            request = request.height(height);
        }

        let alt = self.props.alt.clone();
        let resource = self.resource;
        let intrinsic = self.intrinsic;
        let cover = style.cover;
        let alt_color = to_paint(style.alt_color);
        let placeholder_font_size = style.placeholder_font_size;

        Surface::new()
            .request(request)
            .background(to_paint(style.placeholder_color))
            .radius(if style.rounded {
                style.corner_radius
            } else {
                0.0
            })
            .push(move || match resource {
                Some(resource) => Box::new(
                    Image::new(resource)
                        .fit(if cover {
                            ImageFit::Cover
                        } else {
                            ImageFit::Contain
                        })
                        .intrinsic_size(intrinsic)
                        .alt(alt.clone()),
                ),
                None => Box::new(
                    Text::new(alt.clone())
                        .size(placeholder_font_size)
                        .color(alt_color),
                ),
            })
    }
}

impl Default for FluentImage {
    fn default() -> Self {
        Self::new()
    }
}
