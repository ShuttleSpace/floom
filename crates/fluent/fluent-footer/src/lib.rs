use floem::prelude::*;
use floem::view::AnyView;
use floem_core_theme::props::footer::FooterProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFooterStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub pad: f32,
    pub bordered: bool,
    pub elevated: bool,
}

pub struct FluentFooter {
    props: FooterProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentFooter {
    pub fn new() -> Self {
        Self {
            props: FooterProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            child: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.props.bordered = bordered;
        self
    }

    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = pad;
        self
    }

    /// Supply the footer's content.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    pub fn props(&self) -> &FooterProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> FluentFooterStyle {
        let theme = self.theme.get();
        FluentFooterStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            height: self.props.height,
            pad: self.props.pad,
            bordered: self.props.bordered,
            elevated: self.props.elevation > 0,
        }
    }

    /// Build a bottom application bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child.unwrap_or_else(|| empty().into_any());
        let height = props.height;
// layout-provider auto-register
        floem_core_theme::layout::register_layout(
            floem_core_theme::layout::LayoutSlot::Footer,
            height,
        );

        let pad = props.pad;
        let bordered = props.bordered;
        let elevation = props.elevation;

        Container::new(child).style(move |s| {
            let t = theme.get();
            let mut s = s
                .width_pct(100.0)
                .height(height)
                .padding(pad)
                .items_center()
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko());
            if bordered {
                s = s
                    .border_color(t.colors().outline_variant.to_peniko())
                    .border_top(1.0);
            }
            if elevation > 0 {
                s = s
                    .box_shadow_blur(6.0)
                    .box_shadow_spread(0.0)
                    .box_shadow_color(with_alpha(BLACK, 30).to_peniko());
            }
            s
        })
    }
}

impl Default for FluentFooter {
    fn default() -> Self {
        Self::new()
    }
}
