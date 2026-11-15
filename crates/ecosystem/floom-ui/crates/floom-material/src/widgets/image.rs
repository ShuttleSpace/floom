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
use floom_widget_runtime::theme_signal;

const PLACEHOLDER_FONT_SIZE: f32 = 14.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialImageStyle {
    pub design_system: &'static str,
    /// The corner radius, when the image is rounded.
    pub corner_radius: f32,
    /// The fill behind the picture.
    pub placeholder_color: Color,
    /// The placeholder text's ink.
    pub alt_color: Color,
    /// The placeholder text's size.
    pub placeholder_font_size: f32,
    /// The box's width, if the caller gave one.
    pub width: Option<f32>,
    /// The box's height, if the caller gave one.
    pub height: Option<f32>,
    /// Whether the box's corners are rounded.
    pub rounded: bool,
    /// Whether the picture covers its box rather than fitting inside it.
    pub cover: bool,
}

/// Material's image.
pub struct MaterialImage {
    props: ImageProps,
    theme: RwSignal<MultiTheme>,
    /// The picture to draw, if the caller registered one.
    resource: Option<ResourceId>,
    /// The picture's own size, which the engine needs to scale it.
    intrinsic: Size,
}

impl MaterialImage {
    pub fn new() -> Self {
        Self {
            props: ImageProps::default(),
            theme: theme_signal(),
            resource: None,
            intrinsic: Size::ZERO,
        }
    }

    pub fn src(mut self, src: impl Into<String>) -> Self {
        self.props.src = src.into();
        self
    }

    /// The alternative text: the placeholder's caption, and the picture's
    /// accessible name.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.props.alt = alt.into();
        self
    }

    /// The box's width.
    pub fn width(mut self, width: f32) -> Self {
        self.props.width = Some(width);
        self
    }

    /// The box's height.
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// Whether the box's corners are rounded.
    pub fn rounded(mut self, rounded: bool) -> Self {
        self.props.rounded = rounded;
        self
    }

    /// Whether the picture covers its box rather than fitting inside it.
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

    pub fn resolved_style(&self) -> MaterialImageStyle {
        let theme = self.theme.get();
        let ComponentTokens::Image(tokens) = theme.component(ComponentId::Image) else {
            unreachable!("ComponentId::Image must resolve to ImageTokens")
        };
        MaterialImageStyle {
            design_system: "material",
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

    /// The image: the box, and the picture or the placeholder inside it.
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

impl Default for MaterialImage {
    fn default() -> Self {
        Self::new()
    }
}
