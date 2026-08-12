use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::toolbar::ToolbarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdToolbarStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub padding: f32,
    pub title_font_size: f32,
    pub elevated: bool,
}

pub struct MdToolbar {
    props: ToolbarProps,
    theme: RwSignal<MultiTheme>,
    actions: Option<AnyView>,
}

impl MdToolbar {
    pub fn new() -> Self {
        Self {
            props: ToolbarProps::default(),
            theme: theme_signal(),
            actions: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn leading(mut self, icon: impl Into<String>) -> Self {
        self.props.leading = Some(icon.into());
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    /// Supply the trailing actions slot (icon buttons, menus, etc.).
    pub fn actions(mut self, child: impl IntoView + 'static) -> Self {
        self.actions = Some(child.into_any());
        self
    }

    pub fn props(&self) -> &ToolbarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdToolbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Toolbar(tokens) = theme.component(ComponentId::Toolbar) else {
            unreachable!("ComponentId::Toolbar must resolve to ToolbarTokens")
        };
        MdToolbarStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            height: self.props.height,
            padding: tokens.padding,
            title_font_size: tokens.title_font_size,
            elevated: self.props.elevation > 0,
        }
    }

    /// Build a compact Material 3 toolbar bar with a leading icon, title and an
    /// optional trailing actions slot.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let leading = props.leading.clone();
        let title = props.title.clone();
        let elevation = props.elevation;
        let actions = self.actions.unwrap_or_else(|| empty().into_any());

        let tokens = match theme.get().component(ComponentId::Toolbar) {
            ComponentTokens::Toolbar(t) => t,
            _ => unreachable!(),
        };
        let pad = tokens.padding;
        let title_fs = tokens.title_font_size;

        let leading_v: AnyView = if let Some(ic) = leading {
            Container::new(text(ic))
                .style(move |s| {
                    let t = theme.get();
                    s.font_size(22.0).color(t.colors().on_surface.to_peniko())
                })
                .into_any()
        } else {
            empty().into_any()
        };

        let title_v = text(title)
            .style(move |s| {
                let t = theme.get();
                s.font_size(title_fs)
                    .font_weight(FontWeight::Medium.to_floem())
                    .color(t.colors().on_surface.to_peniko())
            })
            .into_any();

        let left = h_stack((leading_v, title_v))
            .style(|s| s.gap(4.0).items_center())
            .into_any();

        Container::new(h_stack((left, actions)).style(|s| s.items_center().justify_between()))
            .style(move |s| {
                let t = theme.get();
                let mut s = s
                    .width_pct(100.0)
                    .height(props.height)
                    .padding(pad)
                    .items_center()
                    .background(t.colors().surface.to_peniko())
                    .color(t.colors().on_surface.to_peniko());
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

impl Default for MdToolbar {
    fn default() -> Self {
        Self::new()
    }
}
