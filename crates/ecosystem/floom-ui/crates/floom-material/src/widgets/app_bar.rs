use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Shadow;
use floom_widgets::{Align, Axis, Spacer, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::app_bar::AppBarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

const ACTION_GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialAppBarStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub bar_color: Color,
    /// The title's ink.
    pub title_color: Color,
    /// The actions' ink.
    pub action_color: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub title_font_size: f32,
    pub elevation: u8,
}

/// Material's app bar.
pub struct MaterialAppBar {
    props: AppBarProps,
    /// The slot at the left, if any.
    leading: Option<Child>,
    /// The slots at the right.
    actions: Vec<Child>,
    theme: RwSignal<MultiTheme>,
}

impl MaterialAppBar {
    pub fn new() -> Self {
        Self {
            props: AppBarProps::default(),
            leading: None,
            actions: Vec::new(),
            theme: theme_signal(),
        }
    }

    /// The bar's title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    /// The bar's elevation: the depth of the shadow it draws.
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    /// The bar's height, in place of the token's.
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    /// The slot at the left, if any.
    pub fn leading(mut self, leading: Child) -> Self {
        self.leading = Some(leading);
        self
    }

    /// The slots at the right, laid out left to right.
    pub fn actions(mut self, actions: Vec<Child>) -> Self {
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

    pub fn resolved_style(&self) -> MaterialAppBarStyle {
        let theme = self.theme.get();
        let ComponentTokens::AppBar(tokens) = theme.component(ComponentId::AppBar) else {
            unreachable!("ComponentId::AppBar must resolve to AppBarTokens")
        };
        MaterialAppBarStyle {
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

    /// The bar: the row of leading, title and actions, on the bar's own fill.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let title = self.props.title.clone();
        let title_size = style.title_font_size;
        let title_color = to_paint(style.title_color);
        let action_color = to_paint(style.action_color);
        let padding = style.padding_horizontal;

        let mut row = Stack::new(Axis::Horizontal)
            .align(Align::Center, Align::Center)
            .gap(padding)
            .request(LayoutRequest::new().padding_horiz(f64::from(padding)));

        if let Some(leading) = self.leading {
            row = row.child(leading);
        }

        row = row.push({
            let title = title.clone();
            let title_color = title_color;
            move || Box::new(Text::new(title.clone()).size(title_size).color(title_color))
        });

        if !self.actions.is_empty() {
            row = row.push(|| Box::new(Spacer::flexible()));
            let mut actions = Stack::new(Axis::Horizontal)
                .gap(ACTION_GAP)
                .align(Align::End, Align::Center);
            for action in self.actions {
                actions = actions.child(action);
            }
            row = row.push(move || Box::new(actions.clone()));
        }

        let bar_color = to_paint(style.bar_color);
        let elevation = style.elevation;
        let height = style.height;

        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::AppBar,
            height,
        );

        let mut bar = Surface::new()
            .request(LayoutRequest::new().height(height))
            .background(bar_color);
        if elevation > 0 {
            bar = bar.shadow(Shadow {
                color: to_paint(with_alpha(Color::BLACK, 30)),
                blur: 2.0 + 2.0 * f64::from(elevation),
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            });
        }
        bar.push(move || Box::new(row.clone()))
    }
}

impl Default for MaterialAppBar {
    fn default() -> Self {
        Self::new()
    }
}
