// SPDX-License-Identifier: MIT
//! Android platform adapter — **real winit backend** (Phase 17 / 19.4).
//!
//! This is the first cut of the OS-surface bridge for Android. It reuses the
//! lapce/winit fork's `floem-winit-android` backend (the `winit` dependency with
//! the `android-native-activity` feature) exactly the way the desktop adapter
//! reuses `floem-winit` on macOS — so the heavy lifting of the Android lifecycle,
//! `ANativeWindow` and input is winit's, not ours.
//!
//! Architecture mirrors `floom-platform-desktop/src/winit_backend.rs`: winit's
//! *push* event loop is bridged to Floom's *pull* [`Platform::poll_events`] by an
//! in-memory event queue. The [`AndroidDriver`] `ApplicationHandler` translates
//! winit events into [`PlatformEvent`]s and queues them; [`AndroidPlatform`]
//! drains them on `poll_events`.
//!
//! **Layer hygiene**: this crate is L4 and must NOT depend on L5 (`floom-gesture`).
//! So pointer input is emitted as *raw* pointer / scroll events here; tap-vs-scroll
//! fusion (the `TouchRecognizer` we already ported) lives above the backend, in
//! L5, exactly as on desktop. Key/modifier mapping reuses the same logic as the
//! desktop backend — both target this same winit fork.
//!
//! **Documented seams (not yet wired)**: GPU surface creation from the
//! `ANativeWindow` (vello/wgpu device — Step 3), the JNI clipboard bridge
//! (Step 4), and the JNI IME / safe-area bridge. Those are called out inline so
//! the next pass has a home.

#![cfg(target_os = "android")]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{
    ButtonSource, ElementState, Ime as WinitIme, MouseButton, MouseScrollDelta, StartCause,
    WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key as WinitKey, KeyCode as WinitKeyCode, ModifiersState, PhysicalKey};
use winit::platform::android::EventLoopBuilderExtAndroid;
use winit::platform::android::activity::AndroidApp;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{
    ImeRequest, ImeRequestData, Window as WinitWindow, WindowAttributes, WindowId as WinitWindowId,
};

/// What the IME produced, as read back from the anchor view.
use crate::packages::keyboard::soft_input::ImeInput as SoftInput;
use floom_platform::{
    Capability, Clipboard, Ime, KeyCode, Platform, PlatformCapabilities, PlatformEvent,
    PointerButton, SurfaceConfig, SurfaceTarget, Window, WindowConfig, WindowState,
};
use floom_types::{
    DisplayInfo, Modifiers, PlatformId, Point, ScaleFactor, Size, SurfaceId, WindowId,
};

/// Floom side of a winit-hosted Android app. Called from inside winit callbacks.
///
/// This is the mirror of `floom-platform-desktop`'s `WinitApp`: the runner drains
/// [`Platform::poll_events`] here (the frame hook), learns when a window was
/// realised, and gets one last chance to release OS resources on exit.
pub trait AndroidRunner {
    fn window_created(&mut self, platform: &mut AndroidPlatform, window: WindowId) {
        let _ = (platform, window);
    }
    fn events(&mut self, platform: &mut AndroidPlatform) {
        let _ = platform;
    }
    fn about_to_wait(&mut self, platform: &mut AndroidPlatform) {
        let _ = platform;
    }
    fn exiting(&mut self, platform: &mut AndroidPlatform) {
        let _ = platform;
    }
}

/// Bookkeeping for one realised surface (logical size + identity).
#[derive(Debug, Clone, Copy)]
struct AndroidSurface {
    id: SurfaceId,
    size: Size,
}

/// One realised Android window: the winit window plus the Floom surface bookkeeping.
pub struct AndroidWindow {
    id: WindowId,
    native: Arc<dyn WinitWindow>,
    title: String,
    /// Logical size; derived from the native physical size and scale factor.
    size: Size,
    scale: f64,
    state: WindowState,
    surface: Option<AndroidSurface>,
    next_surface: u32,
}

impl AndroidWindow {
    fn new(id: WindowId, native: Arc<dyn WinitWindow>) -> Self {
        let scale = native.scale_factor();
        let size = logical_size(native.surface_size(), scale);
        Self {
            id,
            native,
            title: String::new(),
            size,
            scale,
            state: WindowState::Normal,
            surface: None,
            next_surface: 0,
        }
    }

    /// Native handles for the current surface, if the OS window can supply them.
    /// Rebuilt on demand rather than cached: the handles belong to the window and
    /// change if the window is recreated (Android resume), so a cached copy would
    /// be a stale-pointer bug waiting to happen.
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

impl Window for AndroidWindow {
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

    fn surface_id(&self) -> Option<SurfaceId> {
        self.surface.as_ref().map(|s| s.id)
    }

    fn create_surface(&mut self, config: SurfaceConfig) -> SurfaceId {
        let id = SurfaceId::new(self.next_surface, 1);
        self.next_surface += 1;
        self.surface = Some(AndroidSurface {
            id,
            size: config.size,
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
}

/// Android platform backend. L4; pull-model `Platform` + (via [`AndroidDriver`])
/// the winit push-model `ApplicationHandler` bridged through an event queue.
pub struct AndroidPlatform {
    id: PlatformId,
    windows: HashMap<u32, AndroidWindow>,
    /// winit's opaque window id → ours, for event routing.
    native_ids: HashMap<WinitWindowId, WindowId>,
    next_window: u32,
    /// Requests made before an [`ActiveEventLoop`] was available.
    pending: Vec<(WindowId, WindowConfig)>,
    events: Vec<PlatformEvent>,
    /// Per-window IME enabled state, mirroring the desktop backend and the
    /// headless platform. Kept here rather than only in winit because
    /// `set_cursor_area` may arrive before or after the enable and must not
    /// forward an `Update` for a field that was never enabled.
    ime_enabled: HashMap<u32, bool>,
    /// The semantics tree in the shape the JVM half consumes. Retained even
    /// when no publisher is attached, so tooling and tests can read it.
    accessibility: crate::accessibility::AndroidAccessibility,
    started: Instant,
}

impl Default for AndroidPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidPlatform {
    pub fn new() -> Self {
        Self {
            id: PlatformId::new(10, 1),
            windows: HashMap::new(),
            native_ids: HashMap::new(),
            next_window: 1,
            pending: Vec::new(),
            events: Vec::new(),
            ime_enabled: HashMap::new(),
            accessibility: crate::accessibility::AndroidAccessibility::new(),
            started: Instant::now(),
        }
    }

    /// Queue a window; realised on the next `can_create_surfaces`/`resumed`.
    fn enqueue_window(&mut self, config: WindowConfig) -> WindowId {
        let index = self.next_window;
        self.next_window += 1;
        let id = WindowId::new(index, 1);
        self.pending.push((id, config));
        id
    }

    /// Create a window right now, from inside an event-loop callback.
    ///
    /// Returns `None` when the OS refuses to create the window (which winit
    /// reports rather than panicking); the window id is still consumed.
    fn create_window_now(
        &mut self,
        active: &dyn ActiveEventLoop,
        id: WindowId,
        config: WindowConfig,
    ) -> Option<WindowId> {
        let logical =
            winit::dpi::LogicalSize::new(config.size.width.max(1.0), config.size.height.max(1.0));
        let attributes = WindowAttributes::default()
            .with_title(config.title.clone())
            .with_surface_size(logical)
            .with_resizable(config.resizable);
        let native = Arc::<dyn WinitWindow>::from(active.create_window(attributes).ok()?);

        let mut window = AndroidWindow::new(id, native.clone());
        window.title = config.title;
        let surface = window.create_surface(SurfaceConfig {
            size: window.size,
            transparent: false,
        });

        self.native_ids.insert(native.id(), id);
        self.windows.insert(id.index(), window);
        self.events.push(PlatformEvent::WindowCreated {
            window: id,
            surface,
            size: Size::new(logical.width, logical.height),
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

    /// Move text the IME produced into the event queue.
    ///
    /// The anchor view belongs to the platform layer and the app never sees it,
    /// so the mapping from "what the IME wrote into the anchor" to "an event
    /// the app can act on" has to happen here.
    ///
    /// Only windows that asked for the IME are read: the anchor is one per
    /// process, so with no field focused there is nothing to route text to, and
    /// with several the first enabled one owns it — the same window the
    /// keyboard was shown *for*, which is what a single anchor can honour.
    fn pump_ime_text(&mut self) {
        let Some(window) = self
            .ime_enabled
            .iter()
            .find(|(_, enabled)| **enabled)
            .map(|(index, _)| WindowId::new(*index, 1))
        else {
            return;
        };

        for input in crate::packages::keyboard::soft_input::take_ime_input() {
            match input {
                SoftInput::Commit(text) => {
                    self.events.push(PlatformEvent::ImeCommit { window, text })
                }
                SoftInput::Preedit(text) => {
                    self.events.push(PlatformEvent::ImePreedit { window, text })
                }
            }
        }
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
    fn translate(&mut self, native: WinitWindowId, event: WindowEvent, modifiers: &mut Modifiers) {
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
                *modifiers = map_modifiers(state.state());
                self.events.push(PlatformEvent::ModifiersChanged {
                    window,
                    modifiers: *modifiers,
                });
            }
            WindowEvent::PointerMoved { position, .. } => {
                let position = logical_point(position, scale);
                self.events.push(PlatformEvent::PointerMoved {
                    window,
                    position,
                    modifiers: *modifiers,
                });
            }
            WindowEvent::PointerLeft { position, .. } => {
                if let Some(position) = position {
                    let position = logical_point(position, scale);
                    self.events.push(PlatformEvent::PointerMoved {
                        window,
                        position,
                        modifiers: *modifiers,
                    });
                }
            }
            WindowEvent::PointerButton {
                state,
                button,
                position,
                ..
            } => {
                let position = logical_point(position, scale);
                self.events.push(PlatformEvent::PointerButton {
                    window,
                    position,
                    button: map_button_source(button),
                    pressed: is_pressed(state),
                    modifiers: *modifiers,
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (delta_x, delta_y) = map_scroll_delta(delta, scale);
                if delta_x != 0.0 || delta_y != 0.0 {
                    self.events.push(PlatformEvent::Scroll {
                        window,
                        position: Point::new(0.0, 0.0),
                        delta_x,
                        delta_y,
                        modifiers: *modifiers,
                    });
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let code = map_key(&event.logical_key, event.physical_key);
                if code != KeyCode::NONE {
                    log::debug!("key {:?} pressed={}", code, is_pressed(event.state));
                    self.events.push(PlatformEvent::Key {
                        window,
                        code,
                        pressed: is_pressed(event.state),
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
            _ => {}
        }
    }
}

impl Platform for AndroidPlatform {
    fn id(&self) -> PlatformId {
        self.id
    }
    fn capabilities(&self) -> PlatformCapabilities {
        // Windowing and touch are real, and so are the soft keyboard and the
        // clipboard: the platform polls the anchor view's `Editable` for
        // typed text and reaches `ClipboardManager` over JNI. a11y stays the
        // stub bit set: the adapter exists and receives the tree, but no
        // `AccessibilityNodeProvider` is wired, so nothing reaches TalkBack.
        let mut caps = PlatformCapabilities::unsupported_a11y_stub();
        caps.caps |= Capability::IME | Capability::CLIPBOARD;
        caps
    }
    fn create_window(&mut self, config: WindowConfig) -> WindowId {
        self.enqueue_window(config)
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
    fn accessibility(&mut self) -> Option<&mut dyn floom_accessibility::AccessibilityAdapter> {
        // The adapter is wired; the JVM half that would consume it is the piece
        // still missing, so it publishes to nobody and claims no bridge.
        Some(&mut self.accessibility)
    }
    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>) {
        self.pump_ime_text();
        out.append(&mut self.events);
    }
    fn request_frame(&mut self, window: WindowId) {
        if let Some(w) = self.windows.get(&window.index()) {
            w.native.request_redraw();
        }
    }
    fn now_secs(&self) -> f64 {
        self.started.elapsed().as_secs_f64()
    }
}

/// Clipboard on Android goes through the system `ClipboardManager`. There is no
/// NDK clipboard API, so the call itself is JNI — it lives in
/// [`crate::packages::clipboard`], next to the `jni` crate and
/// the `JavaVM`/activity handover it needs (see [`hand_over_jni_bridge`]).
///
/// The trait's shape (`Option<String>` / `bool`) predates the richer
/// `Result<_, String>` the package uses, so failures are folded here: "empty
/// clipboard" and "bridge unavailable" both read as `None`/`false` to a caller
/// that cannot act differently on them anyway.
impl Clipboard for AndroidPlatform {
    fn get_text(&mut self) -> Option<String> {
        match crate::packages::clipboard::get_text() {
            Ok(text) => text,
            Err(err) => {
                eprintln!("floom2-android: clipboard read failed: {err}");
                None
            }
        }
    }

    fn set_text(&mut self, text: &str) -> bool {
        match crate::packages::clipboard::set_text(text) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("floom2-android: clipboard write failed: {err}");
                false
            }
        }
    }
}

/// IME on Android is the system soft keyboard, driven through the framework.
///
/// # Why this does not go through winit
///
/// The obvious route — winit's `request_ime_update`, which calls the NDK's
/// `ANativeActivity_showSoftInput` — **cannot work for a `NativeActivity`**.
/// The request travels the whole chain and is then refused by the framework:
///
/// ```text
/// ImeTracker: com.example.floom2:… onRequestShow at ORIGIN_CLIENT_SHOW_SOFT_INPUT
/// InputMethodManager: Ignoring showSoftInput() as view=…NativeContentView is not served.
/// ImeTracker: com.example.floom2:… onFailed at PHASE_CLIENT_VIEW_SERVED
/// ```
///
/// `showSoftInput` needs a view the IME can serve, and a `NativeActivity` has
/// none. So the keyboard is driven the way Java apps do it — through a hidden
/// anchor view that this crate's soft-input package owns and
/// attaches to the activity. That package is also where the live keyboard
/// height/visibility are read back from.
impl Ime for AndroidPlatform {
    fn set_enabled(&mut self, window: WindowId, enabled: bool) {
        self.ime_enabled.insert(window.index(), enabled);
        if !self.windows.contains_key(&window.index()) {
            return;
        }
        // stderr rather than the logger: the logger only reaches logcat when
        // the `debug.floom.log` switch is on, and a keyboard that will not open
        // is worth a line even on a device nobody is debugging. stderr always
        // shows, tagged `RustStdoutStderr`.
        if let Err(err) = crate::packages::keyboard::soft_input::set_soft_keyboard_visible(enabled)
        {
            let what = if enabled { "show" } else { "hide" };
            eprintln!("floom2-android: soft keyboard {what} failed: {err}");
        }
    }

    /// Records the caret rect, and forwards it to winit.
    ///
    /// Be clear about what this does here: nothing observable. The Android
    /// backend ignores the area (`set_ime_cursor_area` is a no-op), and winit
    /// only accepts an `Update` while *its* IME state is enabled — which it
    /// never is now, because `set_enabled` deliberately bypasses winit. So the
    /// forwarding below is kept for shape parity with desktop and for the day
    /// the caret anchor is wired to the anchor view; today it is inert.
    ///
    /// Wiring it properly means `InputMethodManager.updateCursorAnchorInfo`, or
    /// moving the EditText anchor to the caret — which only matters once the
    /// `InputConnection` (text delivery) exists.
    fn set_cursor_area(&mut self, window: WindowId, origin: Point, size: Size) {
        let Some(w) = self.windows.get(&window.index()) else {
            return;
        };
        if !self
            .ime_enabled
            .get(&window.index())
            .copied()
            .unwrap_or(false)
        {
            return;
        }
        let data = ImeRequestData::default().with_cursor_area(
            winit::dpi::Position::Logical(winit::dpi::LogicalPosition::new(origin.x, origin.y)),
            winit::dpi::Size::Logical(winit::dpi::LogicalSize::new(size.width, size.height)),
        );
        let _ = w.native.request_ime_update(ImeRequest::Update(data));
    }
}

/// The winit `ApplicationHandler` that drives an [`AndroidPlatform`]. Mirrors
/// `floom-platform-desktop`'s `WinitDriver`.
struct AndroidDriver<A> {
    platform: AndroidPlatform,
    app: A,
    modifiers: Modifiers,
    exit_notified: bool,
}

impl<A: AndroidRunner> AndroidDriver<A> {
    fn realise_windows(&mut self, event_loop: &dyn ActiveEventLoop) {
        let created = self.platform.materialise_pending(event_loop);
        for window in created {
            self.app.window_created(&mut self.platform, window);
        }
        self.app.events(&mut self.platform);
    }

    fn dispatch(&mut self) {
        self.app.events(&mut self.platform);
    }
}

impl<A: AndroidRunner> ApplicationHandler for AndroidDriver<A> {
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
        _event_loop: &dyn ActiveEventLoop,
        window_id: WinitWindowId,
        event: WindowEvent,
    ) {
        let Self {
            platform,
            app,
            modifiers,
            ..
        } = self;
        platform.translate(window_id, event, modifiers);
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

/// Run a Floom app on the Android winit event loop. Blocks until the loop exits.
///
/// Called from the `android_main` entry point with the `AndroidApp` winit hands
/// to us. `windows` are queued before the loop starts and realised as soon as
/// the OS allows window creation (after `resumed`).
pub fn run<A: AndroidRunner + 'static>(app: A, windows: Vec<WindowConfig>) -> Result<(), String> {
    crate::logging::install();
    log::info!("android backend starting");
    let android_app = get_android_app();
    // Hand the JVM + activity to the capability bridge before anything can use
    // it. Both setters are first-write-wins and ignore null, so a second `run`
    // (or a caller that already did this) is harmless.
    hand_over_jni_bridge(&android_app);
    let mut builder = EventLoop::builder();
    builder.with_android_app(android_app);
    let event_loop = builder
        .build()
        .map_err(|err| format!("android event loop: {err}"))?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut platform = AndroidPlatform::new();
    for config in windows {
        platform.create_window(config);
    }

    let driver = AndroidDriver {
        platform,
        app,
        modifiers: Modifiers::NONE,
        exit_notified: false,
    };
    event_loop
        .run_app(driver)
        .map_err(|err| format!("android event loop: {err}"))
}

// --- OS handle / input mapping helpers --------------------------------------

/// The `AndroidApp` is threaded through the winit entry point. `android_main`
/// stashes it here before calling `run`. THIS IS A SEAM: the entry point must
/// call [`set_android_app`] exactly once at startup.
static ANDROID_APP: std::sync::OnceLock<AndroidApp> = std::sync::OnceLock::new();

/// Called by the `android_main` glue so [`run`] can build the event loop with the
/// `AndroidApp` winit requires on Android.
pub fn set_android_app(app: AndroidApp) {
    let _ = ANDROID_APP.set(app);
}

fn get_android_app() -> AndroidApp {
    ANDROID_APP
        .get()
        .cloned()
        .expect("android_main must call set_android_app before run")
}

/// Hand the `JavaVM` and activity over to [`crate::jni`].
///
/// Android exposes the clipboard, `InputMethodManager`, vibration, location …
/// only as Java APIs, so every one of those capabilities needs a JVM. The
/// mobile capability crate owns that bridge but cannot obtain the handles
/// itself — `android-activity` is a dependency of *this* crate — so the
/// handover has to happen here, once, before any capability is used.
///
/// Getting this wrong is silent: `with_env` would keep returning its
/// "JavaVM not initialised" error and every capability would look merely
/// "unsupported" rather than misconfigured. That is why it is a single named
/// function called from [`run`] rather than an inline pair of calls a later
/// refactor could drop.
fn hand_over_jni_bridge(app: &AndroidApp) {
    crate::jni::set_java_vm(app.vm_as_ptr());
    crate::jni::set_activity(app.activity_as_ptr());
}

fn logical_size(size: winit::dpi::PhysicalSize<u32>, scale: f64) -> Size {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Size::new(size.width as f64 / scale, size.height as f64 / scale)
}

fn logical_point(pos: winit::dpi::PhysicalPosition<f64>, scale: f64) -> Point {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    Point::new(pos.x / scale, pos.y / scale)
}

fn map_modifiers(state: ModifiersState) -> Modifiers {
    let mut out = Modifiers::NONE;
    if state.shift_key() {
        out = out.union(Modifiers::SHIFT);
    }
    if state.control_key() {
        out = out.union(Modifiers::CONTROL);
    }
    if state.alt_key() {
        out = out.union(Modifiers::ALT);
    }
    if state.meta_key() {
        out = out.union(Modifiers::META);
    }
    out
}

const PIXELS_PER_SCROLL_LINE: f32 = 20.0;

fn map_scroll_delta(delta: MouseScrollDelta, scale: f64) -> (f64, f64) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    match delta {
        MouseScrollDelta::LineDelta(x, y) => (
            (x * PIXELS_PER_SCROLL_LINE) as f64,
            (y * PIXELS_PER_SCROLL_LINE) as f64,
        ),
        MouseScrollDelta::PixelDelta(pos) => (pos.x / scale, pos.y / scale),
    }
}

fn is_pressed(state: ElementState) -> bool {
    matches!(state, ElementState::Pressed)
}

/// Pointer button, including the back/forward pair.
fn map_button_source(source: ButtonSource) -> PointerButton {
    match source {
        ButtonSource::Mouse(button) => match button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Auxiliary,
            MouseButton::Back => PointerButton::Other(3),
            MouseButton::Forward => PointerButton::Other(4),
            MouseButton::Other(code) => PointerButton::Other(code.min(u8::MAX as u16) as u8),
        },
        // A touch contact is reported as a primary button press — which is
        // precisely how a touch becomes a click in a mobile UI.
        ButtonSource::Touch { .. } | ButtonSource::TabletTool { .. } | ButtonSource::Unknown(_) => {
            PointerButton::Primary
        }
    }
}

/// Logical/physical key → canonical code. Prefers the layout-resolved key so text
/// input follows the keyboard layout, and falls back to the physical code for
/// keys the layout leaves unnamed. Mirrors the desktop backend's `map_key`.
fn map_key(logical: &WinitKey, physical: PhysicalKey) -> KeyCode {
    match logical {
        WinitKey::Named(named) => {
            let code = map_named(*named);
            if code != KeyCode::NONE {
                code
            } else {
                map_physical(physical)
            }
        }
        WinitKey::Character(text) => {
            // A layout may produce a multi-character string (dead-key
            // composition); the first scalar is the key identity, the rest
            // arrives through the IME/text path.
            let code = text
                .chars()
                .next()
                .map(KeyCode::from_char)
                .unwrap_or(KeyCode::NONE);
            if code != KeyCode::NONE {
                code
            } else {
                map_physical(physical)
            }
        }
        // `Dead` and `Unidentified` carry no usable character.
        _ => map_physical(physical),
    }
}

fn map_named(named: winit::keyboard::NamedKey) -> KeyCode {
    use winit::keyboard::NamedKey;
    match named {
        NamedKey::Enter => KeyCode::ENTER,
        NamedKey::Escape => KeyCode::ESCAPE,
        NamedKey::Tab => KeyCode::TAB,
        NamedKey::Backspace => KeyCode::BACKSPACE,
        NamedKey::Delete => KeyCode::DELETE,
        NamedKey::ArrowUp => KeyCode::ARROW_UP,
        NamedKey::ArrowDown => KeyCode::ARROW_DOWN,
        NamedKey::ArrowLeft => KeyCode::ARROW_LEFT,
        NamedKey::ArrowRight => KeyCode::ARROW_RIGHT,
        NamedKey::Home => KeyCode::HOME,
        NamedKey::End => KeyCode::END,
        NamedKey::PageUp => KeyCode::PAGE_UP,
        NamedKey::PageDown => KeyCode::PAGE_DOWN,
        NamedKey::Shift => KeyCode::SHIFT,
        NamedKey::Control => KeyCode::CONTROL,
        NamedKey::Alt | NamedKey::AltGraph => KeyCode::ALT,
        NamedKey::Meta => KeyCode::META,
        NamedKey::CapsLock => KeyCode::CAPS_LOCK,
        NamedKey::Insert => KeyCode::INSERT,
        NamedKey::ContextMenu => KeyCode::CONTEXT_MENU,
        NamedKey::PrintScreen => KeyCode::PRINT_SCREEN,
        NamedKey::ScrollLock => KeyCode::SCROLL_LOCK,
        NamedKey::Pause => KeyCode::PAUSE,
        NamedKey::NumLock => KeyCode::NUM_LOCK,
        NamedKey::F1 => KeyCode::function(1),
        NamedKey::F2 => KeyCode::function(2),
        NamedKey::F3 => KeyCode::function(3),
        NamedKey::F4 => KeyCode::function(4),
        NamedKey::F5 => KeyCode::function(5),
        NamedKey::F6 => KeyCode::function(6),
        NamedKey::F7 => KeyCode::function(7),
        NamedKey::F8 => KeyCode::function(8),
        NamedKey::F9 => KeyCode::function(9),
        NamedKey::F10 => KeyCode::function(10),
        NamedKey::F11 => KeyCode::function(11),
        NamedKey::F12 => KeyCode::function(12),
        _ => KeyCode::NONE,
    }
}

/// Physical code → canonical code. Only needed for keys with no logical name,
/// so it is deliberately small: printable keys come from the layout.
fn map_physical(physical: PhysicalKey) -> KeyCode {
    let PhysicalKey::Code(code) = physical else {
        return KeyCode::NONE;
    };
    match code {
        WinitKeyCode::Space => KeyCode::SPACE,
        WinitKeyCode::Enter | WinitKeyCode::NumpadEnter => KeyCode::ENTER,
        WinitKeyCode::Escape => KeyCode::ESCAPE,
        WinitKeyCode::Tab => KeyCode::TAB,
        WinitKeyCode::Backspace => KeyCode::BACKSPACE,
        WinitKeyCode::Delete => KeyCode::DELETE,
        WinitKeyCode::ArrowUp => KeyCode::ARROW_UP,
        WinitKeyCode::ArrowDown => KeyCode::ARROW_DOWN,
        WinitKeyCode::ArrowLeft => KeyCode::ARROW_LEFT,
        WinitKeyCode::ArrowRight => KeyCode::ARROW_RIGHT,
        WinitKeyCode::Home => KeyCode::HOME,
        WinitKeyCode::End => KeyCode::END,
        WinitKeyCode::PageUp => KeyCode::PAGE_UP,
        WinitKeyCode::PageDown => KeyCode::PAGE_DOWN,
        WinitKeyCode::ShiftLeft | WinitKeyCode::ShiftRight => KeyCode::SHIFT,
        WinitKeyCode::ControlLeft | WinitKeyCode::ControlRight => KeyCode::CONTROL,
        WinitKeyCode::AltLeft | WinitKeyCode::AltRight => KeyCode::ALT,
        WinitKeyCode::MetaLeft | WinitKeyCode::MetaRight => KeyCode::META,
        WinitKeyCode::CapsLock => KeyCode::CAPS_LOCK,
        WinitKeyCode::Insert => KeyCode::INSERT,
        WinitKeyCode::ContextMenu => KeyCode::CONTEXT_MENU,
        WinitKeyCode::PrintScreen => KeyCode::PRINT_SCREEN,
        WinitKeyCode::ScrollLock => KeyCode::SCROLL_LOCK,
        WinitKeyCode::Pause => KeyCode::PAUSE,
        WinitKeyCode::NumLock => KeyCode::NUM_LOCK,
        WinitKeyCode::KeyA => KeyCode::letter(0),
        WinitKeyCode::KeyB => KeyCode::letter(1),
        WinitKeyCode::KeyC => KeyCode::letter(2),
        WinitKeyCode::KeyD => KeyCode::letter(3),
        WinitKeyCode::KeyE => KeyCode::letter(4),
        WinitKeyCode::KeyF => KeyCode::letter(5),
        WinitKeyCode::KeyG => KeyCode::letter(6),
        WinitKeyCode::KeyH => KeyCode::letter(7),
        WinitKeyCode::KeyI => KeyCode::letter(8),
        WinitKeyCode::KeyJ => KeyCode::letter(9),
        WinitKeyCode::KeyK => KeyCode::letter(10),
        WinitKeyCode::KeyL => KeyCode::letter(11),
        WinitKeyCode::KeyM => KeyCode::letter(12),
        WinitKeyCode::KeyN => KeyCode::letter(13),
        WinitKeyCode::KeyO => KeyCode::letter(14),
        WinitKeyCode::KeyP => KeyCode::letter(15),
        WinitKeyCode::KeyQ => KeyCode::letter(16),
        WinitKeyCode::KeyR => KeyCode::letter(17),
        WinitKeyCode::KeyS => KeyCode::letter(18),
        WinitKeyCode::KeyT => KeyCode::letter(19),
        WinitKeyCode::KeyU => KeyCode::letter(20),
        WinitKeyCode::KeyV => KeyCode::letter(21),
        WinitKeyCode::KeyW => KeyCode::letter(22),
        WinitKeyCode::KeyX => KeyCode::letter(23),
        WinitKeyCode::KeyY => KeyCode::letter(24),
        WinitKeyCode::KeyZ => KeyCode::letter(25),
        WinitKeyCode::Digit0 => KeyCode::digit(0),
        WinitKeyCode::Digit1 => KeyCode::digit(1),
        WinitKeyCode::Digit2 => KeyCode::digit(2),
        WinitKeyCode::Digit3 => KeyCode::digit(3),
        WinitKeyCode::Digit4 => KeyCode::digit(4),
        WinitKeyCode::Digit5 => KeyCode::digit(5),
        WinitKeyCode::Digit6 => KeyCode::digit(6),
        WinitKeyCode::Digit7 => KeyCode::digit(7),
        WinitKeyCode::Digit8 => KeyCode::digit(8),
        WinitKeyCode::Digit9 => KeyCode::digit(9),
        WinitKeyCode::F1 => KeyCode::function(1),
        WinitKeyCode::F2 => KeyCode::function(2),
        WinitKeyCode::F3 => KeyCode::function(3),
        WinitKeyCode::F4 => KeyCode::function(4),
        WinitKeyCode::F5 => KeyCode::function(5),
        WinitKeyCode::F6 => KeyCode::function(6),
        WinitKeyCode::F7 => KeyCode::function(7),
        WinitKeyCode::F8 => KeyCode::function(8),
        WinitKeyCode::F9 => KeyCode::function(9),
        WinitKeyCode::F10 => KeyCode::function(10),
        WinitKeyCode::F11 => KeyCode::function(11),
        WinitKeyCode::F12 => KeyCode::function(12),
        _ => KeyCode::NONE,
    }
}
