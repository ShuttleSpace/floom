use floem::prelude::*;
use floem::ext_event::ArcRwSignal;
use floem::reactive::RwSignal;
use floem::views::{Container, Decorators, Empty, Stack, svg};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::progress_circular::ProgressCircularProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdProgressCircularStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub fill_color: Color,
    pub size: f32,
    pub stroke_width: f32,
}

pub struct MdProgressCircular {
    props: ProgressCircularProps,
    theme: RwSignal<MultiTheme>,
}

impl MdProgressCircular {
    pub fn new() -> Self {
        Self {
            props: ProgressCircularProps::default(),
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MdProgressCircularStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressCircular(tokens) =
            theme.component(ComponentId::ProgressCircular)
        else {
            unreachable!("ComponentId::ProgressCircular must resolve to ProgressCircularTokens")
        };
        let sz = if self.props.size > 0.0 {
            self.props.size
        } else {
            tokens.size
        };
        MdProgressCircularStyle {
            design_system: "material",
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
            size: sz,
            stroke_width: tokens.stroke_width,
        }
    }

    /// Build a renderable Material 3 circular progress indicator (SVG arc).
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.props.value.clamp(0.0, 100.0);
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
        let r = size / 2.0 - stroke / 2.0;
        let c = size / 2.0;
        let circ = 2.0 * std::f32::consts::PI * r;

        // Indeterminate animation: a ~30% arc that rotates continuously.
        let offset = ArcRwSignal::new(0.0f32);
        if indeterminate {
            let off = offset.clone();
            std::thread::spawn(move || {
                let mut v: f32 = 0.0;
                loop {
                    v = (v + 0.03) % 1.0;
                    off.set(v);
                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
            });
        }

        let off = offset.clone();
        let build_svg = move || -> String {
            let (da, doff, rot) = if indeterminate {
                let angle = off.get() * 360.0;
                (
                    format!("{:.2} {:.2}", 0.3 * circ, 0.7 * circ),
                    0.0,
                    angle - 90.0,
                )
            } else {
                let p = value / 100.0;
                (format!("{:.2}", circ), circ * (1.0 - p), -90.0)
            };
            format!(
                "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 {s} {s}'>\
                 <circle cx='{c}' cy='{c}' r='{r}' fill='none' stroke='currentColor' \
                 stroke-width='{sw}' stroke-linecap='round' stroke-dasharray='{da}' \
                 stroke-dashoffset='{doff}' transform='rotate({rot} {c} {c})'/></svg>",
                s = size,
                c = c,
                r = r,
                sw = stroke,
                da = da,
                doff = doff,
                rot = rot,
            )
        };

        let track = Container::new(Empty::new()).style(move |s| {
            let t = theme.get();
            s.absolute()
                .size(size, size)
                .border_radius(50.0)
                .border(stroke)
                .border_color(t.colors().surface_variant.to_peniko())
        });

        let progress = svg(build_svg())
            .update_value(build_svg)
            .style(move |s| {
                let t = theme.get();
                s.absolute()
                    .size(size, size)
                    .color(t.colors().primary.to_peniko())
            });

        Stack::new((track, progress))
    }
}

impl Default for MdProgressCircular {
    fn default() -> Self {
        Self::new()
    }
}
