//! The wiring layer: platform events in, pixels out.
//!
//! Everything else in this workspace is a piece. This module is where the pieces
//! are connected, and it is deliberately the *only* place that knows about all
//! of them:
//!
//! ```text
//!   Platform ──events──▶ AppRunner ──dispatch──▶ WidgetHost
//!                           │                       │
//!                           │◀────────Scene─────────┘
//!                           ▼
//!                       Presenter ──▶ swap-chain / CPU buffer
//! ```
//!
//! ## Why the runner does not own the platform
//!
//! A callback-driven backend (winit) owns its event loop and *lends* the
//! platform to the app inside a callback; a poll-driven backend (the fake one,
//! embedded framebuffers) is owned by the app. Rather than pick a side,
//! [`AppRunner::pump`] takes `&mut P` for the call, so the same runner works with
//! both and no backend type appears in its signature.
//!
//! ## What one pump does
//!
//! 1. Drain the platform queue into a local buffer.
//! 2. For each event: update host size, (re)attach or resize the presentation
//!    surface, or translate input into a `floem-types` event and dispatch it.
//! 3. On `RedrawRequested`: run `WidgetHost::frame()` and present the resulting
//!    scene.
//!
//! Steps run in queue order, so a resize and the pointer events that follow it
//! see the new layout.

use std::collections::HashMap;

use floem_error::{ErrorBoundary, Outcome};
use floem_platform::{Platform, PlatformEvent, SurfaceTarget};
use floem_renderer::{AttachedSurface, Presenter, SurfaceHandles};
use floem_services::{Scope, Services};
use floem_types::{
    Event, ImeUpdate, Key, KeyEvent, KeyPhase, Modifiers, Point, PointerEvent, PointerKind,
    PointerPhase, Rect, ScrollEvent, Size, SurfaceId, WindowId,
};

use crate::{FrameStats, WidgetHost};

/// Running totals for one runner, for tests and diagnostics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunStats {
    /// Platform events processed.
    pub events: u64,
    /// Frames presented.
    pub frames: u64,
    /// Pointer/keyboard events dispatched into the widget tree.
    pub input_dispatched: u64,
    pub windows_created: u64,
    pub windows_closed: u64,
    /// Redraws the runner asked the platform for.
    pub redraws_requested: u64,
    /// Of those, the ones asked for by the frame loop itself because the scene
    /// was still changing (a cascade: new text → re-measure → re-layout → new
    /// paint). Content-driven redraw settles to zero on a static tree.
    pub auto_redraws: u64,
    /// Failures the error boundary caught and downgraded to a fallback.
    pub errors_caught: u64,
}

/// Per-window bookkeeping the runner needs across events.
#[derive(Debug, Clone, Copy)]
struct WindowRecord {
    surface: Option<SurfaceId>,
    size: Size,
    scale: f64,
}

/// Connects a platform, a widget host and a presenter.
///
/// The runner is also the app shell: it owns the app-scoped service registry
/// (with a child scope per window) and, optionally, the [`ErrorBoundary`] that
/// turns a panicking widget callback into a recorded report instead of a dead
/// process.
pub struct AppRunner<R: Presenter> {
    host: WidgetHost,
    presenter: R,
    windows: HashMap<u32, WindowRecord>,
    order: Vec<WindowId>,
    /// The window input is routed to and output is presented into.
    primary: Option<WindowId>,
    modifiers: Modifiers,
    stats: RunStats,
    presenter_ready: bool,
    /// App-scoped services; outlives every window.
    scope: Scope,
    /// One child scope per live window, disposed when the window closes.
    window_scopes: HashMap<u32, Scope>,
    /// Optional boundary over widget callbacks (see [`Self::with_error_boundary`]).
    boundary: Option<ErrorBoundary>,
}

impl<R: Presenter> AppRunner<R> {
    /// Wrap an already-populated host.
    pub fn new(host: WidgetHost, presenter: R) -> Self {
        Self {
            host,
            presenter,
            windows: HashMap::new(),
            order: Vec::new(),
            primary: None,
            modifiers: Modifiers::NONE,
            stats: RunStats::default(),
            presenter_ready: false,
            scope: Scope::root(),
            window_scopes: HashMap::new(),
            boundary: None,
        }
    }

    /// Install an error boundary over widget callbacks.
    ///
    /// Without one, a panicking `on_click` (or any widget event handler)
    /// unwinds through the event loop and kills the process. With one, the
    /// panic is captured into a [`floem_error::Report`] — classified by the
    /// app's classifier; a *fatal* verdict still re-raises — and the runner
    /// keeps going. Read reports back via [`Self::boundary`].
    pub fn with_error_boundary(mut self, boundary: ErrorBoundary) -> Self {
        self.boundary = Some(boundary);
        self
    }

    /// The installed boundary, if any (for reading or clearing reports).
    pub fn boundary(&self) -> Option<&ErrorBoundary> {
        self.boundary.as_ref()
    }

    /// The app-scoped service registry. Services registered here resolve from
    /// every window scope and live for the runner's lifetime.
    pub fn services(&self) -> &Services {
        self.scope.services()
    }

    /// The window-scoped service registry: a child of [`Self::services`],
    /// created lazily and disposed when the window closes.
    ///
    /// Returns the whole [`Scope`], not just its own registry: resolution via
    /// [`Scope::get`] falls back through the parent chain, while provisioning
    /// goes through [`Scope::services`]. Route/dialog-level scoping belongs to
    /// the app (see [`Scope::child`]).
    pub fn window_scope(&mut self, window: WindowId) -> &Scope {
        self.window_scopes
            .entry(window.index())
            .or_insert_with(|| Scope::child(self.scope.services()))
    }

    pub fn host(&self) -> &WidgetHost {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut WidgetHost {
        &mut self.host
    }

    pub fn presenter(&self) -> &R {
        &self.presenter
    }

    pub fn presenter_mut(&mut self) -> &mut R {
        &mut self.presenter
    }

    pub fn stats(&self) -> RunStats {
        self.stats
    }

    /// The window input is routed to, once one exists.
    pub fn primary_window(&self) -> Option<WindowId> {
        self.primary
    }

    /// Whether the presentation surface is currently live.
    pub fn presenter_ready(&self) -> bool {
        self.presenter_ready
    }

    /// Logical size of the primary window.
    pub fn window_size(&self) -> Option<Size> {
        let window = self.primary?;
        self.windows.get(&window.index()).map(|w| w.size)
    }

    /// Drain the platform queue and process every event, in order.
    ///
    /// This is the frame hook: call it from a winit callback, or in a loop for a
    /// poll-driven backend.
    pub fn pump<P: Platform>(&mut self, platform: &mut P) -> RunStats {
        let mut events = Vec::new();
        platform.poll_events(&mut events);
        for event in events {
            self.stats.events += 1;
            self.handle(platform, event);
        }
        self.stats
    }

    /// Process one platform event.
    pub fn handle<P: Platform>(&mut self, platform: &mut P, event: PlatformEvent) {
        match event {
            PlatformEvent::WindowCreated {
                window,
                surface,
                size,
            } => {
                let scale = platform
                    .window(window)
                    .map(|w| w.display_info().scale_factor.get())
                    .unwrap_or(1.0);
                self.windows.insert(
                    window.index(),
                    WindowRecord {
                        surface: Some(surface),
                        size,
                        scale,
                    },
                );
                if !self.order.contains(&window) {
                    self.order.push(window);
                }
                self.primary.get_or_insert(window);
                // Window-scoped services live from creation to close.
                self.window_scopes
                    .entry(window.index())
                    .or_insert_with(|| Scope::child(self.scope.services()));
                // The widget tree lays out in logical pixels; the window's
                // logical size *is* the host's viewport.
                self.host.set_size(size);
                self.attach(platform, window);
                platform.request_frame(window);
                self.stats.redraws_requested += 1;
                self.stats.windows_created += 1;
            }
            PlatformEvent::WindowResized { window, size } => {
                if let Some(record) = self.windows.get_mut(&window.index()) {
                    record.size = size;
                    let scale = record.scale;
                    self.host.set_size(size);
                    self.presenter.resize(size, scale);
                }
                platform.request_frame(window);
                self.stats.redraws_requested += 1;
            }
            PlatformEvent::WindowClosed { window } => {
                self.windows.remove(&window.index());
                self.order.retain(|w| *w != window);
                // Dropping the scope disposes the window's own services
                // (children of the app scope; the app scope itself outlives
                // every window).
                self.window_scopes.remove(&window.index());
                self.stats.windows_closed += 1;
                if self.primary == Some(window) {
                    self.presenter.detach();
                    self.presenter_ready = false;
                    self.primary = self.order.first().copied();
                    if let Some(next) = self.primary {
                        self.attach(platform, next);
                    }
                }
            }
            PlatformEvent::WindowFocused { window, .. } => {
                // Focus can change hover/selection rendering.
                self.request_repaint(platform, window);
            }
            PlatformEvent::ModifiersChanged { modifiers, .. } => {
                self.modifiers = modifiers;
            }
            PlatformEvent::PointerMoved {
                window,
                position,
                modifiers,
            } => {
                self.dispatch(
                    PointerEvent::hover(position)
                        .with_modifiers(modifiers)
                        .into(),
                    "pointer move",
                );
                self.request_repaint(platform, window);
            }
            PlatformEvent::PointerButton {
                window,
                position,
                button,
                pressed,
                modifiers,
            } => {
                let phase = if pressed {
                    PointerPhase::Down
                } else {
                    PointerPhase::Up
                };
                self.dispatch(
                    PointerEvent {
                        phase,
                        kind: PointerKind::Mouse,
                        button: Some(map_button(button)),
                        position,
                        modifiers,
                        click_count: 1,
                    }
                    .into(),
                    "pointer button",
                );
                self.request_repaint(platform, window);
            }
            PlatformEvent::Scroll {
                window,
                position,
                delta_x,
                delta_y,
                modifiers,
            } => {
                self.dispatch(
                    Event::Scroll(ScrollEvent {
                        delta_x,
                        delta_y,
                        position,
                        modifiers,
                    }),
                    "scroll",
                );
                self.request_repaint(platform, window);
            }
            PlatformEvent::Key {
                window,
                code,
                pressed,
                repeat,
                modifiers,
            } => {
                if let Some(key) = code.to_key() {
                    let phase = match (pressed, repeat) {
                        (true, true) => KeyPhase::Repeat,
                        (true, false) => KeyPhase::Down,
                        (false, _) => KeyPhase::Up,
                    };
                    self.dispatch(
                        KeyEvent {
                            phase,
                            key,
                            modifiers,
                        }
                        .into(),
                        "keyboard",
                    );
                    self.request_repaint(platform, window);
                }
            }
            PlatformEvent::ImeCommit { window, text } => {
                // The ABI has no "text" event: committed text is delivered as
                // one character key-down per scalar, which is exactly what
                // `KeyEvent::is_text_input` describes, so a text field consumes
                // it through the same path it uses for ordinary typing.
                for ch in text.chars() {
                    self.dispatch(KeyEvent::down(Key::Char(ch)).into(), "ime commit");
                }
                // The composition just finalized, so dismiss any IME popup the
                // platform is showing.
                self.push_ime_cursor_area(platform, window, None);
                self.request_repaint(platform, window);
            }
            PlatformEvent::ImePreedit { window, text } => {
                // Surface the in-progress composition to the focused text
                // widget, which owns the IME state machine and renders the
                // preedit. The caret rectangle it reports back anchors the OS
                // input-method popup above the right glyph.
                self.dispatch(
                    Event::Ime(ImeUpdate::Preedit {
                        text: text.clone(),
                        caret: text.len(),
                        selection: None,
                    }),
                    "ime preedit",
                );
                self.push_ime_cursor_area(platform, window, self.host.ime_caret_rect());
                self.request_repaint(platform, window);
            }
            PlatformEvent::RedrawRequested { window } => {
                // A resize or a resume between the request and its delivery can
                // leave us without a live surface; rebuild before painting.
                if !self.presenter_ready {
                    self.attach(platform, window);
                }
                let _ = self.render();
                // One frame is often not the whole answer: painting can dirty
                // layout (a re-measured text leaf), and unmounts can owe the
                // scene a node removal. Keep asking while there is work left,
                // which lets a cascade settle across frames instead of
                // stopping halfway — and stops on its own once it has.
                if self.host.needs_frame() {
                    self.request_repaint(platform, window);
                    self.stats.auto_redraws += 1;
                }
            }
            PlatformEvent::Suspended => {
                // The OS is taking the surface away (Android pause, display
                // removal). Drop the swap-chain now, rebuild on the next
                // `WindowCreated` or redraw.
                self.presenter.detach();
                self.presenter_ready = false;
            }
            PlatformEvent::Resumed => {
                // Surfaces were rebuilt by the OS; schedule a redraw per window
                // so the redraw arm re-attaches and repaints.
                let windows: Vec<WindowId> = self.order.clone();
                for window in windows {
                    platform.request_frame(window);
                    self.stats.redraws_requested += 1;
                }
            }
        }
    }

    /// Run one frame and present it.
    fn render(&mut self) -> FrameStats {
        let host = &mut self.host;
        let presenter = &mut self.presenter;
        let ready = self.presenter_ready;
        let frame_stats = host.frame();
        if ready {
            presenter.present(host.scene(), host.resources());
            self.stats.frames += 1;
        }
        frame_stats
    }

    /// Dispatch one input event into the widget tree.
    ///
    /// With an error boundary installed (see [`Self::with_error_boundary`]) the
    /// dispatch runs inside `boundary.capture`, so a panicking widget callback
    /// becomes a recorded report + fallback instead of an unwind through the
    /// event loop. The boundary's own classifier decides recoverable vs fatal;
    /// a fatal verdict re-raises from inside `capture`.
    fn dispatch(&mut self, event: floem_types::Event, context: &str) {
        if let Some(boundary) = self.boundary.clone() {
            let host = &mut self.host;
            let outcome = boundary.in_(context.to_string()).capture(move || {
                host.dispatch(event);
                Ok(())
            });
            if outcome == Outcome::Fallback {
                self.stats.errors_caught += 1;
            }
        } else {
            self.host.dispatch(event);
        }
        self.stats.input_dispatched += 1;
    }

    /// Schedules a frame on the primary window.
    ///
    /// This is the door for anything that changes the tree outside the frame
    /// loop — a background service, a timer, a callback from another thread —
    /// and has no platform event of its own to wake the loop. Frames requested
    /// this way still settle: the loop stops asking once the tree stops
    /// changing (see the `RedrawRequested` arm).
    pub fn request_frame<P: Platform>(&mut self, platform: &mut P) {
        if let Some(window) = self.primary {
            self.request_repaint(platform, window);
        }
    }

    /// Schedules a repaint after something visual may have changed.
    ///
    /// In a real window this turns into `request_redraw`; on a poll-driven
    /// backend it queues a `RedrawRequested` for the next pump. Idempotent per
    /// frame — the platform coalesces redraw requests.
    fn request_repaint<P: Platform>(&mut self, platform: &mut P, window: WindowId) {
        if self.windows.contains_key(&window.index()) {
            platform.request_frame(window);
            self.stats.redraws_requested += 1;
        }
    }

    /// Hands the focused text widget's composition caret rect to the platform's
    /// IME bridge, so the native input-method popup sits above the right glyph.
    /// `None` dismisses any area the platform is currently showing (the
    /// composition ended).
    fn push_ime_cursor_area<P: Platform>(
        &mut self,
        platform: &mut P,
        window: WindowId,
        area: Option<Rect>,
    ) {
        if let Some(ime) = platform.ime(window) {
            match area {
                Some(rect) => ime.set_cursor_area(
                    window,
                    Point::new(rect.x0, rect.y0),
                    Size::new(rect.width(), rect.height()),
                ),
                None => ime.set_cursor_area(window, Point::new(0.0, 0.0), Size::new(0.0, 0.0)),
            }
        }
    }

    /// Attach (or re-attach) the presenter to a window's surface.
    fn attach<P: Platform>(&mut self, platform: &P, window: WindowId) {
        let Some(record) = self.windows.get(&window.index()).copied() else {
            return;
        };
        let handles = platform
            .window(window)
            .and_then(|w| w.surface_target())
            .map(to_handles);
        let attached = self.presenter.attach(AttachedSurface::new(
            window,
            record.surface.unwrap_or_else(|| SurfaceId::new(0, 1)),
            record.size,
            record.scale,
            handles,
        ));
        self.presenter_ready = attached;
    }
}

/// Destructures a platform [`SurfaceTarget`] into the renderer's handle pair.
///
/// This is the *only* place the two ABIs meet, which keeps `floem-renderer` free
/// of any platform dependency: it never names [`SurfaceTarget`], only the two
/// handle values carried inside it.
fn to_handles(target: SurfaceTarget) -> SurfaceHandles {
    SurfaceHandles {
        window: target.window_handle,
        display: target.display_handle,
    }
}

/// Folds the platform's extended button set into the widget contract's three.
///
/// `Auxiliary` is the middle button by another name; the back/forward pair and
/// any exotic device buttons act as `Primary` for now (the contract has no
/// identity for them yet).
fn map_button(button: floem_platform::PointerButton) -> floem_types::PointerButton {
    use floem_platform::PointerButton as P;
    match button {
        P::Primary => floem_types::PointerButton::Primary,
        P::Secondary => floem_types::PointerButton::Secondary,
        P::Auxiliary | P::Other(_) => floem_types::PointerButton::Middle,
    }
}
