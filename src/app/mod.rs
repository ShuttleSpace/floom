#[cfg(target_os = "macos")]
pub(crate) mod delegate;
pub(crate) mod handle;
pub(crate) mod winit_embedder;

use std::{
    cell::RefCell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::platform::menu_types::MenuId;
#[cfg(feature = "crossbeam")]
use crossbeam::channel::{Receiver, Sender, unbounded as channel};
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
use peniko::kurbo::Point;
#[cfg(not(feature = "crossbeam"))]
use std::sync::mpsc::{Receiver, Sender, channel};

use floem_legacy_reactive::{Runtime, WriteSignal};
use parking_lot::Mutex;
use raw_window_handle::HasDisplayHandle;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy},
    window::{Theme, WindowId},
};

use crate::{
    action::{Timer, TimerToken},
    inspector::{Capture, profiler::Profile},
    platform::clipboard::Clipboard,
    view::IntoView,
    window::{WindowConfig, WindowCreation},
};
use handle::ApplicationHandle;

pub(crate) type AppEventCallback = dyn Fn(AppEvent);

static EVENT_LOOP_PROXY: Mutex<Option<(EventLoopProxy, Sender<UserEvent>)>> = Mutex::new(None);
static APP_UPDATE_POSTED: AtomicBool = AtomicBool::new(false);

thread_local! {
    pub(crate) static APP_UPDATE_EVENTS: RefCell<Vec<AppUpdateEvent>> = Default::default();
}

#[derive(Debug)]
pub struct AppConfig {
    pub(crate) exit_on_close: bool,
    pub(crate) wgpu_features: wgpu::Features,
    pub(crate) wgpu_backends: Option<wgpu::Backends>,
    pub(crate) global_theme_override: Option<Theme>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            exit_on_close: !cfg!(target_os = "macos"),
            wgpu_features: wgpu::Features::default(),
            wgpu_backends: None,
            global_theme_override: None,
        }
    }
}

impl AppConfig {
    /// Sets whether the application should exit when the last window is closed.
    #[inline]
    pub fn exit_on_close(mut self, exit_on_close: bool) -> Self {
        self.exit_on_close = exit_on_close;
        self
    }

    /// Sets the WGPU features to be used by the application.
    #[inline]
    pub fn wgpu_features(mut self, features: wgpu::Features) -> Self {
        self.wgpu_features = features;
        self
    }

    /// Sets the global theme.
    #[inline]
    pub fn set_global_theme(mut self, theme: Theme) -> Self {
        self.global_theme_override = Some(theme);
        self
    }
}

/// Initializes and runs an application with a single window.
///
/// This function creates a new `Application`, sets up a window with the provided view,
/// and starts the application event loop. The `app_view` closure is used to define
/// the root view of the application window.
///
/// Example:
/// ```no_run
/// floem::launch(|| "Hello, World!")
/// ```
///
/// To build an application and windows with more configuration, see [`Application`].
#[cfg_attr(debug_assertions, track_caller)]
pub fn launch<V: IntoView + 'static>(app_view: impl FnOnce() -> V + 'static) {
    Application::new().window(move |_| app_view(), None).run()
}

pub enum AppEvent {
    WillTerminate,
    Reopen { has_visible_windows: bool },
}

#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
pub(crate) struct MenuWrapper(pub(crate) muda::Menu);
// SAFETY: these unsafe wappers are needed so that we can send the muda memu.
// The muda menu internally uses RC on a String ID and it's Vec of children.
// This unsafe wrapper is memory safe but the race condition could potentially (unlikely)
// lead to bad reference counts and leaked memory.
// I think this is fine for this case.
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
unsafe impl Send for MenuWrapper {}
#[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
unsafe impl Sync for MenuWrapper {}

pub(crate) enum UserEvent {
    AppUpdate,
    Idle,
    QuitApp,
    #[allow(dead_code)]
    Reopen {
        has_visible_windows: bool,
    },
    #[cfg(not(feature = "skia"))]
    GpuResourcesUpdate {
        window_id: WindowId,
    },
    #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
    ShowContextMenu {
        window_id: WindowId,
        menu: MenuWrapper,
        pos: Option<Point>,
    },
    /// iOS soft-keyboard / IME edit produced by the hidden `UITextField`
    /// delegate (`platform::ios_ime`). Carries the winit window id whose text
    /// input is focused plus the neutral IME edit to inject as `Event::Ime`.
    #[cfg(target_os = "ios")]
    IosIme {
        window_id: winit::window::WindowId,
        event: crate::platform::ios_ime::IosImeEvent,
    },
}

#[allow(clippy::large_enum_variant)]
pub(crate) enum AppUpdateEvent {
    NewWindow {
        window_creation: WindowCreation,
    },
    CloseWindow {
        window_id: WindowId,
    },
    RequestCloseWindow {
        window_id: WindowId,
    },
    CaptureWindow {
        window_id: WindowId,
        capture: WriteSignal<Option<Rc<Capture>>>,
    },
    ProfileWindow {
        window_id: WindowId,
        end_profile: Option<WriteSignal<Option<Rc<Profile>>>>,
    },
    RequestTimer {
        timer: Timer,
    },
    RequestAnimationTimer {
        timer: Timer,
        window_id: WindowId,
    },
    CancelTimer {
        timer: TimerToken,
    },
    MenuAction {
        action_id: MenuId,
    },
    ThemeChanged {
        theme: Theme,
    },
    AnimationFrame(bool, WindowId),
}

pub(crate) fn add_app_update_event(event: AppUpdateEvent) {
    APP_UPDATE_EVENTS.with(|events| {
        events.borrow_mut().push(event);
    });
    Application::request_update();
}

/// Drain the pending app update events and return how many of them were
/// `CloseWindow` for the given `window_id`.  This is intended for testing
/// the `handle_default_behaviors` close logic.
#[doc(hidden)]
pub fn take_close_window_event_count(window_id: WindowId) -> usize {
    APP_UPDATE_EVENTS.with(|events| {
        let mut events = events.borrow_mut();
        let count = events
            .iter()
            .filter(
                |e| matches!(e, AppUpdateEvent::CloseWindow { window_id: id } if *id == window_id),
            )
            .count();
        events.retain(
            |e| !matches!(e, AppUpdateEvent::CloseWindow { window_id: id } if *id == window_id),
        );
        count
    })
}

/// Floem top level application
/// This is the entry point of the application.
pub struct Application {
    receiver: Receiver<UserEvent>,
    handle: ApplicationHandle,
    event_loop: Option<EventLoop>,
    initial_windows: Vec<WindowCreation>,
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for Application {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Some(window_creation) = self.initial_windows.pop() {
            self.handle.new_window(
                event_loop,
                window_creation.view_fn,
                self.handle.config.global_theme_override,
                window_creation.config.unwrap_or_default(),
            );
        }
        // On Android the native window is recreated on every pause/resume and rotation. When a new
        // native window appears we must rebuild the (now invalid) wgpu surface against it.
        #[cfg(target_os = "android")]
        self.handle.recreate_lost_android_surfaces();
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        self.handle.handle_timer(event_loop);
        self.handle
            .handle_window_event(window_id, event, event_loop);
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.handle.handle_timer(event_loop);
        for event in self.receiver.try_iter() {
            self.handle.handle_user_event(event_loop, event);
        }
        if Runtime::has_pending_work() {
            self.handle.request_update();
        }
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn ActiveEventLoop) {
        // On Android the native window is about to be destroyed by the OS. Mark every surface as
        // lost so we stop painting against it (and recreate it in `can_create_surfaces`).
        #[cfg(target_os = "android")]
        self.handle.mark_android_surfaces_lost();
        if let Some(action) = self.handle.event_listener.as_ref() {
            action(AppEvent::WillTerminate);
        }
    }

    fn about_to_wait(&mut self, event_loop: &dyn ActiveEventLoop) {
        #[cfg(not(any(target_os = "android", target_os = "ios", target_env = "ohos")))]
        self.handle.flush_deferred_context_menus();
        self.handle.handle_timer(event_loop);
        if Runtime::has_pending_work() {
            self.handle.request_update();
        }
    }
}

impl Application {
    pub fn new() -> Self {
        Self::new_with_config(AppConfig::default())
    }
    pub fn new_with_config(config: AppConfig) -> Self {
        let event_loop = EventLoop::new().expect("can't start the event loop");

        #[cfg(target_os = "macos")]
        delegate::set_app_delegate();

        let event_loop_proxy = event_loop.create_proxy();
        let (sender, receiver) = channel();

        *EVENT_LOOP_PROXY.lock() = Some((event_loop_proxy.clone(), sender));
        unsafe {
            Clipboard::init(event_loop.display_handle().unwrap().as_raw());
        }
        let handle = ApplicationHandle::new(config);

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        muda::MenuEvent::set_event_handler(Some(move |event: muda::MenuEvent| {
            add_app_update_event(AppUpdateEvent::MenuAction {
                action_id: event.id,
            });
        }));

        Self {
            receiver,
            handle,
            event_loop: Some(event_loop),
            initial_windows: Vec::new(),
        }
    }

    /// Create an `Application` bound to the Android `NativeActivity`.
    ///
    /// Unlike [`Application::new_with_config`], which builds a default
    /// `EventLoop`, this associates the `AndroidApp` passed to `android_main()`
    /// with the event loop via winit's `EventLoopBuilderExtAndroid`. The
    /// returned application is then driven exactly like the desktop one
    /// (`.window(...).run()`).
    #[cfg(target_os = "android")]
    pub fn new_android(
        app: winit::platform::android::activity::AndroidApp,
        mut config: AppConfig,
    ) -> Self {
        use winit::platform::android::EventLoopBuilderExtAndroid;

        // Android only supports the Vulkan backend; probing other backends is pointless and can
        // error. Default to Vulkan unless the caller explicitly chose backends.
        if config.wgpu_backends.is_none() {
            config.wgpu_backends = Some(wgpu::Backends::VULKAN);
        }

        let mut builder = EventLoop::builder();
        // Capture JNI handles before `with_android_app` moves `app` into the
        // builder; the clipboard bridge needs the JavaVM + Activity pointers.
        let clipboard_vm = app.vm_as_ptr();
        let clipboard_activity = app.activity_as_ptr() as jni::sys::jobject;
        builder.with_android_app(app);
        let event_loop = builder
            .build()
            .expect("can't start the android event loop");

        let event_loop_proxy = event_loop.create_proxy();
        let (sender, receiver) = channel();

        *EVENT_LOOP_PROXY.lock() = Some((event_loop_proxy.clone(), sender));
        unsafe {
            // Initialize the shared Android JNI bridge from the AndroidApp's
            // JNI handles (the legacy `display`-based init has no clipboard on
            // Android). The bridge is used by the clipboard, external-URL
            // opening, file dialogs, and receiving shared files.
            Clipboard::init_android(clipboard_vm, clipboard_activity);
        }
        #[cfg(target_os = "android")]
        {
            // Read the launch intent (if the app was started by a share). Safe
            // with the default `NativeActivity`; the JNI bridge is now ready.
            crate::platform::android_intent::read_launch_intent();
        }
        let handle = ApplicationHandle::new(config);

        Self {
            receiver,
            handle,
            event_loop: Some(event_loop),
            initial_windows: Vec::new(),
        }
    }

    /// Create an `Application` for iOS.
    ///
    /// Unlike [`Application::new_with_config`], this builds a plain
    /// `EventLoop` with no `AndroidApp`/JNI handle: the lapce winit fork's iOS
    /// backend turns the Rust `fn main` into a proper UIKit app — `EventLoop::run`
    /// calls `UIApplicationMain` with winit's own `UIApplicationDelegate`. Floem's
    /// existing winit event path (pointer/keyboard/IME) therefore reuses with no
    /// custom input translation. (No `EventLoopBuilderExtIOS` exists in this
    /// fork; a plain `EventLoop` is sufficient.) See `docs/ios.md`.
    #[cfg(target_os = "ios")]
    pub fn new_ios(mut config: AppConfig) -> Self {
        // iOS only supports the Metal backend; probing other backends is
        // pointless and can error. Default to Metal unless the caller chose.
        if config.wgpu_backends.is_none() {
            config.wgpu_backends = Some(wgpu::Backends::METAL);
        }

        // The iOS backend owns the UIKit lifecycle; no desktop `Clipboard::init`
        // (display-handle clipboard), no macOS app delegate, and no muda
        // `MenuEvent` handler (muda has no iOS backend — iOS uses the in-UI
        // context menu like Android).
        let event_loop = EventLoop::new().expect("can't start the ios event loop");

        let event_loop_proxy = event_loop.create_proxy();
        let (sender, receiver) = channel();

        *EVENT_LOOP_PROXY.lock() = Some((event_loop_proxy.clone(), sender));
        let handle = ApplicationHandle::new(config);

        // Install the share/intent hook (iOS Step 8): inject
        // `application:openURL:options:` onto winit's `WinitApplicationDelegate`
        // so files shared into the app are delivered to the registered
        // `set_shared_files_handler`. Scheduled on the main queue because winit
        // registers that class lazily inside `EventLoop::run()`.
        crate::platform::ios_receive::install_openurl_hook();

        Self {
            receiver,
            handle,
            event_loop: Some(event_loop),
            initial_windows: Vec::new(),
        }
    }

    /// Create an `Application` for OpenHarmony (HarmonyOS).
    ///
    /// Unlike Android/iOS/macOS, the event loop is **not** created here. The
    /// `floem-winit-ohos` runtime owns the OHOS event loop: its `runtime_new`
    /// calls `EventLoop::bootstrap()` and then drives this `Application` as the
    /// `ApplicationHandler` from a worker thread via `pump_app_events`. So this
    /// constructor only builds the handler state (`event_loop` stays `None`);
    /// the real loop lives inside `floem-winit-ohos`.
    ///
    /// # Compile-time note
    ///
    /// `floem-winit-ohos`'s `RuntimeApplication` requires
    /// `floem_winit_core::application::ApplicationHandler`. That is the **same**
    /// trait Floem implements: `floem-winit` (the facade Floem depends on)
    /// re-exports `floem-winit-core::application::ApplicationHandler`, and
    /// `floem-winit-core` is a single shared crate instance used by both
    /// `floem-winit` and `floem-winit-ohos`. So the `ApplicationHandler` bound
    /// is satisfied — the two traits unify through `floem-winit-core`.
    ///
    /// The fork's runtime owns the OHOS event loop **and** the `ApplicationHandler`
    /// entirely on its worker thread: the handler is created inside the thread
    /// via `AppFactory` and never crosses a thread boundary, so `RuntimeApplication`
    /// does **not** require `Send`. Floem's `Application` is intentionally
    /// `!Send` (its reactivity layer uses `Rc`-backed signals and
    /// `Box<dyn Fn>` callbacks), so the fork's `Send` bound was relaxed to
    /// match. The method set (`can_create_surfaces` / `window_event` /
    /// `about_to_wait` / `proxy_wake_up` / `destroy_surfaces`) already matches.
    #[cfg(target_env = "ohos")]
    pub fn new_ohos(mut config: AppConfig) -> Self {
        // OHOS uses the Vulkan backend via VK_OHOS_surface (requires wgpu >= 0.30).
        if config.wgpu_backends.is_none() {
            config.wgpu_backends = Some(wgpu::Backends::VULKAN);
        }

        let (_sender, receiver) = channel();
        let handle = ApplicationHandle::new(config);

        Self {
            receiver,
            handle,
            event_loop: None,
            initial_windows: Vec::new(),
        }
    }

    /// Factory handed to `floem_winit_ohos::export_ohos_winit_app!`.
    ///
    /// `runtime_new` calls this once to obtain the `ApplicationHandler` it
    /// pumps. Returns an `Application` built by [`Application::new_ohos`].
    ///
    /// NOTE: this stub returns an `Application` with **no** window. Apps should
    /// normally supply their own factory that calls `.window(...)` (see
    /// `examples/ohos/src/lib.rs`). The `RuntimeApplication` bound is satisfied
    /// now that the fork no longer requires `Send` — the OHOS runtime owns the
    /// handler on its worker thread, so `Application`'s `!Send` reactivity
    /// layer is not a problem.
    #[cfg(target_env = "ohos")]
    pub fn floem_app_factory() -> Self {
        Self::new_ohos(AppConfig::default())
    }

    pub fn on_event(mut self, action: impl Fn(AppEvent) + 'static) -> Self {
        self.handle.event_listener = Some(Box::new(action));
        self
    }

    /// Create a new window for the application, if you want multiple windows,
    /// just chain more window method to the builder.
    ///
    /// # Note
    ///
    /// Using `None` as a configuration argument is equivalent to using
    /// `WindowConfig::default()`.
    pub fn window<V: IntoView + 'static>(
        mut self,
        app_view: impl FnOnce(WindowId) -> V + 'static,
        config: Option<WindowConfig>,
    ) -> Self {
        self.initial_windows.push(WindowCreation {
            view_fn: Box::new(move |window_id: WindowId| app_view(window_id).into_any()),
            config,
        });
        self
    }

    #[cfg_attr(debug_assertions, track_caller)]
    pub fn run(mut self) {
        Runtime::init_on_ui_thread();
        // Nudge UI when sync signals are updated from other threads.
        Runtime::set_sync_effect_waker(|| Application::send_proxy_event(UserEvent::Idle));
        let event_loop = self.event_loop.take().unwrap();
        let _ = event_loop.run_app(self);
    }

    pub(crate) fn send_proxy_event(event: UserEvent) {
        if let Some((proxy, sender)) = EVENT_LOOP_PROXY.lock().as_ref() {
            let _ = sender.send(event);
            proxy.wake_up();
        }
    }

    pub(crate) fn request_update() {
        if !APP_UPDATE_POSTED.swap(true, Ordering::AcqRel) {
            Self::send_proxy_event(UserEvent::AppUpdate);
        }
    }

    pub(crate) fn clear_update_posted() {
        APP_UPDATE_POSTED.store(false, Ordering::Release);
    }
}

/// Immediately terminates the application.
///
/// This is an unconditional exit: no windows receive a `CloseRequested` event
/// and no `WindowCloseRequested` handlers are consulted. The event loop exits
/// immediately.
pub fn quit_app() {
    Application::send_proxy_event(UserEvent::QuitApp);
}

/// Signals the application to reopen.
///
/// This function sends a `Reopen` event to the application's event loop.
/// It is safe to call from any thread.
pub fn reopen() {
    Application::send_proxy_event(UserEvent::Reopen {
        has_visible_windows: false,
    });
}
