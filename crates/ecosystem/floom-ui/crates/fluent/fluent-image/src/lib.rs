//! Fluent image, ported to the Floom 2.0 widget API.
//!
//! An image in a box: the picture, or -- when there is no picture -- the alt
//! text on the placeholder fill, in a box that may be rounded and may have a
//! size of its own.
//!
//! ## The gap: 2.0 has no in-kit image loading
//!
//! v1's image named a **source**: a local file path or an `http(s)://` URL.
//! The kit read the file itself (`std::fs::read`), handed the bytes to
//! v1's `img` view, and fell back to the placeholder for a URL or a missing
//! file.
//!
//! 2.0's `Image` widget draws a **resource id**, and only the host can mint
//! one: `WidgetHost::register_image_decoded` decodes the bytes and interns
//! them in the registry the renderer reads from. A kit is below the host and
//! has no way to reach that registry, so:
//!
//! * the port takes the resource the **caller** registered -- see
//!   [`FluentImage::resource`] -- and the caller also says how big the
//!   picture is -- see [`FluentImage::intrinsic_size`], which is what the
//!   engine's `Cover` and `Contain` need to scale it;
//! * there is **no network**: an `http(s)://` source has no 2.0 equivalent at
//!   all, because 2.0 has no HTTP stack;
//! * the **missing picture** case is reproduced: with no resource, the kit
//!   draws v1's placeholder.
//!
//! [`FluentImage::src`] is kept so the props keep reporting the source v1
//! named, but it records it and nothing else: it loads nothing and draws
//! nothing.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for the box, the engine's `Image`
//! for the picture and a `Text` for the placeholder.
//!
//! ## What this kit answers
//!
//! * the corner radius and the placeholder fill: the tokens;
//! * the placeholder's ink: `on_surface_variant` at 180/255;
//! * the placeholder's size: 14, which is v1's default label size;
//! * the box's size: the caller's, if it gave one;
//! * the corner radius: the token's, but only when the image is rounded.
//!
//! ## The style the port adds
//!
//! v1's `MdImageStyle` reported the corner radius, the placeholder fill, the
//! size and the rounded flag -- and nothing else, although the paint also used
//! whether the image covered its box, the placeholder's ink and its size. The
//! port reports all of them, and names the design system, which v1's style
//! did not.
//!
//! ## The theme source
//!
//! v1's Fluent image took a **snapshot** of its own design system's theme
//! rather than reading the app-wide signal, so an app-wide theme change
//! does not re-theme it. The port keeps that difference.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The box's own ink.** v1 set `.color(on_surface_variant)` on the box,
//!   which is the ink any text inside it would inherit. 2.0's `Surface` paints
//!   a fill and only the child knows it is text, and the placeholder carries
//!   its own ink, so the box's ink has nowhere to go.
//! * **The corner radius on the picture.** v1 rounded the *box*; the picture
//!   inside it was not clipped. 2.0 rounds the fill the same way, and the
//!   picture is not clipped either -- the engine's clip is a separate
//!   declaration, and this kit does not make one.

use floom_core::LayoutRequest;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, ResourceId, Size};
use floom_widgets::{Align, Image, ImageFit, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::image::ImageProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// The placeholder's size. v1 wrote none, so the label was drawn at v1's
/// default.
const PLACEHOLDER_FONT_SIZE: f32 = 14.0;

/// The image's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentImageStyle {
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
pub struct FluentImage {
    props: ImageProps,
    theme: RwSignal<MultiTheme>,
    /// The picture to draw, if the caller registered one.
    resource: Option<ResourceId>,
    /// The picture's own size, which the engine needs to scale it.
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

    pub fn resolved_style(&self) -> FluentImageStyle {
        let theme = self.theme.get();
        let ComponentTokens::Image(tokens) = theme.component(ComponentId::Image) else {
            unreachable!("ComponentId::Image must resolve to ImageTokens")
        };
        FluentImageStyle {
            design_system: "fluent",
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
                        // v1's `object_fit`.
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

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
