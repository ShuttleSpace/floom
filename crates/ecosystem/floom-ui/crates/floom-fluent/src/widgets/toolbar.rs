//! Fluent toolbar, ported to the Floom 2.0 widget API.

use floom_core::{FontWeight as EngineFontWeight, LayoutRequest, WidgetAny};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::toolbar::ToolbarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Align, Axis, Icon, Stack, Surface, Text};

const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

const LEADING_GAP: f32 = 4.0;

const LEADING_ICON_SIZE: f32 = 22.0;

/// — note there is no offset, so the shadow is a symmetric halo rather than a
const ELEVATION_BLUR: f32 = 6.0;

const ELEVATION_ALPHA: u8 = 30;

/// Marks [`ToolbarTokens::height`] as deliberately unread. See the module note.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolbarTokensHeightNote;

impl ToolbarTokensHeightNote {
    pub fn token_height(&self, theme: &MultiTheme) -> f32 {
        match theme.component(ComponentId::Toolbar) {
            ComponentTokens::Toolbar(t) => t.height,
            _ => 0.0,
        }
    }
}

pub type ActionsBuilder = std::sync::Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

fn to_engine_weight(w: floom_core_theme::tokens::typography::FontWeight) -> EngineFontWeight {
    use floom_core::FontWeight as W;
    use floom_core_theme::tokens::typography::FontWeight as T;
    match w {
        T::Thin => W::Thin,
        T::ExtraLight => W::ExtraLight,
        T::Light => W::Light,
        T::Regular => W::Normal,
        T::Medium => W::Medium,
        T::SemiBold => W::SemiBold,
        T::Bold => W::Bold,
        T::ExtraBold => W::ExtraBold,
        T::Black => W::Black,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentToolbarStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub padding: f32,
    pub title_font_size: f32,
    pub title_weight: EngineFontWeight,
    pub elevated: bool,
}

pub struct FluentToolbar {
    props: ToolbarProps,
    theme: RwSignal<MultiTheme>,
    actions: Option<ActionsBuilder>,
}

impl FluentToolbar {
    pub fn new() -> Self {
        Self {
            props: ToolbarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
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

    pub fn actions(
        mut self,
        build: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.actions = Some(std::sync::Arc::new(build));
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

    pub fn resolved_style(&self) -> FluentToolbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Toolbar(tokens) = theme.component(ComponentId::Toolbar) else {
            unreachable!("ComponentId::Toolbar must resolve to ToolbarTokens")
        };
        FluentToolbarStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            height: self.props.height,
            padding: tokens.padding,
            title_font_size: tokens.title_font_size,
            title_weight: to_engine_weight(
                floom_core_theme::tokens::typography::FontWeight::Medium,
            ),
            elevated: self.props.elevation > 0,
        }
    }

    pub fn request(&self) -> LayoutRequest {
        let style = self.resolved_style();
        LayoutRequest::new().height(style.height).width_pct(100.0)
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    pub fn shadow(&self) -> Option<Shadow> {
        self.resolved_style().elevated.then(|| Shadow {
            color: to_paint(with_alpha(BLACK, ELEVATION_ALPHA)),
            blur: f64::from(ELEVATION_BLUR),
            offset_x: 0.0,
            offset_y: 0.0,
            spread: 0.0,
        })
    }

    pub fn leading_widget(&self, glyph: char) -> Icon {
        let content = to_paint(self.resolved_style().content);
        Icon::new(glyph).size(LEADING_ICON_SIZE).color(content)
    }

    pub fn title_widget(&self, title: &str) -> Text {
        let style = self.resolved_style();
        Text::new(title)
            .size(style.title_font_size)
            .weight(style.title_weight)
            .color(to_paint(style.content))
    }

    pub fn leading_group(&self) -> Stack {
        let style = self.resolved_style();
        leading_group_of(
            self.props.leading.as_deref().and_then(|s| s.chars().next()),
            &self.props.title,
            style.title_font_size,
            style.title_weight,
            to_paint(style.content),
        )
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let request = self.request();
        let padding = self.padding();
        let shadow = self.shadow();
        let surface_colour = to_paint(style.surface);
        let content = to_paint(style.content);
        let title = self.props.title.clone();
        let leading = self.props.leading.as_deref().and_then(|s| s.chars().next());
        let title_size = style.title_font_size;
        let title_weight = style.title_weight;
        let actions = self.actions;

        let mut surface = Surface::new()
            .request(request)
            .padding(padding)
            .background(surface_colour)
            .push(move || {
                Box::new(bar_of(
                    leading,
                    &title,
                    title_size,
                    title_weight,
                    content,
                    actions.clone(),
                ))
            });
        if let Some(shadow) = shadow {
            surface = surface.shadow(shadow);
        }
        surface
    }
}

fn bar_of(
    leading: Option<char>,
    title: &str,
    title_size: f32,
    title_weight: EngineFontWeight,
    content: PaintColor,
    actions: Option<ActionsBuilder>,
) -> Stack {
    let title = title.to_string();
    let mut bar = Stack::new(Axis::Horizontal)
        .align(Align::SpaceBetween, Align::Center)
        .push(move || {
            Box::new(leading_group_of(
                leading,
                &title,
                title_size,
                title_weight,
                content,
            ))
        });
    if let Some(actions) = actions {
        bar = bar.push(move || actions());
    }
    bar
}

fn leading_group_of(
    leading: Option<char>,
    title: &str,
    title_size: f32,
    title_weight: EngineFontWeight,
    content: PaintColor,
) -> Stack {
    let mut group = Stack::new(Axis::Horizontal)
        .gap(LEADING_GAP)
        .align(Align::Start, Align::Center);
    if let Some(glyph) = leading {
        group =
            group.push(move || Box::new(Icon::new(glyph).size(LEADING_ICON_SIZE).color(content)));
    }
    let title = title.to_string();
    group.push(move || {
        Box::new(
            Text::new(title.clone())
                .size(title_size)
                .weight(title_weight)
                .color(content),
        )
    })
}

impl Default for FluentToolbar {
    fn default() -> Self {
        Self::new()
    }
}
