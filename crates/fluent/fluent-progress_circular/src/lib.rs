use std::f32::consts::PI;

use floem::prelude::*;
use floem::views::{Decorators};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::progress_circular::ProgressCircularProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentProgressCircularStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub progress_color: Color,
    pub size: f32,
    pub stroke_width: f32,
}

pub struct FluentProgressCircular {
    props: ProgressCircularProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentProgressCircular {
    pub fn new() -> Self {
        Self {
            props: ProgressCircularProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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
    pub fn size(mut self, size: f32) -> Self {
        self.props.size = size;
        self
    }
    pub fn props(&self) -> &ProgressCircularProps {
        &self.props
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentProgressCircularStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressCircular(tokens) =
            theme.component(ComponentId::ProgressCircular)
        else {
            unreachable!("ComponentId::ProgressCircular must resolve to ProgressCircularTokens")
        };
        FluentProgressCircularStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            progress_color: theme.colors().primary,
            // The `size` prop overrides the token-derived diameter.
            size: if self.props.size > 0.0 {
                self.props.size
            } else {
                tokens.size
            },
            stroke_width: tokens.stroke_width,
        }
    }

    /// Build a renderable Fluent circular progress indicator drawn with an SVG
    /// `<circle>` using `stroke-dasharray`/`stroke-dashoffset`. When `indeterminate`
    /// is set, a static accent arc (fallback) is shown instead of a value.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.props.value;
        let indeterminate = self.props.indeterminate;

        let tokens = match theme.get().component(ComponentId::ProgressCircular) {
            ComponentTokens::ProgressCircular(t) => t,
            _ => unreachable!(),
        };
        let size = if self.props.size > 0.0 {
            self.props.size
        } else {
            tokens.size
        };
        let stroke = tokens.stroke_width;

        let build = move || {
            let t = theme.get();
            let s = size;
            let sw = stroke;
            let c = s / 2.0;
            let r = (s - sw) / 2.0;
            let circ = 2.0 * PI * r;
            let (dash, offset, rot) = if indeterminate {
                // A static ~25% accent arc (indeterminate fallback).
                (circ * 0.25, 0.0, 0.0)
            } else {
                let v = value.clamp(0.0, 100.0) / 100.0;
                (circ, circ * (1.0 - v), -90.0)
            };
            format!(
                "<svg width=\"{s}\" height=\"{s}\" viewBox=\"0 0 {s} {s}\">\
                 <circle cx=\"{c}\" cy=\"{c}\" r=\"{r}\" fill=\"none\" \
                 stroke=\"{track}\" stroke-width=\"{sw}\"/>\
                 <circle cx=\"{c}\" cy=\"{c}\" r=\"{r}\" fill=\"none\" \
                 stroke=\"{prog}\" stroke-width=\"{sw}\" stroke-linecap=\"round\" \
                 stroke-dasharray=\"{dash:.2}\" stroke-dashoffset=\"{offset:.2}\" \
                 transform=\"rotate({rot} {c} {c})\"/></svg>",
                track = t.colors().surface_variant.to_css(),
                prog = t.colors().primary.to_css(),
            )
        };

        svg(build())
            .update_value(build)
            .style(move |s| s.width(size).height(size))
    }
}

impl Default for FluentProgressCircular {
    fn default() -> Self {
        Self::new()
    }
}
