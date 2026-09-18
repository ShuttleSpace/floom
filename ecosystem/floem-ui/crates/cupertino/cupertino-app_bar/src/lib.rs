use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem::IntoView;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::app_bar::AppBarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoAppBarStyle {
    pub design_system: &'static str,
    pub height: f32,
    pub padding_horizontal: f32,
    pub title_font_size: f32,
    pub elevation: u8,
    pub background: Color,
    pub title_color: Color,
    pub action_color: Color,
}

pub struct CupertinoAppBar {
    props: AppBarProps,
    theme: RwSignal<MultiTheme>,
    leading: Option<AnyView>,
    actions: Vec<AnyView>,
}

impl CupertinoAppBar {
    pub fn new() -> Self {
        Self {
            props: AppBarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            leading: None,
            actions: Vec::new(),
        }
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
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
    /// Optional view shown at the leading (left) edge of the bar.
    pub fn leading(mut self, leading: impl IntoView + 'static) -> Self {
        self.leading = Some(leading.into_any());
        self
    }
    /// Views shown at the trailing (right) edge, left-to-right.
    pub fn actions(mut self, actions: Vec<AnyView>) -> Self {
        self.actions = actions;
        self
    }
    pub fn props(&self) -> &AppBarProps {
        &self.props
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoAppBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::AppBar(tokens) = theme.component(ComponentId::AppBar) else {
            unreachable!("ComponentId::AppBar must resolve to AppBarTokens")
        };
        CupertinoAppBarStyle {
            design_system: DESIGN_SYSTEM,
            height: if self.props.height > 0.0 {
                self.props.height
            } else {
                tokens.height
            },
            padding_horizontal: tokens.padding_horizontal,
            title_font_size: tokens.title_font_size,
            elevation: self.props.elevation,
            // Cupertino bars are translucent / slightly blurred.
            background: with_alpha(theme.colors().surface, 230),
            title_color: theme.colors().on_surface,
            action_color: theme.colors().primary,
        }
    }

    /// Build a renderable Cupertino translucent app bar with a large title.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let title = self.props.title.clone();
        let leading = self.leading.unwrap_or_else(|| Empty::new().into_any());
        let actions = self.actions;

        let tokens = match theme.get().component(ComponentId::AppBar) {
            ComponentTokens::AppBar(t) => t,
            _ => unreachable!(),
        };
        let bar_height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.height
        };
        let padding_horizontal = tokens.padding_horizontal;
        let title_font_size = tokens.title_font_size;
        let elevation = self.props.elevation;
// layout-provider auto-register
        floem_core_theme::layout::register_layout(
            floem_core_theme::layout::LayoutSlot::AppBar,
            bar_height,
        );


        let action_views: Vec<AnyView> = actions
            .into_iter()
            .map(|a| a.style(move |s| s.cursor(CursorStyle::Pointer)))
            .collect();

        let title_view = Label::new(title).style(move |s| {
            let t = theme.get();
            s.flex_grow(1.0)
                .justify_center()
                .color(t.colors().on_surface.to_peniko())
                .font_size(title_font_size)
                .font_weight(FontWeight::SemiBold.to_floem())
        });

        Stack::horizontal((
            leading.style(move |s| s.padding_right(padding_horizontal / 2.0)),
            title_view,
            list(action_views)
                .style(move |s| s.gap(8.0).items_center().padding_left(padding_horizontal / 2.0)),
        ))
        .style(move |s| {
            let t = theme.get();
            let elevation_blur = (elevation as f32) * 3.0;
            s.width_pct(100.0)
                .height(bar_height)
                .padding_horiz(padding_horizontal)
                .items_center()
                .background(with_alpha(t.colors().surface, 230).to_peniko())
                // hairline bottom divider
                .border_bottom(0.5)
                .border_color(with_alpha(t.colors().on_surface, 40).to_peniko())
                .apply_if(elevation > 0, |s| {
                    s.box_shadow_blur(elevation_blur)
                        .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 40).to_peniko())
                })
        })
    }
}

impl Default for CupertinoAppBar {
    fn default() -> Self {
        Self::new()
    }
}
