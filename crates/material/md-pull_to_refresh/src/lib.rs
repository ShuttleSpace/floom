use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, v_stack, text};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::pull_to_refresh::PullToRefreshProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

/// Material 3 pull-to-refresh wrapper (Vuetify `VPullToRefresh`).
///
/// Detects a downward drag on the wrapped content and, once it passes `threshold`,
/// fires `on_refresh` and shows a refreshing indicator. The host clears the
/// refresh state via `refreshing_signal()` (there is no built-in timer in this
/// Floem build).
pub struct MdPullToRefresh {
    props: PullToRefreshProps,
    child: Option<Box<dyn Fn() -> AnyView>>,
    refreshing: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl MdPullToRefresh {
    pub fn new() -> Self {
        let props = PullToRefreshProps::default();
        Self {
            refreshing: RwSignal::new(props.loading),
            props,
            child: None,
            theme: theme_signal(),
        }
    }

    pub fn child(mut self, child: impl Fn() -> AnyView + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self.refreshing.set(loading);
        self
    }

    pub fn threshold(mut self, threshold: f32) -> Self {
        self.props.threshold = threshold;
        self
    }

    pub fn on_refresh(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_refresh = Some(Box::new(callback));
        self
    }

    /// Component-owned refresh state; the host sets it `false` when done.
    pub fn refreshing_signal(&self) -> RwSignal<bool> {
        self.refreshing
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let threshold = self.props.threshold;
        let max_pull = match theme.get().component(ComponentId::PullToRefresh) {
            ComponentTokens::PullToRefresh(t) => t.max_pull,
            _ => unreachable!(),
        };
        let on_refresh = Rc::new(self.props.on_refresh);
        let child_factory = self.child;
        let refreshing = self.refreshing;

        let pull: RwSignal<f32> = RwSignal::new(0.0);
        let dragging: RwSignal<bool> = RwSignal::new(false);
        let start_y: RwSignal<f32> = RwSignal::new(0.0);

        let indicator = dyn_view(move || {
            let p = pull.get();
            let is_refreshing = refreshing.get();
            if p <= 0.5 && !is_refreshing {
                return Empty::new().into_any();
            }
            let (label, h) = if is_refreshing {
                ("↻ Refreshing…", 28.0)
            } else {
                ("↓ Pull to refresh", p.min(28.0))
            };
            text(label)
                .style(move |s| {
                    let t = theme.get();
                    s.height(h)
                        .font_size(13.0)
                        .color(t.colors().on_surface_variant.to_peniko())
                        .items_center()
                        .justify_center()
                })
                .into_any()
        });

        let child_view: AnyView = match child_factory {
            Some(f) => f(),
            None => Empty::new().into_any(),
        };

        let content = v_stack((indicator, child_view));

        Container::new(content)
            .style(move |s| {
                let t = theme.get();
                s.width_pct(100.0)
                    .background(t.colors().background.to_peniko())
                    .items_start()
            })
            .on_event(listener::PointerDown, {
                let dragging = dragging;
                let start_y = start_y;
                let pull = pull;
                move |cx, pbe| {
                    let y = pbe.state.logical_point().y as f32;
                    start_y.set(y);
                    dragging.set(true);
                    if let Some(pid) = pbe.pointer.pointer_id {
                        cx.request_pointer_capture(pid);
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerMove, {
                let dragging = dragging;
                let start_y = start_y;
                let pull = pull;
                let max_pull = max_pull;
                move |_, pbe| {
                    if dragging.get() {
                        let y = pbe.current.logical_point().y as f32;
                        let dy = y - start_y.get();
                        if dy > 0.0 {
                            pull.set(dy.min(max_pull));
                        } else {
                            pull.set(0.0);
                        }
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerUp, {
                let dragging = dragging;
                let pull = pull;
                let threshold = threshold;
                let refreshing = refreshing;
                let on_refresh = on_refresh;
                move |_, _| {
                    if dragging.get() {
                        dragging.set(false);
                        if pull.get() >= threshold {
                            refreshing.set(true);
                            if let Some(cb) = on_refresh.as_ref() {
                                cb();
                            }
                        }
                        pull.set(0.0);
                    }
                    EventPropagation::Continue
                }
            })
    }
}

impl Default for MdPullToRefresh {
    fn default() -> Self {
        Self::new()
    }
}
