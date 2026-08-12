use floem::prelude::*;
use floem::view::AnyView;
use floem_core_theme::props::system_bar::SystemBarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSystemBarStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub pad: f32,
    pub dark: bool,
}

pub struct FluentSystemBar {
    props: SystemBarProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl FluentSystemBar {
    pub fn new() -> Self {
        Self {
            props: SystemBarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            child: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    /// Dark variant: uses a darker, higher-contrast background.
    pub fn dark(mut self, dark: bool) -> Self {
        self.props.dark = dark;
        self
    }

    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = pad;
        self
    }

    /// Supply the system bar's content.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    pub fn props(&self) -> &SystemBarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> FluentSystemBarStyle {
        let theme = self.theme.get();
        let (surface, content) = if self.props.dark {
            (theme.colors().scrim, theme.colors().on_surface)
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface_variant)
        };
        FluentSystemBarStyle {
            surface,
            content,
            height: self.props.height,
            pad: self.props.pad,
            dark: self.props.dark,
        }
    }

    /// Build a thin status bar at the top of the application shell.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child.unwrap_or_else(|| empty().into_any());
        let height = props.height;
// layout-provider auto-register
        floem_core_theme::layout::register_layout(
            floem_core_theme::layout::LayoutSlot::SystemBar,
            height,
        );

        let pad = props.pad;
        let dark = props.dark;

        Container::new(child).style(move |s| {
            let t = theme.get();
            let (bg, fg) = if dark {
                (t.colors().scrim, t.colors().on_surface)
            } else {
                (t.colors().surface_variant, t.colors().on_surface_variant)
            };
            s.width_pct(100.0)
                .height(height)
                .padding(pad)
                .items_center()
                .background(bg.to_peniko())
                .color(fg.to_peniko())
        })
    }
}

impl Default for FluentSystemBar {
    fn default() -> Self {
        Self::new()
    }
}
