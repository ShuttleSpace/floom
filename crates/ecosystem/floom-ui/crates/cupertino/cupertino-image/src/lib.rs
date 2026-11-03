//! Cupertino image, ported to the Floom 2.0 widget API.
//!

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

/// The placeholder's size. v1 wrote none, so the label was drawn at v1's
/// default.
const PLACEHOLDER_FONT_SIZE: f32 = 14.0;

/// The image's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoImageStyle {
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
pub struct CupertinoImage {
    props: ImageProps,
    theme: RwSignal<MultiTheme>,
    /// The picture to draw, if the caller registered one.
    resource: Option<ResourceId>,
    /// The picture's own size, which the engine needs to scale it.
    intrinsic: Size,
}

impl CupertinoImage {
    pub fn new() -> Self {
        Self {
            props: ImageProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            resource: None,
            intrinsic: Size::ZERO,
        }
    }

    /// The source v1 named.
    ///
    /// Recorded, not loaded: see the module documentation. 2.0's image is
    /// drawn from a resource the host registered, so this only says what the
    /// source was.
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

    /// The picture to draw.
    ///
    /// The resource is the host's: register the bytes with
    /// `WidgetHost::register_image_decoded` (or `register_image`) and hand the
    /// id it returns to this builder. Without one, the kit draws v1's
    /// placeholder.
    pub fn resource(mut self, resource: ResourceId) -> Self {
        self.resource = Some(resource);
        self
    }

    /// The picture's own size, in logical pixels.
    ///
    /// The engine scales the picture into the box it is given, and it needs
    /// the picture's own size to do that: without it, `Cover` and `Contain`
    /// both mean "fill the box". v1 knew the size because it read the bytes.
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

    pub fn resolved_style(&self) -> CupertinoImageStyle {
        let theme = self.theme.get();
        let ComponentTokens::Image(tokens) = theme.component(ComponentId::Image) else {
            unreachable!("ComponentId::Image must resolve to ImageTokens")
        };
        CupertinoImageStyle {
            design_system: "cupertino",
            corner_radius: tokens.corner_radius,
            placeholder_color: tokens.placeholder_color,
            // v1's placeholder was `on_surface_variant` at 180/255.
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
            // v1's `.items_center().justify_center()`.
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
            // v1 rounded the box only when the image was rounded.
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

impl Default for CupertinoImage {
    fn default() -> Self {
        Self::new()
    }
}
