use std::f32::consts::PI;

use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::views::{svg};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::progress_circular::ProgressCircularProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoProgressCircularStyle {
    pub design_system: &'static str,
    pub size: f32,
    pub stroke_width: f32,
    pub track_color: Color,
    pub fill_color: Color,
}

pub struct CupertinoProgressCircular {
    props: ProgressCircularProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoProgressCircular {
    pub fn new() -> Self {
        Self {
            props: ProgressCircularProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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
    pub fn resolved_style(&self) -> CupertinoProgressCircularStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressCircular(tokens) =
            theme.component(ComponentId::ProgressCircular)
        else {
            unreachable!("ComponentId::ProgressCircular must resolve to ProgressCircularTokens")
        };
        let size = if self.props.size > 0.0 {
            self.props.size
        } else {
            tokens.size
        };
        CupertinoProgressCircularStyle {
            design_system: DESIGN_SYSTEM,
            size,
            stroke_width: tokens.stroke_width,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
        }
    }

    /// Build a renderable Cupertino circular progress indicator (SVG ring).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.props.value;
        let indeterminate = self.props.indeterminate;
        let size = if self.props.size > 0.0 {
            self.props.size
        } else {
            match theme.get().component(ComponentId::ProgressCircular) {
                ComponentTokens::ProgressCircular(t) => t.size,
                _ => unreachable!(),
            }
        };

        let build = move || -> String {
            let t = theme.get();
            let tokens = match t.component(ComponentId::ProgressCircular) {
                ComponentTokens::ProgressCircular(tok) => tok,
                _ => unreachable!(),
            };
            let sw = tokens.stroke_width;
            let c = size / 2.0;
            let r = (size - sw).max(1.0) / 2.0;
            let circ = 2.0 * PI * r;
            let (dash, offset) = if indeterminate {
                // Static ~25% accent arc (no request_anim_frame in this Floem).
                (circ * 0.25, 0.0)
            } else {
                let ratio = value.clamp(0.0, 100.0) / 100.0;
                (circ, circ * (1.0 - ratio))
            };
            let track = t.colors().surface_variant.to_css();
            let accent = t.colors().primary.to_css();
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{size}\" height=\"{size}\" \
                 viewBox=\"0 0 {size} {size}\">\
                 <circle cx=\"{c}\" cy=\"{c}\" r=\"{r}\" fill=\"none\" \
                 stroke=\"{track}\" stroke-width=\"{sw}\"/>\
                 <circle cx=\"{c}\" cy=\"{c}\" r=\"{r}\" fill=\"none\" \
                 stroke=\"{accent}\" stroke-width=\"{sw}\" stroke-linecap=\"round\" \
                 stroke-dasharray=\"{dash} {circ}\" stroke-dashoffset=\"{offset}\" \
                 transform=\"rotate(-90 {c} {c})\"/>\
                 </svg>"
            )
        };

        svg(build()).update_value(build).style(move |s| s.size(size, size))
    }
}

impl Default for CupertinoProgressCircular {
    fn default() -> Self {
        Self::new()
    }
}
