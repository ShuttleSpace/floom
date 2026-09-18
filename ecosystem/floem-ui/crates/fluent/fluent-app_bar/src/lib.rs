use floem::prelude::*;
use floem::view::AnyView;
use floem::IntoView;
use floem::views::{Decorators, Empty, Label};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::app_bar::AppBarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentAppBarStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub title_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub title_font_size: f32,
    pub elevation: u8,
}

pub struct FluentAppBar {
    props: AppBarProps,
    leading: Option<AnyView>,
    actions: Vec<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl FluentAppBar {
    pub fn new() -> Self {
        Self {
            props: AppBarProps::default(),
            leading: None,
            actions: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    /// Optional leading control shown at the far left of the bar.
    pub fn leading(mut self, leading: impl IntoView + 'static) -> Self {
        self.leading = Some(leading.into_any());
        self
    }
    /// Trailing actions laid out left-to-right on the right side of the bar.
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
    pub fn resolved_style(&self) -> FluentAppBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::AppBar(tokens) = theme.component(ComponentId::AppBar) else {
            unreachable!("ComponentId::AppBar must resolve to AppBarTokens")
        };
        FluentAppBarStyle {
            design_system: DESIGN_SYSTEM,
            background: theme.colors().surface,
            title_color: theme.colors().on_surface,
            height: if self.props.height > 0.0 {
                self.props.height
            } else {
                tokens.height
            },
            padding_horizontal: tokens.padding_horizontal,
            title_font_size: tokens.title_font_size,
            elevation: self.props.elevation,
        }
    }

    /// Build a renderable Fluent app bar (compact, accent title, subtle elevation).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let leading = self.leading;
        let actions = self.actions;
        let title = self.props.title.clone();
        let elevation = self.props.elevation;

        let tokens = match theme.get().component(ComponentId::AppBar) {
            ComponentTokens::AppBar(t) => t,
            _ => unreachable!(),
        };
        let pad = tokens.padding_horizontal;
        let title_fs = tokens.title_font_size;
        let bar_height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.height
        };
// layout-provider auto-register
        floem_core_theme::layout::register_layout(
            floem_core_theme::layout::LayoutSlot::AppBar,
            bar_height,
        );

        let title_view = Label::new(title).style(move |s| {
            let t = theme.get();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(title_fs)
                .font_weight(FontWeight::SemiBold.to_floem())
        });

        let mut children: Vec<AnyView> = Vec::new();
        if let Some(lead) = leading {
            children.push(lead);
        }
        children.push(title_view.into_any());
        // Flexible spacer pushes the trailing actions to the right.
        children.push(Container::new(Empty::new()).style(|s| s.flex()).into_any());
        for a in actions {
            children.push(a);
        }

        Container::new(list(children)).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height(bar_height)
                .padding_horiz(pad)
                .flex_row()
                .items_center()
                .background(t.colors().surface.to_peniko())
                .border_bottom(1.0)
                .border_color(with_alpha(t.colors().on_surface, 12).to_peniko())
                .apply_if(elevation > 0, |s| {
                    s.box_shadow_blur((elevation as f32) * 2.0)
                        .box_shadow_spread(1.0)
                        .box_shadow_color(with_alpha(BLACK, 24).to_peniko())
                })
        })
    }
}

impl Default for FluentAppBar {
    fn default() -> Self {
        Self::new()
    }
}
