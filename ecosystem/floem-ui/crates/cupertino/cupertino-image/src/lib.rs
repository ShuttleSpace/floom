use floem::prelude::*;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::image::ImageProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoImageStyle {
    pub corner_radius: f32,
    pub placeholder_color: Color,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub rounded: bool,
}

pub struct CupertinoImage {
    props: ImageProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoImage {
    pub fn new() -> Self {
        Self {
            props: ImageProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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
            corner_radius: tokens.corner_radius,
            placeholder_color: tokens.placeholder_color,
            width: self.props.width,
            height: self.props.height,
            rounded: self.props.rounded,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let tokens = match theme.get().component(ComponentId::Image) {
            ComponentTokens::Image(t) => t,
            _ => unreachable!(),
        };
        let corner_radius = if props.rounded { tokens.corner_radius } else { 0.0 };
        let placeholder = tokens.placeholder_color;
        let bytes = load_image_bytes(&props.src);
        let content: AnyView = if let Some(b) = bytes {
            img(move || b.clone())
                .style(move |s| {
                    s.object_fit(if props.cover {
                        floem::style::ObjectFit::Cover
                    } else {
                        floem::style::ObjectFit::Contain
                    })
                })
                .into_any()
        } else {
            Label::new(props.alt.clone())
                .style(move |s| {
                    let t = theme.get();
                    s.color(with_alpha(t.colors().on_surface_variant, 180).to_peniko())
                })
                .into_any()
        };
        Container::new(content).style(move |s| {
            let t = theme.get();
            let mut s = s
                .background(placeholder.to_peniko())
                .color(t.colors().on_surface_variant.to_peniko())
                .items_center()
                .justify_center();
            if let Some(w) = props.width {
                s = s.width(w);
            }
            if let Some(h) = props.height {
                s = s.height(h);
            }
            if props.rounded {
                s = s.border_radius(corner_radius);
            }
            s
        })
    }
}

impl Default for CupertinoImage {
    fn default() -> Self {
        Self::new()
    }
}

fn load_image_bytes(src: &str) -> Option<Vec<u8>> {
    if src.starts_with("http://") || src.starts_with("https://") {
        return None;
    }
    std::fs::read(src).ok()
}
