// SPDX-License-Identifier: MIT
//! A real winit-backed [`Platform`].
//!
//! This is the crate's reason to exist: the rest of `floom-platform-desktop`
//! maps *samples* of desktop input onto the ABI, while this module owns an
//! actual OS window and event loop.
//!
//! ## Shape of the integration
//!
//! winit owns the main thread and calls *us*; the ABI is poll-based
//! ([`Platform::poll_events`]). The two are reconciled by making the backend
//! push into a queue as winit events arrive and then handing that queue to the
//! application at well-defined points. [`WinitApp`] is those points:
//!
//! ```text
//! winit callback            →  what the backend does
//! ────────────────────────────────────────────────────────────────────────────
//! can_create_surfaces       →  materialise queued windows, emit WindowCreated
//! window_event(..)          →  translate to PlatformEvents, then app.events()
//! about_to_wait             →  app.about_to_wait() (idle hook / timers)
//! exiting                   →  app.exiting()
//! ```
//!
//! `app.events(&mut platform)` is the "run a frame" hook: it drains
//! [`Platform::poll_events`] and does whatever the app does with them — in the
//! app shell, run `WidgetHost::frame()` and present. Because the window queue is
//! drained there, an app never has to know how winit batches its callbacks.
//!
//! ## Window creation is deferred
//!
//! [`ActiveEventLoop`] is only available inside a callback, so
//! [`Platform::create_window`] records the request and returns a
//! [`WindowId`] immediately; the window is created on the next
//! `can_create_surfaces` / `resumed`. Callers that already hold an
//! [`ActiveEventLoop`] can skip the queue with
//! [`WinitPlatform::create_window_now`].
//!
//! ## Not yet wired
//!
//! * Clipboard off macOS is still an in-process string — see the note on
//!   [`Clipboard for WinitPlatform`](Self#impl-Clipboard-for-WinitPlatform):
//!   macOS reaches the system pasteboard through [`crate::clipboard`], and the
//!   other desktop targets have not been given that bridge yet.
//! * macOS menu-bar / app-delegate integration lives in the legacy crate.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use floom_platform::{
    Clipboard, Ime, KeyCode, Modifiers, Platform, PlatformCapabilities, PlatformEvent,
    SurfaceConfig, SurfaceTarget, Window, WindowCommand, WindowConfig, WindowState,
};
use floom_types::{
    CursorStyle, DisplayInfo, KeyEvent, KeyPhase, PlatformId, Point, ScaleFactor, Size, SurfaceId,
    WindowId,
};

use winit::application::ApplicationHandler;
use winit::event::{Ime as WinitIme, StartCause, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{
    ImeCapabilities, ImeEnableRequest, ImeRequest, ImeRequestData, Window as WinitWindow,
    WindowAttributes, WindowId as WinitWindowId,
};

// The `floom-winit` fork keeps `CursorIcon` private -- it does
// `use cursor_icon::CursorIcon;` internally -- so naming a shape means naming the
// `cursor-icon` crate itself. That is already a transitive dependency of the fork
// at one version in `Cargo.lock`, so this adds a line rather than a download, and
// Cargo unifies the two references to the same type.
use cursor_icon::CursorIcon;

use crate::winit_input;

/// Logical surface bookkeeping for one window.
#[derive(Debug, Clone, Copy)]
struct WinitSurface {
    id: SurfaceId,
    size: Size,
    transparent: bool,
}

/// One live OS window plus its ABI projection.
struct WinitWindowAdapter {
    id: WindowId,
    native: Arc<dyn WinitWindow>,
    title: String,
    /// Logical size; derived from the native physical size and scale factor.
    size: Size,
    scale: f64,
    state: WindowState,
    surface: Option<WinitSurface>,
    next_surface: u32,
    /// Last pointer position in logical pixels.
    ///
    /// `WindowEvent::MouseWheel` carries no position in this winit version, but
    /// `ScrollEvent` needs one to route the scroll to the element under the
    /// cursor, so the backend remembers where the pointer last was.
    last_pointer: Option<Point>,
}

impl std::fmt::Debug for WinitWindowAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinitWindowAdapter")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("size", &self.size)
            .field("scale", &self.scale)
            .field("state", &self.state)
            .field("surface", &self.surface)
            .finish()
    }
}

impl WinitWindowAdapter {
    /// Native handles for the current surface, if the OS window can supply
    /// them. Rebuilt on demand rather than cached: the handles belong to the
    /// window and change if the window is recreated (Android resume, monitor
    /// migration), so a cached copy would be a stale-pointer bug waiting to
    /// happen.
    fn target(&self) -> Option<SurfaceTarget> {
        let surface = self.surface.as_ref()?;
        let window_handle = self.native.window_handle().ok()?;
        let display_handle = self.native.display_handle().ok()?;
        Some(SurfaceTarget::new(
            surface.id,
            self.size,
            self.scale,
            window_handle.as_raw(),
            display_handle.as_raw(),
        ))
    }
}

impl Window for WinitWindowAdapter {
    fn id(&self) -> WindowId {
        self.id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn size(&self) -> Size {
        self.size
    }

    fn state(&self) -> WindowState {
        self.state
    }

    fn set_title(&mut self, title: &str) {
        self.title = title.to_string();
        self.native.set_title(title);
    }

    fn request_redraw(&mut self) {
        self.native.request_redraw();
    }

    /// Winit takes `&self`, so this stays a plain read-and-forward; the
    /// `&mut` is only there because the platform contract's other requests are
    /// all mutating.
    fn set_cursor(&mut self, cursor: CursorStyle) {
        let icon = match cursor {
            CursorStyle::Default => CursorIcon::Default,
            CursorStyle::Pointer => CursorIcon::Pointer,
            CursorStyle::Text => CursorIcon::Text,
        };
        self.native.set_cursor(icon.into());
    }

    fn surface_id(&self) -> Option<SurfaceId> {
        self.surface.as_ref().map(|s| s.id)
    }

    fn create_surface(&mut self, config: SurfaceConfig) -> SurfaceId {
        let id = SurfaceId::new(self.next_surface, 1);
        self.next_surface += 1;
        self.surface = Some(WinitSurface {
            id,
            size: config.size,
            transparent: config.transparent,
        });
        id
    }

    fn display_info(&self) -> DisplayInfo {
        DisplayInfo {
            logical_size: self.size,
            scale_factor: ScaleFactor::new(self.scale),
        }
    }

    fn surface_target(&self) -> Option<SurfaceTarget> {
        self.target()
    }

    /// Winit's setters take `&self`, so there is nothing to keep alive here —
    /// the state this adapter reports is updated from the window's own events
    /// (`Resized`, `ScaleFactorChanged`), not from the request.
    fn control(&mut self, command: WindowCommand) -> Result<(), String> {
        match command {
            WindowCommand::Resize(size) => {
                let asked = self.native.request_surface_size(winit::dpi::Size::Logical(
                    winit::dpi::LogicalSize::new(size.width, size.height),
                ));
                if asked.is_none() {
                    return Err("window control: the window refused a size request — it is \
                         fullscreen, or its size is fixed"
                        .into());
                }
            }
            WindowCommand::Move(position) => {
                self.native
                    .set_outer_position(winit::dpi::Position::Logical(
                        winit::dpi::LogicalPosition::new(position.x, position.y),
                    ));
            }
            WindowCommand::Fullscreen(on) => {
                self.native.set_fullscreen(if on {
                    Some(winit::monitor::Fullscreen::Borderless(None))
                } else {
                    None
                });
                self.state = if on {
                    WindowState::Fullscreen
                } else {
                    WindowState::Normal
                };
            }
            WindowCommand::Minimized(on) => {
                self.native.set_minimized(on);
                self.state = if on {
                    WindowState::Minimized
                } else {
                    WindowState::Normal
                };
            }
            WindowCommand::Maximized(on) => {
                self.native.set_maximized(on);
                self.state = if on {
                    WindowState::Maximized
                } else {
                    WindowState::Normal
                };
            }
            WindowCommand::AlwaysOnTop(on) => {
                self.native.set_window_level(if on {
                    winit::window::WindowLevel::AlwaysOnTop
                } else {
                    winit::window::WindowLevel::Normal
                });
            }
            WindowCommand::Decorations(on) => self.native.set_decorations(on),
            WindowCommand::Visible(on) => self.native.set_visible(on),
            WindowCommand::Focus => self.native.focus_window(),
        }
        Ok(())
    }
}

/// Desktop [`Platform`] backed by a live winit event loop.
pub struct WinitPlatform {
    id: PlatformId,
    windows: HashMap<u32, WinitWindowAdapter>,
    /// winit's opaque window id → ours, for event routing.
    native_ids: HashMap<WinitWindowId, WindowId>,
    next_window: u32,
    /// Requests made before an [`ActiveEventLoop`] was available.
    pending: VecDeque<(WindowId, WindowConfig)>,
    events: Vec<PlatformEvent>,
    /// Process-local clipboard text. **Not** read on macOS: there the
    /// system pasteboard is the clipboard, and keeping a second copy here
    /// would only let the two disagree.
    #[cfg(not(target_os = "macos"))]
    clipboard: String,
    started: Instant,
    ime_enabled: HashMap<u32, bool>,
    /// The platform capabilities to report once a native-view provider has been
    /// handed out, and `None` until then.
    ///
    /// Stored rather than recomputed from a bool, so that the flag has exactly
    /// one source: the bridge's own
    /// [`DesktopNativeBridge::platform_capabilities`](crate::DesktopNativeBridge::platform_capabilities).
    /// The distinction matters because advertising
    /// [`Capability::NATIVE_VIEW`](floom_platform::Capability::NATIVE_VIEW) makes
    /// every `NativeSurface` mount try to composite, so a session that never
    /// built a bridge must not advertise it.
    native_caps: Option<PlatformCapabilities>,
}

impl std::fmt::Debug for WinitPlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WinitPlatform")
            .field("windows", &self.windows.keys().collect::<Vec<_>>())
            .field("pending", &self.pending.len())
            .field("events", &self.events.len())
            .finish()
    }
}

impl Default for WinitPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl WinitPlatform {
    pub fn new() -> Self {
        Self {
            id: PlatformId::new(3, 1),
            windows: HashMap::new(),
            native_ids: HashMap::new(),
            next_window: 0,
            pending: VecDeque::new(),
            events: Vec::new(),
            #[cfg(not(target_os = "macos"))]
            clipboard: String::new(),
            started: Instant::now(),
            ime_enabled: HashMap::new(),
            native_caps: None,
        }
    }

    /// Build the native-view provider over `window`'s content view.
    ///
    /// The caller installs it — `WidgetHost::register_native_provider` with
    /// the kinds this bridge can really draw — because the provider is the
    /// app's to own: this crate knows how to make an AppKit one and nothing
    /// above it needs to know that AppKit was involved. Handing one out is what
    /// flips [`Platform::capabilities`](floom_platform::Platform::capabilities)
    /// to advertising [`Capability::NATIVE_VIEW`], which is why the flag is set
    /// here and not at construction.
    ///
    /// Calling this twice for one window builds a second set of views over the
    /// same parent rather than returning the first; a caller that installs both
    /// has asked for two providers, and refusing the second would be the
    /// surprising behaviour.
    #[cfg(target_os = "macos")]
    pub fn native_view_provider(
        &mut self,
        window: WindowId,
    ) -> Result<
        Box<dyn floom_native::NativeViewProvider>,
        crate::native_appkit::NativeViewUnavailable,
    > {
        use objc2::MainThreadMarker;

        use crate::DesktopNativeBridge;
        use crate::native_appkit::{MacNativeViewHost, NativeViewUnavailable};

        let parent = self.native_view_parent(window)?;
        let mtm = MainThreadMarker::new().ok_or(NativeViewUnavailable::NotMainThread)?;

        let bridge = DesktopNativeBridge::over(MacNativeViewHost::new(parent, mtm));
        self.native_caps = Some(bridge.platform_capabilities());
        Ok(Box::new(bridge))
    }

    /// The view participants would be added to, for `window`.
    ///
    /// Public as well as used by [`Self::native_view_provider`], because a
    /// device check has to be able to ask the *OS* where a participant landed,
    /// and the OS's answer starts at this view's tree. Going through the bridge
    /// instead would return the bridge's own story, which is the one thing the
    /// check is not allowed to trust.
    #[cfg(target_os = "macos")]
    pub fn native_view_parent(
        &self,
        window: WindowId,
    ) -> Result<crate::native_appkit::NativeParent, crate::native_appkit::NativeViewUnavailable>
    {
        use objc2::rc::Retained;
        use objc2_app_kit::NSView;
        use winit::raw_window_handle::RawWindowHandle;

        use crate::native_appkit::{NativeParent, NativeViewUnavailable};

        let adapter = self
            .windows
            .get(&window.index())
            .filter(|w| w.id == window)
            .ok_or(NativeViewUnavailable::UnknownWindow)?;
        let handle = adapter
            .native
            .window_handle()
            .map_err(|_| NativeViewUnavailable::NoNativeHandle)?;
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
            return Err(NativeViewUnavailable::NotAppKit);
        };
        // SAFETY: the view pointer comes from the window winit owns and that we
        // just looked up, so it is live; `retain` takes our own reference to it,
        // which is what makes holding it past this call sound.
        let parent = unsafe { Retained::retain(handle.ns_view.as_ptr().cast::<NSView>()) }
            .ok_or(NativeViewUnavailable::NoNativeHandle)?;
        Ok(NativeParent::from_retained(parent))
    }

    /// Whether a native-view provider has been built for this session.
    pub fn has_native_views(&self) -> bool {
        self.native_caps.is_some()
    }

    pub fn ime_is_enabled(&self, window: WindowId) -> bool {
        self.ime_enabled
            .get(&window.index())
            .copied()
            .unwrap_or(false)
    }

    /// Number of window requests that have not been realised yet.
    pub fn pending_windows(&self) -> usize {
        self.pending.len()
    }

    /// The surface configuration the backend created for `window`.
    ///
    /// A renderer backend uses this to pick an alpha mode before it builds its
    /// swap-chain, instead of re-deriving it from window attributes.
    pub fn surface_config(&self, window: WindowId) -> Option<SurfaceConfig> {
        let w = self
            .windows
            .get(&window.index())
            .filter(|w| w.id == window)?;
        let surface = w.surface.as_ref()?;
        Some(SurfaceConfig {
            size: surface.size,
            transparent: surface.transparent,
        })
    }

    /// Create a window right now, from inside an event-loop callback.
    ///
    /// Returns `None` when the OS refuses to create the window (which winit
    /// reports rather than panicking); the window id is still consumed.
    pub fn create_window_now(
        &mut self,
        active: &dyn ActiveEventLoop,
        id: WindowId,
        config: WindowConfig,
    ) -> Option<WindowId> {
        let logical = winit::dpi::LogicalSize::new(config.size.width, config.size.height);
        let attributes = WindowAttributes::default()
            .with_title(config.title.clone())
            .with_surface_size(logical)
            .with_resizable(config.resizable);
        let native = Arc::<dyn WinitWindow>::from(active.create_window(attributes).ok()?);

        let scale = native.scale_factor();
        let size = logical_size(native.surface_size(), scale);
        let mut window = WinitWindowAdapter {
            id,
            native: native.clone(),
            title: config.title,
            size,
            scale,
            state: WindowState::Normal,
            surface: None,
            next_surface: 0,
            last_pointer: None,
        };
        let surface = window.create_surface(SurfaceConfig {
            size,
            transparent: false,
        });

        self.native_ids.insert(native.id(), id);
        self.windows.insert(id.index(), window);
        self.events.push(PlatformEvent::WindowCreated {
            window: id,
            surface,
            size,
        });
        Some(id)
    }

    /// Realise every queued window. Called from `can_create_surfaces`/`resumed`.
    fn materialise_pending(&mut self, active: &dyn ActiveEventLoop) -> Vec<WindowId> {
        let queued: Vec<_> = self.pending.drain(..).collect();
        let mut created = Vec::new();
        for (id, config) in queued {
            if self.create_window_now(active, id, config).is_some() {
                created.push(id);
            } else {
                // The OS refused; surface the failure instead of dropping it.
                self.events.push(PlatformEvent::WindowClosed { window: id });
            }
        }
        created
    }

    fn window_index(&self, native: WinitWindowId) -> Option<WindowId> {
        self.native_ids.get(&native).copied()
    }

    fn set_size(&mut self, window: WindowId, physical: winit::dpi::PhysicalSize<u32>, scale: f64) {
        if let Some(w) = self.windows.get_mut(&window.index()) {
            if w.id != window {
                return;
            }
            w.scale = scale;
            w.size = logical_size(physical, scale);
            if let Some(surface) = w.surface.as_mut() {
                surface.size = w.size;
            }
        }
    }

    /// Translate one winit window event into ABI events and update local state.
    ///
    /// `modifiers` is threaded through so an input event carries the chord that
    /// was active *at that event*, rather than whatever the most recent
    /// `ModifiersChanged` happened to leave behind.
    fn translate(
        &mut self,
        active: &dyn ActiveEventLoop,
        native: WinitWindowId,
        event: WindowEvent,
        modifiers: &mut Modifiers,
    ) {
        let Some(window) = self.window_index(native) else {
            return;
        };
        let scale = self
            .windows
            .get(&window.index())
            .map(|w| w.scale)
            .unwrap_or(1.0);

        match event {
            WindowEvent::CloseRequested => {
                // winit does not close the window automatically — we must tell
                // it to. Without this the native title-bar buttons fire the
                // request, the event is queued, and the window stays open.
                active.exit();
                self.events.push(PlatformEvent::WindowClosed { window });
            }
            WindowEvent::Destroyed => {
                self.native_ids.remove(&native);
                self.windows.remove(&window.index());
                self.events.push(PlatformEvent::WindowClosed { window });
            }
            WindowEvent::SurfaceResized(physical) => {
                self.set_size(window, physical, scale);
                let size = self
                    .windows
                    .get(&window.index())
                    .map(|w| w.size)
                    .unwrap_or_default();
                self.events
                    .push(PlatformEvent::WindowResized { window, size });
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                let physical = self
                    .windows
                    .get(&window.index())
                    .map(|w| w.native.surface_size())
                    .unwrap_or(winit::dpi::PhysicalSize::new(0, 0));
                self.set_size(window, physical, scale_factor);
                let size = self
                    .windows
                    .get(&window.index())
                    .map(|w| w.size)
                    .unwrap_or_default();
                self.events
                    .push(PlatformEvent::WindowResized { window, size });
            }
            WindowEvent::Focused(focused) => {
                self.events
                    .push(PlatformEvent::WindowFocused { window, focused });
            }
            WindowEvent::RedrawRequested => {
                self.events.push(PlatformEvent::RedrawRequested { window });
            }
            WindowEvent::ModifiersChanged(state) => {
                *modifiers = winit_input::map_modifiers(state.state());
                self.events.push(PlatformEvent::ModifiersChanged {
                    window,
                    modifiers: *modifiers,
                });
            }
            WindowEvent::PointerMoved { position, .. } => {
                let position = logical_point(position, scale);
                if let Some(w) = self.windows.get_mut(&window.index()) {
                    w.last_pointer = Some(position);
                }
                self.events.push(PlatformEvent::PointerMoved {
                    window,
                    position,
                    modifiers: *modifiers,
                });
            }
            WindowEvent::PointerLeft {
                position: Some(position),
                ..
            } => {
                let position = logical_point(position, scale);
                if let Some(w) = self.windows.get_mut(&window.index()) {
                    w.last_pointer = Some(position);
                }
                self.events.push(PlatformEvent::PointerMoved {
                    window,
                    position,
                    modifiers: *modifiers,
                });
            }
            WindowEvent::PointerButton {
                state,
                button,
                position,
                ..
            } => {
                let position = logical_point(position, scale);
                if let Some(w) = self.windows.get_mut(&window.index()) {
                    w.last_pointer = Some(position);
                }
                self.events.push(PlatformEvent::PointerButton {
                    window,
                    position,
                    button: winit_input::map_button_source(button),
                    pressed: winit_input::is_pressed(state),
                    modifiers: *modifiers,
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = winit_input::map_scroll_delta(delta, scale);
                if delta_x != 0.0 || delta_y != 0.0 {
                    let position = self
                        .windows
                        .get(&window.index())
                        .and_then(|w| w.last_pointer)
                        .unwrap_or_default();
                    self.events.push(PlatformEvent::Scroll {
                        window,
                        position,
                        delta_x,
                        delta_y,
                        modifiers: *modifiers,
                    });
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let code = winit_input::map_key(&event.logical_key, event.physical_key);
                if code != KeyCode::NONE {
                    self.events.push(PlatformEvent::Key {
                        window,
                        code,
                        pressed: winit_input::is_pressed(event.state),
                        repeat: event.repeat,
                        modifiers: *modifiers,
                    });
                }
            }
            WindowEvent::Ime(WinitIme::Preedit(text, _)) => {
                self.events.push(PlatformEvent::ImePreedit { window, text });
            }
            WindowEvent::Ime(WinitIme::Commit(text)) => {
                self.events.push(PlatformEvent::ImeCommit { window, text });
            }
            WindowEvent::Ime(WinitIme::Enabled) => {
                self.ime_enabled.insert(window.index(), true);
            }
            WindowEvent::Ime(WinitIme::Disabled) => {
                self.ime_enabled.insert(window.index(), false);
            }
            // Everything else (theme, occlusion, gestures, drag-and-drop) is
            // either already covered by another event or not yet modelled by
            // the ABI. Dropping it here keeps the ABI honest: a variant exists
            // because someone consumes it, not because winit has one.
            _ => {}
        }
        let _ = active;
    }
}

fn logical_size(physical: winit::dpi::PhysicalSize<u32>, scale: f64) -> Size {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Size::new(
        f64::from(physical.width) / scale,
        f64::from(physical.height) / scale,
    )
}

fn logical_point(position: winit::dpi::PhysicalPosition<f64>, scale: f64) -> Point {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Point::new(position.x / scale, position.y / scale)
}

/// The [`floom_types::PointerEvent`] a queued platform event stands for, if it
/// is one.
///
/// The same mapping `floom-app`'s runner does, because the app-side hook has to
/// answer about the event the tree would receive — not about the winit event,
/// which carries a physical position and a backend button id.
/// `crates/app/floom-app/src/runner.rs::map_button` is the other half and must keep
/// the same folding of `Auxiliary`/`Other` onto `Middle`; a hook that saw
/// `Other(7)` would answer about a button the tree never sees.
fn pointer_event(event: &PlatformEvent) -> Option<floom_types::PointerEvent> {
    use floom_types::{PointerEvent, PointerKind, PointerPhase};
    match event {
        PlatformEvent::PointerMoved {
            position,
            modifiers,
            ..
        } => Some(PointerEvent::hover(*position).with_modifiers(*modifiers)),
        PlatformEvent::PointerButton {
            position,
            button,
            pressed,
            modifiers,
            ..
        } => {
            let button = match button {
                floom_platform::PointerButton::Primary => floom_types::PointerButton::Primary,
                floom_platform::PointerButton::Secondary => floom_types::PointerButton::Secondary,
                floom_platform::PointerButton::Auxiliary
                | floom_platform::PointerButton::Other(_) => floom_types::PointerButton::Middle,
            };
            Some(PointerEvent {
                phase: if *pressed {
                    PointerPhase::Down
                } else {
                    PointerPhase::Up
                },
                kind: PointerKind::Mouse,
                button: Some(button),
                position: *position,
                modifiers: *modifiers,
                click_count: 1,
            })
        }
        _ => None,
    }
}

impl Platform for WinitPlatform {
    fn id(&self) -> PlatformId {
        self.id
    }

    fn capabilities(&self) -> PlatformCapabilities {
        // The bridge's own answer once one exists, and `DesktopPlatform`'s
        // no-bridge answer until then. Both come from `desktop_capabilities`, so
        // the flag is derived in exactly one place — this is the call site that
        // can know whether a bridge was built.
        self.native_caps
            .unwrap_or_else(|| crate::native::desktop_capabilities(false))
    }

    fn ime(&mut self, _window: WindowId) -> Option<&mut dyn Ime> {
        // The OS input method is offered as a switch only while the focused
        // view is IME-enabled; `WinitPlatform` is the IME bridge, so this is the
        // object that turns the enable into a winit `set_ime_allowed`. Without
        // it, a focused text field can never take a Chinese (or any) input
        // source — the OS simply will not show one.
        Some(self)
    }

    /// Queue a window request; it is realised on the next event-loop callback.
    ///
    /// See the module docs for why this cannot create the window eagerly.
    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        let index = self.next_window;
        self.next_window += 1;
        let id = WindowId::new(index, 1);
        self.pending.push_back((id, config));
        id
    }

    fn window(&self, id: WindowId) -> Option<&dyn Window> {
        self.windows
            .get(&id.index())
            .filter(|w| w.id == id)
            .map(|w| w as &dyn Window)
    }

    fn window_mut(&mut self, id: WindowId) -> Option<&mut dyn Window> {
        let w = self.windows.get_mut(&id.index())?;
        if w.id != id {
            return None;
        }
        Some(w as &mut dyn Window)
    }

    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        out.append(&mut self.events);
    }

    /// Ask winit for a redraw. The resulting `RedrawRequested` arrives
    /// asynchronously through the normal event path, so nothing is queued here
    /// — the redraw callback stays the single source of "a frame is due".
    fn request_frame(&mut self, window: WindowId) {
        if let Some(w) = self.windows.get_mut(&window.index())
            && w.id == window
        {
            w.native.request_redraw();
        }
    }

    fn now_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

impl Clipboard for WinitPlatform {
    /// macOS reads the **system** pasteboard (`NSPasteboard`), so text copied
    /// here is pasteable in any other app and vice versa. See
    /// [`crate::clipboard`] for why the difference is not cosmetic: a
    /// process-local `String` passes every test a clipboard can be asked to
    /// pass, while being unusable.
    ///
    /// Every other desktop target keeps the in-process placeholder, which is
    /// honest about its own reach (`None` until something is written here, and
    /// invisible outside this process) rather than claiming to be the OS's.
    #[cfg(target_os = "macos")]
    fn get_text(&mut self) -> Option<String> {
        crate::clipboard::system_read_text()
    }

    #[cfg(target_os = "macos")]
    fn set_text(&mut self, text: &str) -> bool {
        crate::clipboard::system_write_text(text)
    }

    #[cfg(not(target_os = "macos"))]
    fn get_text(&mut self) -> Option<String> {
        Some(self.clipboard.clone())
    }

    #[cfg(not(target_os = "macos"))]
    fn set_text(&mut self, text: &str) -> bool {
        self.clipboard = text.to_string();
        true
    }
}

impl Ime for WinitPlatform {
    fn set_enabled(&mut self, window: WindowId, enabled: bool) {
        self.ime_enabled.insert(window.index(), enabled);
        let Some(w) = self.windows.get(&window.index()) else {
            return;
        };
        let request = if enabled {
            // Empty capabilities + default data is the "just turn it on"
            // request; `ImeEnableRequest::new` rejects a capability whose
            // initial value was not supplied, so the two must match.
            match ImeEnableRequest::new(ImeCapabilities::new(), ImeRequestData::default()) {
                Some(request) => ImeRequest::Enable(request),
                None => return,
            }
        } else {
            ImeRequest::Disable
        };
        // A backend may legitimately not support IME; that is a capability
        // question, not an error for the caller.
        let _ = w.native.request_ime_update(request);
    }

    fn set_cursor_area(&mut self, window: WindowId, origin: Point, size: Size) {
        let Some(w) = self.windows.get(&window.index()) else {
            return;
        };
        let data = ImeRequestData::default().with_cursor_area(
            winit::dpi::Position::Logical(winit::dpi::LogicalPosition::new(origin.x, origin.y)),
            winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(size.width, size.height)),
        );
        // Fails with `NotEnabled` when no field has focus yet — expected.
        let _ = w.native.request_ime_update(ImeRequest::Update(data));
    }
}

/// The application side of a winit-hosted Floom app.
///
/// Implementors get called from inside winit callbacks, with the backend handy,
/// and are expected to drain [`Platform::poll_events`] in [`events`](Self::events).
pub trait WinitApp {
    /// A queued window has just been realised by the OS.
    fn window_created(&mut self, platform: &mut WinitPlatform, window: WindowId) {
        let _ = (platform, window);
    }

    /// One batch of platform events is ready. This is the frame hook.
    fn events(&mut self, platform: &mut WinitPlatform) {
        let _ = platform;
    }

    /// A key event, before the tree sees it.
    ///
    /// The counterpart of [`Self::events`] for one key: the platform has already
    /// mapped the OS key into the canonical [`KeyCode`], and this is the last
    /// moment the app can consume it without the tree also receiving it.
    ///
    /// Returns whether the event was consumed. A consumed event is *not* pushed
    /// as a [`PlatformEvent::Key`], so a global shortcut — a devtools panel, a
    /// command palette — can answer a keystroke without the focused control
    /// also typing it. Returning `false` (the default) forwards the key, which
    /// is what every app that has no global shortcut wants.
    fn key(&mut self, _event: &floom_types::KeyEvent) -> bool {
        false
    }

    /// A pointer event, before the tree sees it.
    ///
    /// The counterpart of [`Self::key`] for the pointer: the event has already
    /// been mapped into viewport coordinates, and this is the last moment the
    /// app can consume it without the tree also receiving it.
    ///
    /// Returns whether the event was consumed. A consumed event is *not* pushed
    /// as a [`PlatformEvent::PointerMoved`] / [`PlatformEvent::PointerButton`],
    /// so an inspector's pick can answer a click without the control underneath
    /// also being pressed. Returning `false` (the default) forwards it.
    fn pointer(&mut self, _event: &floom_types::PointerEvent) -> bool {
        false
    }

    /// The event loop is about to block.
    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        let _ = platform;
    }

    /// The loop has decided to exit, called exactly once while the platform is
    /// still alive — the place to drop GPU surfaces and other OS resources.
    /// winit has no dedicated callback for this in this version; the loop
    /// reports it through `ActiveEventLoop::exiting()`, so the driver invokes
    /// this from `about_to_wait` the first time that flag is set.
    fn exiting(&mut self, platform: &mut WinitPlatform) {
        let _ = platform;
    }
}

struct WinitDriver<A> {
    platform: WinitPlatform,
    app: A,
    modifiers: Modifiers,
    /// Guards [`WinitApp::exiting`] so it fires once even though it is driven
    /// off `about_to_wait`, which runs repeatedly.
    exit_notified: bool,
}

impl<A: WinitApp> ApplicationHandler for WinitDriver<A> {
    fn new_events(&mut self, _event_loop: &dyn ActiveEventLoop, _cause: StartCause) {}

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.realise_windows(event_loop);
    }

    fn resumed(&mut self, event_loop: &dyn ActiveEventLoop) {
        // winit calls `resumed` before `can_create_surfaces` on some backends;
        // `materialise_pending` drains the queue, so doing it in both places is
        // idempotent.
        self.realise_windows(event_loop);
        self.platform.events.push(PlatformEvent::Resumed);
        self.dispatch();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WinitWindowId,
        event: WindowEvent,
    ) {
        // A key event is offered to the app *before* it becomes a platform
        // event, so a global shortcut can consume it. `translate` pushes at most
        // one `Key` per call, and it pushes it last, so the hook runs first and
        // a consumed event is popped back off the tail.
        let mut consumed = false;
        if let WindowEvent::KeyboardInput { event, .. } = &event {
            let code = winit_input::map_key(&event.logical_key, event.physical_key);
            if code != KeyCode::NONE
                && let Some(key) = code.to_key()
            {
                let phase = match (winit_input::is_pressed(event.state), event.repeat) {
                    (true, true) => KeyPhase::Repeat,
                    (true, false) => KeyPhase::Down,
                    (false, _) => KeyPhase::Up,
                };
                let Self {
                    platform: _,
                    app,
                    modifiers,
                    ..
                } = self;
                consumed = app.key(&KeyEvent {
                    phase,
                    key,
                    modifiers: *modifiers,
                });
            }
        }

        let Self {
            platform,
            app,
            modifiers,
            ..
        } = self;
        platform.translate(event_loop, window_id, event, modifiers);
        if consumed && let Some(PlatformEvent::Key { .. }) = platform.events.last() {
            platform.events.pop();
        }
        // Same shape for the pointer: `translate` pushes at most one pointer
        // event per call and pushes it last, so the hook sees exactly the event
        // the tree would have. A consumed one is popped back off the tail, which
        // is what lets a pick select an element without also pressing it.
        offer_pointer(app, &mut platform.events);
        app.events(platform);
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        // winit exposes "the loop is shutting down" as a query, not a callback.
        // Check it here so the app gets one last chance to release OS resources
        // (GPU surfaces above all) while its own state is still reachable.
        if event_loop.exiting() && !self.exit_notified {
            self.exit_notified = true;
            let Self { platform, app, .. } = self;
            app.exiting(platform);
        }
        let Self { platform, app, .. } = self;
        app.about_to_wait(platform);
    }

    fn suspended(&mut self, _event_loop: &dyn ActiveEventLoop) {
        self.platform.events.push(PlatformEvent::Suspended);
        self.dispatch();
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        // The OS is tearing the surfaces down (Android pause, display removal).
        // The next `can_create_surfaces` rebuilds them; the render layer must
        // drop its swap-chain in the meantime, which is what `Suspended` means.
        self.platform.events.push(PlatformEvent::Suspended);
        self.dispatch();
    }
}

/// Offers the pointer event at the tail of `events` to the app, popping it if
/// the app consumed it.
///
/// Extracted from `window_event` so the consume-or-forward decision is testable
/// without a window: it is the whole contract of [`WinitApp::pointer`], and the
/// bug it prevents — a pick that selects *and* presses — is invisible in a
/// headless test of the picker alone.
///
/// `translate` pushes at most one pointer event per call and pushes it last, so
/// the hook sees exactly the event the tree would have. A non-pointer tail (a
/// resize, a focus change) is left alone: the hook is about the pointer, and
/// offering it something else would let an app consume an unrelated event.
fn offer_pointer<A: WinitApp + ?Sized>(app: &mut A, events: &mut Vec<PlatformEvent>) {
    let Some(pointer) = events.last().and_then(pointer_event) else {
        return;
    };
    if app.pointer(&pointer)
        && let Some(PlatformEvent::PointerMoved { .. } | PlatformEvent::PointerButton { .. }) =
            events.last()
    {
        events.pop();
    }
}

impl<A: WinitApp> WinitDriver<A> {
    fn realise_windows(&mut self, event_loop: &dyn ActiveEventLoop) {
        let Self { platform, app, .. } = self;
        let created = platform.materialise_pending(event_loop);
        for window in created {
            app.window_created(platform, window);
        }
        app.events(platform);
    }

    fn dispatch(&mut self) {
        let Self { platform, app, .. } = self;
        app.events(platform);
    }
}

/// Run a Floom app on the real winit event loop. Blocks until the loop exits.
///
/// `windows` are queued before the loop starts and realised as soon as the OS
/// allows window creation. Must be called from the main thread on macOS/iOS.
pub fn run<A: WinitApp + 'static>(app: A, windows: Vec<WindowConfig>) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut builder = winit::event_loop::EventLoop::builder();
    #[cfg(target_os = "macos")]
    {
        // Bring the app to the foreground when launched from a terminal or
        // another app, so its title-bar buttons are not dimmed. Otherwise a
        // window opened by `floom run` can sit behind the terminal with no
        // responsive close/minimise/maximise controls.
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        builder.with_activate_ignoring_other_apps(true);
    }
    #[cfg(not(target_os = "macos"))]
    let builder = winit::event_loop::EventLoop::builder();
    let event_loop = builder
        .build()
        .map_err(|err| format!("event loop: {err}"))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut platform = WinitPlatform::new();
    for config in windows {
        platform.create_window(config);
    }

    let driver = WinitDriver {
        platform,
        app,
        modifiers: Modifiers::NONE,
        exit_notified: false,
    };
    event_loop
        .run_app(driver)
        .map_err(|err| format!("event loop: {err}"))
}

#[cfg(all(test, feature = "winit"))]
mod pointer_hook {
    //! The consume-or-forward contract of [`WinitApp::pointer`], without a window.
    //!
    //! `offer_pointer` is what `window_event` runs between `translate` and
    //! `app.events`, so these tests drive the real function with a real event
    //! queue. What they cannot cover is the winit → `PlatformEvent` half; that
    //! needs an OS window and is the device case.

    use super::*;
    use floom_platform::{PlatformEvent, PointerButton as PlatformPointerButton};
    use floom_types::{Modifiers, Point, WindowId};

    fn window() -> WindowId {
        WindowId::new(0, 1)
    }

    fn moved() -> PlatformEvent {
        PlatformEvent::PointerMoved {
            window: window(),
            position: Point::new(12.0, 34.0),
            modifiers: Modifiers::NONE,
        }
    }

    fn pressed() -> PlatformEvent {
        PlatformEvent::PointerButton {
            window: window(),
            position: Point::new(12.0, 34.0),
            button: PlatformPointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }
    }

    /// An app whose pointer hook answers whatever it was told to.
    struct Stub {
        answer: bool,
        seen: Vec<floom_types::PointerEvent>,
    }

    impl WinitApp for Stub {
        fn pointer(&mut self, event: &floom_types::PointerEvent) -> bool {
            self.seen.push(*event);
            self.answer
        }
    }

    #[test]
    fn a_consumed_pointer_event_never_reaches_the_tree() {
        let mut app = Stub {
            answer: true,
            seen: Vec::new(),
        };
        let mut events = vec![moved()];
        offer_pointer(&mut app, &mut events);
        assert_eq!(app.seen.len(), 1, "the hook was asked");
        assert!(
            events.is_empty(),
            "a consumed move is dropped, so a pick's hover is not also a hover"
        );
    }

    #[test]
    fn a_forwarded_pointer_event_stays_in_the_queue() {
        let mut app = Stub {
            answer: false,
            seen: Vec::new(),
        };
        let mut events = vec![pressed()];
        offer_pointer(&mut app, &mut events);
        assert_eq!(app.seen.len(), 1);
        assert_eq!(events.len(), 1, "the tree still receives the click");
    }

    /// The hook is handed the event the *tree* would receive, in the tree's own
    /// vocabulary — not the platform's. The button folding is the part that can
    /// silently differ: `Auxiliary` and the exotic buttons are `Middle` on the
    /// widget side, and a hook that saw `Other(7)` would answer about a button
    /// the tree never sees.
    #[test]
    fn the_hook_sees_the_tree_vocabulary_not_the_platforms() {
        let mut app = Stub {
            answer: false,
            seen: Vec::new(),
        };
        let mut events = vec![
            PlatformEvent::PointerMoved {
                window: window(),
                position: Point::new(1.0, 2.0),
                modifiers: Modifiers::SHIFT,
            },
            PlatformEvent::PointerButton {
                window: window(),
                position: Point::new(3.0, 4.0),
                button: PlatformPointerButton::Auxiliary,
                pressed: false,
                modifiers: Modifiers::NONE,
            },
        ];
        // Only the tail is offered: `translate` pushes at most one pointer event
        // per call, so an older queued one is not this call's business.
        offer_pointer(&mut app, &mut events);
        let seen = &app.seen[0];
        assert_eq!(seen.phase, floom_types::PointerPhase::Up);
        assert_eq!(seen.button, Some(floom_types::PointerButton::Middle));
        assert_eq!(seen.position, Point::new(3.0, 4.0));
        assert_eq!(events.len(), 2, "not consumed, both stay");
    }

    #[test]
    fn a_non_pointer_event_is_never_offered_to_the_hook() {
        let mut app = Stub {
            answer: true,
            seen: Vec::new(),
        };
        let mut events = vec![PlatformEvent::ModifiersChanged {
            window: window(),
            modifiers: Modifiers::META,
        }];
        offer_pointer(&mut app, &mut events);
        assert!(
            app.seen.is_empty(),
            "the pointer hook is about the pointer; an app must not be able to \
             consume an unrelated event through it"
        );
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn an_empty_queue_is_harmless() {
        let mut app = Stub {
            answer: true,
            seen: Vec::new(),
        };
        let mut events = Vec::new();
        offer_pointer(&mut app, &mut events);
        assert!(app.seen.is_empty());
        assert!(events.is_empty());
    }
}
