use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Decorators, Empty, h_stack_from_iter, Stack, Label};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::floating_action_button::FloatingActionButtonProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

/// Material 3 Floating Action Button: a circular, elevated primary button
/// (optionally extended with a label).
pub struct MdFloatingActionButton {
    props: FloatingActionButtonProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl MdFloatingActionButton {
    pub fn new() -> Self {
        Self {
            props: FloatingActionButtonProps::default(),
            theme: theme_signal(),
            child: None,
        }
    }

    /// Leading icon / content of the FAB.
    pub fn child(mut self, child: impl IntoView) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Optional text label; renders the FAB as an extended (pill) button.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    /// Force the extended (label + icon) variant even when a label is present.
    pub fn extended(mut self, extended: bool) -> Self {
        self.props.extended = extended;
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

    pub fn props(&self) -> &FloatingActionButtonProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = RwSignal::new(self.props.disabled);
        let on_click = self.props.on_click;
        let label = self.props.label.clone();
        let extended = self.props.extended && !label.is_empty();
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let tokens = match theme.get().component(ComponentId::FloatingActionButton) {
            ComponentTokens::FloatingActionButton(t) => t,
            _ => unreachable!(),
        };
        let size = tokens.size;
        let radius = tokens.border_radius;
        let elevation = tokens.elevation;

        let content: AnyView = if extended {
            Stack::horizontal_from_iter(vec![
                child,
                Label::new(label)
                    .style(move |s| {
                        let t = theme.get();
                        s.color(t.colors().on_primary.to_peniko())
                            .font_size(t.typography().label.large.font_size)
                            .font_weight(t.typography().label.large.font_weight.to_floom())
                    })
                    .into_any(),
            ])
            .style(move |s| {
                s.height(size)
                    .padding_horiz(20.0)
                    .gap(8.0)
                    .items_center()
                    .justify_center()
            })
            .into_any()
        } else {
            Container::new(child)
                .style(move |s| s.size(size, size).items_center().justify_center())
                .into_any()
        };

        Container::new(content)
            .style(move |s| {
                let t = theme.get();
                let bg = if disabled.get() {
                    with_alpha(t.colors().on_surface, 12)
                } else {
                    t.colors().primary
                };
                let fg = if disabled.get() {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().on_primary
                };
                s.background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .border_radius(radius)
                    .items_center()
                    .justify_center()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled.get())
                    .box_shadow_blur(2.0 + 1.0 * (elevation as f64))
                    .box_shadow_spread(1.0)
                    .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 30).to_peniko())
            })
            .action(move || {
                if !disabled.get() {
                    if let Some(cb) = &on_click {
                        cb();
                    }
                }
            })
    }
}

impl Default for MdFloatingActionButton {
    fn default() -> Self {
        Self::new()
    }
}
