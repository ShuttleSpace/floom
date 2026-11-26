//! **FROZEN as of 2026-11-27.** Nothing in the workspace depends on this crate:
//! the shipping OpenHarmony path is `floom-platform-harmony` (N-API + a touch queue),
//! which is the one with device evidence. Keep this for the day someone bridges IME —
//! it is the only layer in the tree that speaks keyboard/composition.
//! OpenHarmony backend for the split Winit architecture.

mod app;
mod event_loop;
pub mod ffi;
mod keycodes;
#[doc(hidden)]
pub mod log;

pub use crate::app::{
    OhosApp, OhosKeyAction, OhosMouseAction, OhosMouseButton, OhosPointerSource, OhosTouchAction,
};
pub use crate::event_loop::{
    ActiveEventLoop, EventLoop, EventLoopBuilder, EventLoopBuilderExtOhos,
    PlatformSpecificEventLoopAttributes, PlatformSpecificWindowAttributes, Window,
};
pub use crate::log::{
    DEFAULT_LOG_DOMAIN, DEFAULT_LOG_TAG, LOG_PREFIX, OhosLogLevel, deveco_log, deveco_log_with,
    deveco_log_with_level,
};
pub use floem_winit_core::application::ApplicationHandler;
pub use floem_winit_core::event::{
    ButtonSource, ElementState, Ime, KeyEvent, MouseButton, MouseScrollDelta, PointerSource,
    WindowEvent,
};
pub use floem_winit_core::event_loop::{
    ActiveEventLoop as RootActiveEventLoop, ControlFlow, EventLoopProxy,
};
pub use floem_winit_core::window::{
    Window as CoreWindow, WindowAttributes, WindowId as WinitWindowId,
};

/// Event-loop proxy of the runtime created by the most recent
/// `ohos_winit_runtime_new()` call.
///
/// `runtime_new` bootstraps the OHOS event loop and then calls the app factory
/// on its worker thread; the factory builds a `floom::Application`, which picks
/// the proxy up (via [`runtime_event_loop_proxy`]) to wire its cross-thread
/// event notifications (`UserEvent::GpuResourcesUpdate`, timers, sync-signal
/// wake-ups) into the runtime's event loop.
static RUNTIME_EVENT_LOOP_PROXY: Mutex<Option<EventLoopProxy>> = Mutex::new(None);

/// Clone of the OHOS runtime's event-loop proxy, if a runtime exists.
pub fn runtime_event_loop_proxy() -> Option<EventLoopProxy> {
    RUNTIME_EVENT_LOOP_PROXY.lock().unwrap().clone()
}

pub(crate) fn set_runtime_event_loop_proxy(proxy: EventLoopProxy) {
    *RUNTIME_EVENT_LOOP_PROXY.lock().unwrap() = Some(proxy);
}

use std::ffi::c_void;
use std::sync::Mutex;

use floem_winit_core::event_loop::ActiveEventLoop as CoreActiveEventLoop;

pub trait ActiveEventLoopExtOhos {
    fn ohos_app(&self) -> &OhosApp;
}

impl ActiveEventLoopExtOhos for dyn CoreActiveEventLoop + '_ {
    fn ohos_app(&self) -> &OhosApp {
        let event_loop = self
            .cast_ref::<crate::event_loop::ActiveEventLoop>()
            .expect("ActiveEventLoop is not an OHOS backend instance");
        event_loop.ohos_app()
    }
}

pub trait WindowExtOhos {
    fn xcomponent_ptr(&self) -> *mut c_void;
    fn native_window_ptr(&self) -> *mut c_void;
    fn density_scale(&self) -> f64;
    fn font_scale(&self) -> f64;
}

impl WindowExtOhos for dyn CoreWindow + '_ {
    fn xcomponent_ptr(&self) -> *mut c_void {
        let window = self
            .cast_ref::<crate::event_loop::Window>()
            .expect("Window is not an OHOS backend instance");
        window.xcomponent_ptr()
    }

    fn native_window_ptr(&self) -> *mut c_void {
        let window = self
            .cast_ref::<crate::event_loop::Window>()
            .expect("Window is not an OHOS backend instance");
        window.native_window_ptr()
    }

    fn density_scale(&self) -> f64 {
        let window = self
            .cast_ref::<crate::event_loop::Window>()
            .expect("Window is not an OHOS backend instance");
        window.density_scale()
    }

    fn font_scale(&self) -> f64 {
        let window = self
            .cast_ref::<crate::event_loop::Window>()
            .expect("Window is not an OHOS backend instance");
        window.font_scale()
    }
}

#[macro_export]
macro_rules! export_ohos_winit_app {
    // An expression, not just a path: the factory may be a closure carrying the
    // app's tree builder, which cannot be a plain `fn` because it owns state.
    ($factory:expr) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn ohos_winit_runtime_new() -> *mut ::std::ffi::c_void {
            $crate::ffi::runtime_new_take($factory) as *mut ::std::ffi::c_void
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_free(runtime: *mut ::std::ffi::c_void) {
            unsafe { $crate::ffi::runtime_free(runtime.cast()) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_log(message: *const ::std::ffi::c_char) {
            unsafe { $crate::log::deveco_log_from_c(message) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_surface_created(
            runtime: *const ::std::ffi::c_void,
            xcomponent: *mut ::std::ffi::c_void,
            native_window: *mut ::std::ffi::c_void,
            width: u32,
            height: u32,
            density_scale: f64,
            font_scale: f64,
        ) {
            unsafe {
                $crate::ffi::runtime_surface_created(
                    runtime.cast(),
                    xcomponent,
                    native_window,
                    width,
                    height,
                    density_scale,
                    font_scale,
                )
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_surface_changed(
            runtime: *const ::std::ffi::c_void,
            xcomponent: *mut ::std::ffi::c_void,
            native_window: *mut ::std::ffi::c_void,
            width: u32,
            height: u32,
            density_scale: f64,
            font_scale: f64,
        ) {
            unsafe {
                $crate::ffi::runtime_surface_changed(
                    runtime.cast(),
                    xcomponent,
                    native_window,
                    width,
                    height,
                    density_scale,
                    font_scale,
                )
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_surface_destroyed(
            runtime: *const ::std::ffi::c_void,
        ) {
            unsafe { $crate::ffi::runtime_surface_destroyed(runtime.cast()) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_focus(
            runtime: *const ::std::ffi::c_void,
            focused: bool,
        ) {
            unsafe { $crate::ffi::runtime_focus(runtime.cast(), focused) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_visibility(
            runtime: *const ::std::ffi::c_void,
            visible: bool,
        ) {
            unsafe { $crate::ffi::runtime_visibility(runtime.cast(), visible) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_low_memory(runtime: *const ::std::ffi::c_void) {
            unsafe { $crate::ffi::runtime_low_memory(runtime.cast()) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_frame(runtime: *const ::std::ffi::c_void) {
            unsafe { $crate::ffi::runtime_frame(runtime.cast()) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_key(
            runtime: *const ::std::ffi::c_void,
            action: u32,
            key_code: u32,
            repeat: bool,
            device_id: i64,
        ) {
            unsafe { $crate::ffi::runtime_key(runtime.cast(), action, key_code, repeat, device_id) }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_touch(
            runtime: *const ::std::ffi::c_void,
            action: u32,
            source: u32,
            finger_id: u64,
            x: f64,
            y: f64,
            force: f64,
            has_force: bool,
            device_id: i64,
            primary: bool,
        ) {
            unsafe {
                $crate::ffi::runtime_touch(
                    runtime.cast(),
                    action,
                    source,
                    finger_id,
                    x,
                    y,
                    force,
                    has_force,
                    device_id,
                    primary,
                )
            }
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn ohos_winit_runtime_mouse(
            runtime: *const ::std::ffi::c_void,
            action: u32,
            button: u32,
            has_button: bool,
            x: f64,
            y: f64,
            delta_x: f64,
            delta_y: f64,
            device_id: i64,
            primary: bool,
        ) {
            unsafe {
                $crate::ffi::runtime_mouse(
                    runtime.cast(),
                    action,
                    button,
                    has_button,
                    x,
                    y,
                    delta_x,
                    delta_y,
                    device_id,
                    primary,
                )
            }
        }
    };
}
