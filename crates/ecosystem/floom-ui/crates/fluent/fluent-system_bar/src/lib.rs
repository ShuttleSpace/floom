use floom::prelude::*;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, SystemBarTokens};
use floom_core_theme::props::system_bar::SystemBarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

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
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
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

    pub fn resolved_style(&self) -> FluentSystemBarStyle {
        let theme = self.theme.get();
        let tokens = system_bar_tokens(&theme);
        let (surface, content) = if self.props.dark {
            (theme.colors().scrim, theme.colors().on_surface)
        } else {
            (theme.colors().surface_variant, theme.colors().on_surface_variant)
        };
        FluentSystemBarStyle {
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
        floom_core_theme::layout::register_layout(
            floom_core_theme::layout::LayoutSlot::SystemBar,
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

impl Default for FluentSystemBar {
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
