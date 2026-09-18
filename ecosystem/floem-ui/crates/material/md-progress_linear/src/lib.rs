use floem::prelude::*;
use floem::ext_event::ArcRwSignal;
use floem::reactive::RwSignal;
use floem::views::{Container, Decorators, Empty};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::progress_linear::ProgressLinearProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdProgressLinearStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub fill_color: Color,
    pub height: f32,
    pub border_radius: f32,
}

pub struct MdProgressLinear {
    props: ProgressLinearProps,
    theme: RwSignal<MultiTheme>,
}

impl MdProgressLinear {
    pub fn new() -> Self {
        Self {
            props: ProgressLinearProps::default(),
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

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    pub fn resolved_style(&self) -> MdProgressLinearStyle {
        let theme = self.theme.get();
        let ComponentTokens::ProgressLinear(tokens) =
            theme.component(ComponentId::ProgressLinear)
        else {
            unreachable!("ComponentId::ProgressLinear must resolve to ProgressLinearTokens")
        };
        let h = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        MdProgressLinearStyle {
            design_system: "material",
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
            height: h,
            border_radius: tokens.border_radius,
        }
    }

    /// Build a renderable Material 3 linear progress indicator.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let value = self.props.value.clamp(0.0, 100.0);
        let indeterminate = self.props.indeterminate;

        let tokens = match theme.get().component(ComponentId::ProgressLinear) {
            ComponentTokens::ProgressLinear(t) => t,
            _ => unreachable!(),
        };
        let height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        let radius = tokens.border_radius;

        // Indeterminate animation: a ~40% segment that slides left<->right.
        let offset = ArcRwSignal::new(0.0f32);
        if indeterminate {
            let off = offset.clone();
            std::thread::spawn(move || {
                let mut v: f32 = 0.0;
                let mut dir: f32 = 1.0;
                loop {
                    v += dir * 0.03;
                    if v >= 1.0 {
                        v = 1.0;
                        dir = -1.0;
                    } else if v <= 0.0 {
                        v = 0.0;
                        dir = 1.0;
                    }
                    off.set(v);
                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
            });
        }

        let fill = if indeterminate {
            let off = offset.clone();
            Container::new(Empty::new()).style(move |s| {
                let t = theme.get();
                s.absolute()
                    .height_pct(100.0)
                    .width_pct(40.0)
                    .inset_left_pct((off.get() * 60.0) as f64)
                    .background(t.colors().primary.to_peniko())
            })
        } else {
            Container::new(Empty::new()).style(move |s| {
                let t = theme.get();
                s.absolute()
                    .height_pct(100.0)
                    .width_pct(value as f64)
                    .background(t.colors().primary.to_peniko())
            })
        };

        Container::new(fill).style(move |s| {
            let t = theme.get();
            s.width_full()
                .height(height)
                .border_radius(radius)
                .background(t.colors().surface_variant.to_peniko())
        })
    }
}

impl Default for MdProgressLinear {
    fn default() -> Self {
        Self::new()
    }
}
