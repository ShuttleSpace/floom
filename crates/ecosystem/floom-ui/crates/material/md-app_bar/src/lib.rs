use floom::prelude::*;
use floom::view::AnyView;
use floom::IntoView;
use floom::views::{Container, Decorators, Empty, h_stack_from_iter, Stack, Label};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::app_bar::AppBarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdAppBarStyle {
    pub design_system: &'static str,
    pub bar_color: Color,
    pub title_color: Color,
    pub action_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub title_font_size: f32,
    pub elevation: u8,
}

pub struct MdAppBar {
    props: AppBarProps,
    leading: Option<AnyView>,
    actions: Vec<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl MdAppBar {
    pub fn new() -> Self {
        Self {
            props: AppBarProps::default(),
            leading: None,
            actions: Vec::new(),
            theme: theme_signal(),
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

    /// Optional leading widget shown at the left (e.g. a menu icon).
    pub fn leading(mut self, leading: impl IntoView + 'static) -> Self {
        self.leading = Some(leading.into_any());
        self
    }

    /// Trailing action widgets laid out left-to-right at the right.
    pub fn actions(mut self, actions: Vec<AnyView>) -> Self {
        self.actions = actions;
        self
    }

    pub fn resolved_style(&self) -> MdAppBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::AppBar(tokens) = theme.component(ComponentId::AppBar) else {
            unreachable!("ComponentId::AppBar must resolve to AppBarTokens")
        };
        MdAppBarStyle {
            design_system: "material",
            bar_color: theme.colors().surface,
            title_color: theme.colors().on_surface,
            action_color: theme.colors().on_surface_variant,
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

    /// Build a renderable Material 3 top app bar.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let leading = self.leading;
        let actions = self.actions;
        let title = self.props.title.clone();

        let tokens = match theme.get().component(ComponentId::AppBar) {
            ComponentTokens::AppBar(t) => t,
            _ => unreachable!(),
        };
        let height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.height
        };
        let pad = tokens.padding_horizontal;
        let title_font = tokens.title_font_size;
        let elevation = self.props.elevation;
// layout-provider auto-register
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::AppBar,
            height,
        );


        let mut children: Vec<AnyView> = Vec::new();
        if let Some(lead) = leading {
            children.push(lead);
        }

        let title_view = Label::new(title).style(move |s| {
            let t = theme.get();
            s.flex_grow(1.0)
                .color(t.colors().on_surface.to_peniko())
                .font_size(title_font)
        });
        children.push(title_view.into_any());

        if !actions.is_empty() {
            let actions_view = Stack::horizontal_from_iter(actions)
                .style(move |s| s.gap(8.0).items_center().justify_end());
            children.push(actions_view.into_any());
        }

        Container::new(Stack::horizontal_from_iter(children).style(move |s| {
            s.items_center().gap(pad).height_full().padding_horiz(pad)
        }))
        .style(move |s| {
            let t = theme.get();
            s.width_full()
                .height(height)
                .items_center()
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
                .apply_if(elevation > 0, |s| {
                    s.box_shadow_blur(2.0 + 2.0 * (elevation as f64))
                        .box_shadow_spread(1.0)
                        .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 30).to_peniko())
                })
        })
    }
}

impl Default for MdAppBar {
    fn default() -> Self {
        Self::new()
    }
}
