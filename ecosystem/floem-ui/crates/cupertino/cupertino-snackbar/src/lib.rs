use std::rc::Rc;
use std::time::Duration;

use floem::event::EventPropagation;
use floem::ext_event::ArcRwSignal;
use floem::prelude::*;
use floem::reactive::{RwSignal, SignalGet, create_effect};
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::snackbar::SnackbarProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoSnackbarStyle {
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

pub struct CupertinoSnackbar {
    props: SnackbarProps,
    open: RwSignal<bool>,
    /// `Send + Sync` mirror used by the auto-dismiss timer thread.
    open_sync: ArcRwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSnackbar {
    pub fn new() -> Self {
        let props = SnackbarProps::default();
        Self {
            open: RwSignal::new(false),
            open_sync: ArcRwSignal::new(false),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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
    /// Expose the internal open signal so an external trigger can show or hide the snackbar.
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
    pub fn resolved_style(&self) -> CupertinoSnackbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Snackbar(tokens) = theme.component(ComponentId::Snackbar) else {
            unreachable!("ComponentId::Snackbar must resolve to SnackbarTokens")
        };
        CupertinoSnackbarStyle {
            design_system: DESIGN_SYSTEM,
            surface_color: theme.colors().on_surface,
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

    /// Build a renderable Cupertino snackbar positioned bottom-center.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let open_sync = self.open_sync.clone();
        let duration = self.props.duration_ms;
        let on_action = Rc::new(self.props.on_action);
        let action_label = self.props.action_label.clone();
        let message = self.props.message.clone();

        // --- Auto-dismiss wiring (RwSignal is not Send, so a Sync mirror drives the thread) ---
        // Mirror open -> open_sync so opening via open_signal() also arms the timer.
        create_effect({
            let open = open;
            let open_sync = open_sync.clone();
            move |_| {
                let v = open.get();
                if open_sync.get_untracked() != v {
                    open_sync.set(v);
                }
            }
        });
        // Mirror open_sync -> open: the timer thread flips open_sync, which closes the snackbar.
        create_effect({
            let open = open;
            let open_sync = open_sync.clone();
            move |_| {
                open_sync.track();
                let v = open_sync.get_untracked();
                if open.get() != v {
                    open.set(v);
                }
            }
        });
        // Arm a background timer whenever the snackbar opens.
        create_effect({
            let open = open;
            let open_sync = open_sync.clone();
            move |_| {
                if open.get() && duration > 0 {
                    let sync = open_sync.clone();
                    let dur = duration as u64;
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(dur));
                        sync.set(false);
                    });
                }
            }
        });

        let tokens = match theme.get().component(ComponentId::Snackbar) {
            ComponentTokens::Snackbar(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = theme.get().radius().lg;
        let min_height = tokens.min_height;
        let max_width = tokens.max_width;
        let font_size = tokens.font_size;
        let action_gap = tokens.action_gap;

        let has_action = action_label.is_some();
        let action_label_str = action_label.unwrap_or_default();

        let action_view = Label::new(action_label_str)
            .style(move |s| {
                let t = theme.get();
                s.color(t.colors().primary.to_peniko())
                    .font_size(font_size)
                    .font_weight(FontWeight::Medium.to_floem())
                    .padding_horiz(8.0)
                    .cursor(CursorStyle::Pointer)
                    .apply_if(!has_action, |s| s.hide())
            })
            .on_event(listener::Click, {
                let on_action = on_action.clone();
                let open_sig = open;
                move |_, _| {
                    if let Some(cb) = on_action.as_ref() {
                        cb();
                    }
                    open_sig.set(false);
                    EventPropagation::Continue
                }
            });

        let surface = Container::new(
            Stack::horizontal((Label::new(message).style(move |s| {
                let t = theme.get();
                s.color(t.colors().surface.to_peniko())
                    .font_size(font_size)
                    .flex_basis(1.0)
                    .flex_grow(1.0)
            }), action_view))
            .style(move |s| {
                let t = theme.get();
                s.gap(action_gap)
                    .padding(padding)
                    .min_height(min_height)
                    .max_width(max_width)
                    .background(t.colors().on_surface.to_peniko())
                    .border_radius(radius)
                    .items_center()
                    .box_shadow_blur(16.0)
                    .box_shadow_spread(1.0)
                    .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 40).to_peniko())
            }),
        )
        // Slide up from the bottom edge and fade in; reverse on dismiss.
        .animation(|a| {
            a.run_on_create(true)
                .run_on_remove(true)
                .duration(Duration::from_millis(250))
                .keyframe(0, |f| f.style(|s| s.translate_y(40.0).opacity(0.0)))
                .keyframe(100, |f| f.computed_style())
        });

        let layer = Container::new(surface).style(move |s| {
            let _t = theme.get();
            s.width_pct(100.0)
                .height_pct(100.0)
                .items_end()
                .justify_center()
                .padding(16.0)
                .apply_if(!open.get(), |s| s.hide())
        });

        Overlay::new(layer)
    }
}

impl Default for CupertinoSnackbar {
    fn default() -> Self {
        Self::new()
    }
}
