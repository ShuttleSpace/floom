use floom::prelude::*;
use floom::reactive::RwSignal;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::progress_linear::ProgressLinearProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoProgressLinearStyle {
    pub design_system: &'static str,
    pub track_height: f32,
    pub border_radius: f32,
    pub track_color: Color,
    pub fill_color: Color,
}

pub struct CupertinoProgressLinear {
    props: ProgressLinearProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoProgressLinear {
    pub fn new() -> Self {
        Self {
            props: ProgressLinearProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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
    pub fn resolved_style(&self) -> CupertinoProgressLinearStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressLinear(tokens) =
            theme.component(ComponentId::ProgressLinear)
        else {
            unreachable!("ComponentId::ProgressLinear must resolve to ProgressLinearTokens")
        };
        let track_height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        CupertinoProgressLinearStyle {
            design_system: DESIGN_SYSTEM,
            track_height,
            border_radius: tokens.border_radius,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
        }
    }

    /// Build a renderable Cupertino linear progress indicator.
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

        let fill = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            let ratio = value.clamp(0.0, 100.0) / 100.0;
            let width_pct = if indeterminate { 40.0 } else { (ratio * 100.0) as f64 };
            s.absolute()
                .inset_left(0.0)
                .inset_top(0.0)
                .height(track_height)
                .width_pct(width_pct)
                .background(t.colors().primary.to_peniko())
                .border_radius(radius)
        });

        Container::new(fill).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height(track_height)
                .background(t.colors().surface_variant.to_peniko())
                .border_radius(radius)
        })
    }
}

impl Default for CupertinoProgressLinear {
    fn default() -> Self {
        Self::new()
    }
}
