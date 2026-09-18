use floem::prelude::*;
use floem::view::AnyView;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::card::CardProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::shadow::ShadowData;
use floem_widget_runtime::Ripple;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentCardStyle {
    pub design_system: &'static str,
    pub container_color: Color,
    pub content_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub shadow: ShadowData,
}

pub struct FluentCard {
    props: CardProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentCard {
    pub fn new() -> Self {
        Self {
            props: CardProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    pub fn resolved_style(&self) -> FluentCardStyle {
        let theme = self.theme.get();
        let ComponentTokens::Card(tokens) = theme.component(ComponentId::Card) else {
            unreachable!("ComponentId::Card must resolve to CardTokens")
        };
        let level = usize::from(self.props.elevation.min(5));
        FluentCardStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            shadow: theme.shadow().levels[level],
        }
    }

    /// Build a renderable Fluent card surface.
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
            s.background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
                .padding(tokens.padding)
                .border_radius(t.radius().xl)
                .box_shadow_h_offset(shadow.offset_x)
                .box_shadow_v_offset(shadow.offset_y)
                .box_shadow_blur(shadow.blur)
                .box_shadow_spread(shadow.spread)
                .box_shadow_color(with_alpha(BLACK, shadow.color_a).to_peniko())
        });

        // Press ripple, clipped to the card's rounded bounds.
        Ripple::new(inner)
            .radius_fn(|t| t.radius().xl)
            .view()
    }
}

impl Default for FluentCard {
    fn default() -> Self {
        Self::new()
    }
}
