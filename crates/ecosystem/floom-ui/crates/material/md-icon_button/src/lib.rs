use floom::prelude::*;
use floom::style::CursorStyle;
use floom::views::{Container, Decorators, svg};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon_button::{IconButtonProps, IconButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;
use floom_widget_runtime::Ripple;

pub struct MdIconButton {
    props: IconButtonProps,
    theme: RwSignal<MultiTheme>,
}

impl MdIconButton {
    pub fn new(icon: impl Into<String>) -> Self {
        Self {
            props: IconButtonProps {
                icon: icon.into(),
                ..Default::default()
            },
            theme: theme_signal(),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = Some(size);
        self
    }

    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let icon = self.props.icon;
        let size = self.props.size;
        let variant = self.props.variant;
        let disabled = self.props.disabled;
        let on_click = self.props.on_click;

        let tokens = match theme.get().component(ComponentId::IconButton) {
            ComponentTokens::IconButton(t) => t,
            _ => unreachable!(),
        };
        let btn_size = size.unwrap_or(tokens.size);
        let icon_size = tokens.icon_size;

        let icon_view = svg(icon).style(move |s| {
            let t = theme.get();
            let fg = if disabled {
                with_alpha(t.colors().on_surface, 38)
            } else {
                t.colors().on_surface_variant
            };
            s.size(icon_size, icon_size).color(fg.to_peniko())
        });

        let inner = Container::new(icon_view)
            .style(move |s| {
                let t = theme.get();
                let (bg, border) = match variant {
                    IconButtonVariant::Text => (Color::rgba(0, 0, 0, 0), Color::rgba(0, 0, 0, 0)),
                    IconButtonVariant::Contained => (t.colors().primary, t.colors().primary),
                    IconButtonVariant::Outlined => (Color::rgba(0, 0, 0, 0), t.colors().outline),
                };
                let (bg, border) = if disabled {
                    (
                        with_alpha(t.colors().on_surface, 12),
                        with_alpha(t.colors().on_surface, 38),
                    )
                } else {
                    (bg, border)
                };
                s.size(btn_size, btn_size)
                    .border_radius(tokens.radius)
                    .background(bg.to_peniko())
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled)
            })
            .action(move || {
                if !disabled {
                    if let Some(cb) = &on_click {
                        cb();
                    }
                }
            });

        // Press ripple, clipped to the button's rounded bounds; ink color
        // follows the surface/foreground so it reads on every variant.
        Ripple::new(inner)
            .radius(tokens.radius)
            .color(move |t: &MultiTheme| match variant {
                IconButtonVariant::Contained => with_alpha(t.colors().on_primary, 24),
                _ => with_alpha(t.colors().on_surface, 24),
            })
            .disabled(disabled)
            .view()
    }
}
