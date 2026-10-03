use floom::prelude::*;
use floom::reactive::{RwSignal, SignalGet};
use floom::view::AnyView;
use floom::IntoView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::card::CardProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::shadow::ShadowData;
use floom_widget_runtime::Ripple;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoCardStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct CupertinoCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl CupertinoCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            child: None,
        }
    }
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation.min(5);
        self
    }
    /// Set the card's content.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }
    pub fn props(&self) -> &CardProps {
        &self.props
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        CupertinoCardStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }

    /// Build a renderable Cupertino card surface.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let elevation = self.props.elevation;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        let inner = Container::new(child).style(move |s| {
            let t = theme.get();
            let ComponentTokens::Card(tokens) = t.component(ComponentId::Card) else {
                unreachable!()
            };
            let level = usize::from(elevation.min(5));
            let shadow = t.shadow().levels[level];
            let radius = t.radius().md;
            s.background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
                .padding(tokens.padding)
                .border_radius(radius)
                .box_shadow_h_offset(shadow.offset_x)
                .box_shadow_v_offset(shadow.offset_y)
                .box_shadow_blur(shadow.blur)
                .box_shadow_spread(shadow.spread)
                .box_shadow_color(with_alpha(BLACK, shadow.color_a).to_peniko())
        });

        // Press ripple, clipped to the card's rounded bounds.
        Ripple::new(inner)
            .radius_fn(|t| t.radius().md)
            .view()
    }
}

impl Default for CupertinoCard {
    fn default() -> Self {
        Self::new()
    }
}
