use floom::prelude::*;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, FooterTokens};
use floom_core_theme::props::footer::FooterProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdFooterStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub pad: f32,
    pub bordered: bool,
    pub elevated: bool,
}

pub struct MdFooter {
    props: FooterProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl MdFooter {
    pub fn new() -> Self {
        Self {
            props: FooterProps::default(),
            theme: theme_signal(),
            child: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
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
        self.props.pad = Some(pad);
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

    pub fn resolved_style(&self) -> MdFooterStyle {
        let theme = self.theme.get();
        let tokens = footer_tokens(&theme);
        MdFooterStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            height: self.props.height.unwrap_or(tokens.height),
            pad: self.props.pad.unwrap_or(tokens.pad),
            bordered: self.props.bordered,
            elevated: self.props.elevation > 0,
        }
    }

    /// Build a bottom application bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        let prop_height = props.height;
        let prop_pad = props.pad;
        let height = prop_height.unwrap_or_else(|| footer_tokens(&theme.get()).height);
        // layout-provider auto-register
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::Footer,
            height,
        );

        let bordered = props.bordered;
        let elevation = props.elevation;

        Container::new(child).style(move |s| {
            let t = theme.get();
            let tk = footer_tokens(&t);
            let mut s = s
                .width_pct(100.0)
                .height(prop_height.unwrap_or(tk.height))
                .padding(prop_pad.unwrap_or(tk.pad))
                .items_center()
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko());
            if bordered {
                s = s
                    .border_color(t.colors().outline_variant.to_peniko())
                    .border_top(tk.border);
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

impl Default for MdFooter {
    fn default() -> Self {
        Self::new()
    }
}

fn footer_tokens(theme: &MultiTheme) -> FooterTokens {
    match theme.component(ComponentId::Footer) {
        ComponentTokens::Footer(t) => t,
        _ => unreachable!("ComponentId::Footer must resolve to FooterTokens"),
    }
}
