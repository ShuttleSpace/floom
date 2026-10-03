use floom::prelude::*;
use floom::views::{Decorators, Empty};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::progress_linear::ProgressLinearProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentProgressLinearStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub progress_color: Color,
    pub track_height: f32,
    pub border_radius: f32,
}

pub struct FluentProgressLinear {
    props: ProgressLinearProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentProgressLinear {
    pub fn new() -> Self {
        Self {
            props: ProgressLinearProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }
    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.props.indeterminate = indeterminate;
        self
    }
    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }
    pub fn props(&self) -> &ProgressLinearProps {
        &self.props
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentProgressLinearStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressLinear(tokens) =
            theme.component(ComponentId::ProgressLinear)
        else {
            unreachable!("ComponentId::ProgressLinear must resolve to ProgressLinearTokens")
        };
        FluentProgressLinearStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            progress_color: theme.colors().primary,
            // The `height` prop overrides the token-derived track height.
            track_height: if self.props.height > 0.0 {
                self.props.height
            } else {
                tokens.track_height
            },
            border_radius: tokens.border_radius,
        }
    }

    /// Build a renderable Fluent linear progress indicator. When `indeterminate`
    /// is set, a static accent segment (fallback) is shown in place of a value.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.props.value;
        let indeterminate = self.props.indeterminate;

        let tokens = match theme.get().component(ComponentId::ProgressLinear) {
            ComponentTokens::ProgressLinear(t) => t,
            _ => unreachable!(),
        };
        let track_height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        let radius = tokens.border_radius;

        let fill_pct = if indeterminate {
            40.0
        } else {
            value.clamp(0.0, 100.0)
        };

        let fill = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            s.height_full()
                .width_pct(fill_pct as f64)
                .background(t.colors().primary.to_peniko())
                .border_radius(radius)
        });

        Container::new(fill).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height(track_height)
                .background(t.colors().surface_variant.to_peniko())
                .border_radius(radius)
                .items_center()
        })
    }
}

impl Default for FluentProgressLinear {
    fn default() -> Self {
        Self::new()
    }
}
