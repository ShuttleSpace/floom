use std::rc::Rc;
use std::time::Duration;

use floem::action::exec_after;
use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::{create_effect, RwSignal, SignalGet, SignalUpdate};
use floem::view::AnyView;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::snackbar::SnackbarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::typography::FontWeight;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSnackbarStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub on_surface: Color,
    pub action_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
    pub action_gap: f32,
}

pub struct MdSnackbar {
    props: SnackbarProps,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl MdSnackbar {
    pub fn new() -> Self {
        let props = SnackbarProps::default();
        Self {
            open: RwSignal::new(false),
            theme: theme_signal(),
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

    /// Expose the internal open signal so a host can show or hide the snackbar.
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    pub fn resolved_style(&self) -> MdSnackbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Snackbar(tokens) = theme.component(ComponentId::Snackbar) else {
            unreachable!("ComponentId::Snackbar must resolve to SnackbarTokens")
        };
        MdSnackbarStyle {
            design_system: "material",
            surface: theme.colors().on_surface,
            on_surface: theme.colors().surface,
            action_color: theme.colors().primary,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
            action_gap: tokens.action_gap,
        }
    }

    /// Build a renderable, interactive Material 3 snackbar shown bottom-center.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let message = self.props.message.clone();
        let action_label = self.props.action_label.clone();
        let on_action: Rc<Option<Box<dyn Fn()>>> = Rc::new(self.props.on_action);
        let duration_ms = self.props.duration_ms;

        let tokens = match theme.get().component(ComponentId::Snackbar) {
            ComponentTokens::Snackbar(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = tokens.border_radius;
        let min_height = tokens.min_height;
        let font_size = tokens.font_size;
        let max_width = tokens.max_width;
        let gap = tokens.action_gap;

        // Auto-dismiss after `duration_ms` (main-thread timer; RwSignal is !Send).
        if duration_ms > 0 {
            let open_sig = open;
            create_effect(move |_| {
                if open_sig.get() {
                    let sig = open_sig;
                    exec_after(Duration::from_millis(duration_ms as u64), move |_| {
                        sig.set(false);
                    });
                }
            });
        }

        let message_view = text(message).style(move |s| {
            let t = theme.get();
            s.color(t.colors().surface.to_peniko())
                .font_size(font_size)
        });

        let action_view: AnyView = match action_label {
            Some(lbl) => text(lbl)
                .style(move |s| {
                    let t = theme.get();
                    s.color(t.colors().primary.to_peniko())
                        .font_size(t.typography().label.large.font_size)
                        .font_weight(FontWeight::Medium.to_floem())
                        .padding_horiz(8.0)
                        .cursor(CursorStyle::Pointer)
                })
                .on_event(listener::Click, {
                    let open = open;
                    let on_action = on_action.clone();
                    move |_, _| {
                        if let Some(cb) = on_action.as_ref() {
                            cb();
                        }
                        open.set(false);
                        EventPropagation::Continue
                    }
                })
                .into_any(),
            None => Empty::new().into_any(),
        };

        let surface = Container::new(
            h_stack((message_view, action_view)).style(move |s| s.items_center().gap(gap)),
        )
        .style(move |s| {
            let t = theme.get();
            s.min_height(min_height)
                .max_width(max_width)
                .padding(padding)
                .border_radius(radius)
                .background(t.colors().on_surface.to_peniko())
                .color(t.colors().surface.to_peniko())
                .items_center()
                .apply_if(!open.get(), |s| s.hide())
        });

        let overlay = Container::new(surface).style(move |s| {
            s.width_pct(100.0)
                .height_pct(100.0)
                .items_center()
                .justify_end()
                .padding_bottom(24.0)
                .apply_if(!open.get(), |s| s.hide())
        });

        Overlay::new(overlay)
    }
}

impl Default for MdSnackbar {
    fn default() -> Self {
        Self::new()
    }
}
