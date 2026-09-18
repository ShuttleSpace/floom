use floem::prelude::*;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens, SystemBarTokens};
use floem_core_theme::props::system_bar::SystemBarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSystemBarStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub pad: f32,
    pub dark: bool,
}

pub struct MdSystemBar {
    props: SystemBarProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl MdSystemBar {
    pub fn new() -> Self {
        Self {
            props: SystemBarProps::default(),
            theme: theme_signal(),
            child: None,
        }
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = Some(height);
        self
    }

    /// Dark variant: uses a darker, higher-contrast background.
    pub fn dark(mut self, dark: bool) -> Self {
        self.props.dark = dark;
        self
    }

    pub fn pad(mut self, pad: f32) -> Self {
        self.props.pad = Some(pad);
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

    pub fn resolved_style(&self) -> MdSystemBarStyle {
        let theme = self.theme.get();
        let tokens = system_bar_tokens(&theme);
        let (surface, content) = if self.props.dark {
            (theme.colors().scrim, theme.colors().on_surface)
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface_variant)
        };
        MdSystemBarStyle {
            surface,
            content,
            height: self.props.height.unwrap_or(tokens.height),
            pad: self.props.pad.unwrap_or(tokens.pad),
            dark: self.props.dark,
        }
    }

    /// Build a thin status bar at the top of the application shell.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        let prop_height = props.height;
        let prop_pad = props.pad;
        let height = prop_height.unwrap_or_else(|| system_bar_tokens(&theme.get()).height);
        // layout-provider auto-register
        floem_core_theme::layout::register_layout(
            floem_core_theme::layout::LayoutSlot::SystemBar,
            height,
        );

        let dark = props.dark;

        Container::new(child).style(move |s| {
            let t = theme.get();
            let (bg, fg) = if dark {
                (t.colors().scrim, t.colors().on_surface)
            } else {
                (t.colors().surface_variant, t.colors().on_surface_variant)
            };
            let tk = system_bar_tokens(&t);
            s.width_pct(100.0)
                .height(prop_height.unwrap_or(tk.height))
                .padding(prop_pad.unwrap_or(tk.pad))
                .items_center()
                .background(bg.to_peniko())
                .color(fg.to_peniko())
        })
    }
}

impl Default for MdSystemBar {
    fn default() -> Self {
        Self::new()
    }
}

fn system_bar_tokens(theme: &MultiTheme) -> SystemBarTokens {
    match theme.component(ComponentId::SystemBar) {
        ComponentTokens::SystemBar(t) => t,
        _ => unreachable!("ComponentId::SystemBar must resolve to SystemBarTokens"),
    }
}
