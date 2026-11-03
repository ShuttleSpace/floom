//! Fluent counter, ported to the Floom 2.0 widget API.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{Counter, CounterStyle};

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, CounterTokens};
use floom_core_theme::props::counter::CounterProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

pub const READOUT_FONT_SIZE: f32 = 16.0;

fn counter_tokens(theme: &MultiTheme) -> CounterTokens {
    match theme.component(ComponentId::Counter) {
        ComponentTokens::Counter(t) => t,
        _ => unreachable!("ComponentId::Counter must resolve to CounterTokens"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentCounterStyle {
    pub design_system: &'static str,
    pub button_background: Color,
    pub button_foreground: Color,
    pub value_color: Color,
    pub border_color: Color,
    pub button_size: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub gap: f32,
    pub border: f32,
}

pub struct FluentCounter {
    props: CounterProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentCounter {
    pub fn new() -> Self {
        Self {
            props: CounterProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn value(mut self, v: i64) -> Self {
        self.props.value = v;
        self
    }

    pub fn min(mut self, m: i64) -> Self {
        self.props.min = Some(m);
        self
    }

    pub fn max(mut self, m: i64) -> Self {
        self.props.max = Some(m);
        self
    }

    pub fn step(mut self, s: i64) -> Self {
        self.props.step = s;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(i64) + Send + Sync + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &CounterProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentCounterStyle {
        let theme = self.theme.get();
        let tokens = counter_tokens(&theme);
        FluentCounterStyle {
            design_system: DESIGN_SYSTEM,
            button_background: theme.colors().surface_variant,
            button_foreground: theme.colors().on_surface,
            value_color: theme.colors().on_surface,
            border_color: theme.colors().outline,
            button_size: tokens.button_size,
            width: tokens.width,
            height: tokens.height,
            radius: tokens.radius,
            gap: tokens.gap,
            border: tokens.border,
        }
    }

    pub fn counter_style(&self) -> CounterStyle {
        let style = self.resolved_style();
        CounterStyle {
            button_size: style.button_size,
            width: style.width,
            height: style.height,
            radius: style.radius,
            gap: style.gap,
            border: style.border,
            border_color: to_paint(style.border_color),
            button_background: to_paint(style.button_background),
            button_foreground: to_paint(style.button_foreground),
            value_font_size: READOUT_FONT_SIZE,
            value_color: to_paint(style.value_color),
        }
    }

    pub fn view(self) -> Counter {
        let mut counter = Counter::new(self.props.value).style(self.counter_style());
        if let Some(min) = self.props.min {
            counter = counter.min(min);
        }
        if let Some(max) = self.props.max {
            counter = counter.max(max);
        }
        counter = counter.step_size(self.props.step);
        if let Some(on_change) = self.props.on_change {
            counter = counter.on_change(move |v| on_change(v));
        }
        counter
    }
}

impl Default for FluentCounter {
    fn default() -> Self {
        Self::new()
    }
}
