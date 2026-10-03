use std::rc::Rc;
use std::time::Duration;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::IntoView;
use floom::reactive::create_effect;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::snackbar::SnackbarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentSnackbarStyle {
    pub design_system: &'static str,
    pub surface_color: Color,
    pub text_color: Color,
    pub action_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
    pub action_gap: f32,
}

pub struct FluentSnackbar {
    props: SnackbarProps,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl FluentSnackbar {
    pub fn new() -> Self {
        let props = SnackbarProps::default();
        Self {
            open: RwSignal::new(false),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            props,
        }
    }
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }
    pub fn action_label(mut self, action_label: Option<String>) -> Self {
        self.props.action_label = action_label;
        self
    }
    pub fn on_action(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_action = Some(Box::new(callback));
        self
    }
    pub fn duration_ms(mut self, duration_ms: u32) -> Self {
        self.props.duration_ms = duration_ms;
        self
    }
    pub fn props(&self) -> &SnackbarProps {
        &self.props
    }
    /// Expose the internal open signal so callers can show or dismiss the snackbar.
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    /// Set the internal `open` signal from an external signal.
    pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
        self.open = open;
        self
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentSnackbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Snackbar(tokens) = theme.component(ComponentId::Snackbar) else {
            unreachable!("ComponentId::Snackbar must resolve to SnackbarTokens")
        };
        FluentSnackbarStyle {
            design_system: DESIGN_SYSTEM,
            surface_color: with_alpha(theme.colors().on_surface, 235),
            text_color: theme.colors().surface,
            action_color: theme.colors().primary,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
            action_gap: tokens.action_gap,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let message = self.props.message.clone();
        let action_label = self.props.action_label.clone();
        let on_action = Rc::new(self.props.on_action);
        let duration_ms = self.props.duration_ms;

        let tokens = match theme.get().component(ComponentId::Snackbar) {
            ComponentTokens::Snackbar(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = theme.get().radius().sm;
        let min_height = tokens.min_height;
        let font_size = tokens.font_size;
        let max_width = tokens.max_width;
        let action_gap = tokens.action_gap;

        // Auto-dismiss after `duration_ms` when it is positive.
        if duration_ms > 0 {
            create_effect(move |_| {
                if open.get() {
                    let open2 = open;
                    let d = duration_ms;
                    floom::action::exec_after(std::time::Duration::from_millis(d as u64), move |_token| {
                        open2.set(false);
                    });
                }
            });
        }

        let msg_view: AnyView = Label::new(message)
            .style(move |s| {
                let t = theme.get();
                s.color(t.colors().surface.to_peniko())
                    .font_size(font_size)
            })
            .into_any();

        let action_view: AnyView = match action_label {
            Some(lbl) => {
                let on_action = on_action.clone();
                let open = open;
                Label::new(lbl)
                    .style(move |s| {
                        let t = theme.get();
                        s.color(t.colors().primary.to_peniko())
                            .font_size(font_size)
                            .font_weight(FontWeight::SemiBold.to_floom())
                            .padding_horiz(8.0)
                            .cursor(CursorStyle::Pointer)
                    })
                    .on_event(listener::Click, move |_, _| {
                        if let Some(cb) = on_action.as_ref() {
                            cb();
                        }
                        open.set(false);
                        EventPropagation::Continue
                    })
                    .into_any()
            }
            None => Empty::new().into_any(),
        };

        let stack = Stack::horizontal((msg_view, action_view)).style(move |s| {
            let _t = theme.get();
            s.gap(action_gap).items_center()
        });

        let surface = Container::new(stack)
            .style(move |s| {
                let t = theme.get();
                s.padding(padding)
                    .min_height(min_height)
                    .max_width(max_width)
                    .background(with_alpha(t.colors().on_surface, 235).to_peniko())
                    .color(t.colors().surface.to_peniko())
                    .border_radius(radius)
                    .box_shadow_blur(16.0)
                    .box_shadow_spread(1.0)
                    .box_shadow_color(with_alpha(BLACK, 25).to_peniko())
            })
            // Slide up from the bottom edge and fade in; reverse on dismiss.
            .animation(|a| {
                a.run_on_create(true)
                    .run_on_remove(true)
                    .duration(Duration::from_millis(250))
                    .keyframe(0, |f| f.style(|s| s.translate_y(40.0).opacity(0.0)))
                    .keyframe(100, |f| f.computed_style())
            });

        let positioned = Container::new(surface)
            .style(move |s| {
                s.absolute()
                    .inset_bottom(24.0)
                    .inset_left(0.0)
                    .width_pct(100.0)
                    .items_center()
                    .justify_center()
                    .apply_if(!open.get(), |s| s.hide())
            });

        Overlay::new(positioned)
    }
}

impl Default for FluentSnackbar {
    fn default() -> Self {
        Self::new()
    }
}
