use std::any::Any;
use std::ffi::c_void;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use floem_winit_core::application::ApplicationHandler;
use floem_winit_core::event_loop::pump_events::{EventLoopExtPumpEvents, PumpStatus};

use crate::{
    EventLoop, OhosApp, OhosKeyAction, OhosMouseAction, OhosMouseButton, OhosPointerSource,
    OhosTouchAction,
};

// NOTE: `RuntimeApplication` intentionally does **not** require `Send`.
//
// Safety / rationale — `runtime_new` (below) spawns a worker thread and
// *creates* the user-supplied `ApplicationHandler` **inside** that thread via
// `AppFactory`. The handler object therefore never crosses a thread boundary:
// it is born on the worker thread and pumped there (via `pump_app_events`)
// until the runtime is torn down. The only value moved *into* the worker is
// the OHOS `EventLoop`, which **is** `Send`; the `ApplicationHandler` itself
// is not moved.
//
// Requiring `Send` would force every `ApplicationHandler` — in particular
// `floem::Application`, whose reactivity layer is built on `Rc`-backed
// signals, `Box<dyn Fn>` callbacks and `*mut`-carrying platform backends — to
// be `Send`, which fights Floem's deliberately single-threaded design. Because
// the handler is thread-local to the worker, the bound is unnecessary and is
// omitted here.
pub trait RuntimeApplication: ApplicationHandler {}

impl<T> RuntimeApplication for T where T: ApplicationHandler {}

pub type AppFactory = fn() -> Box<dyn RuntimeApplication>;

pub struct OhosRuntime {
    app: OhosApp,
    stop_requested: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

pub fn runtime_new(factory: AppFactory) -> *mut OhosRuntime {
    let event_loop = match EventLoop::bootstrap() {
        Ok(event_loop) => event_loop,
        Err(_) => return std::ptr::null_mut(),
    };
    let app = event_loop.ohos_app().clone();
    let stop_requested = Arc::new(AtomicBool::new(false));

    // Expose the loop's proxy so the app factory (which runs on the worker
    // thread below) can wire floem's cross-thread notifications into this loop.
    crate::set_runtime_event_loop_proxy(event_loop.create_proxy());
    crate::deveco_log("runtime_new: event loop bootstrapped, spawning worker");

    let worker_stop = stop_requested.clone();
    let worker = thread::spawn(move || {
        let runtime = AssertUnwindSafe(move || {
            // Floem's core logging (log/tracing) has no subscriber on OHOS, so
            // panics would otherwise be silent. Log every panic to hilog, then
            // defer to the previous hook (stderr).
            let previous_hook = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                crate::deveco_log_with_level(
                    crate::OhosLogLevel::Error,
                    format!("panic: {info}"),
                );
                previous_hook(info);
            }));
            let mut event_loop = event_loop;
            let mut application = factory();
            loop {
                match event_loop
                    .pump_app_events(Some(Duration::from_millis(16)), application.as_mut())
                {
                    PumpStatus::Continue if worker_stop.load(Ordering::Acquire) => break,
                    PumpStatus::Continue => {}
                    PumpStatus::Exit(_) => break,
                }
            }
        });

        if let Err(payload) = panic::catch_unwind(runtime) {
            crate::deveco_log_with_level(
                crate::OhosLogLevel::Error,
                format!(
                    "floem-winit-ohos runtime worker panicked: {}",
                    panic_payload_to_string(payload.as_ref())
                ),
            );
        }
    });

    Box::into_raw(Box::new(OhosRuntime {
        app,
        stop_requested,
        worker: Some(worker),
    }))
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_free(runtime: *mut OhosRuntime) {
    if runtime.is_null() {
        return;
    }

    let mut runtime = unsafe { Box::from_raw(runtime) };
    runtime.stop_requested.store(true, Ordering::Release);
    runtime.app.notify_frame();
    if let Some(worker) = runtime.worker.take() {
        let _ = worker.join();
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_surface_created(
    runtime: *const OhosRuntime,
    xcomponent: *mut c_void,
    native_window: *mut c_void,
    width: u32,
    height: u32,
    density_scale: f64,
    font_scale: f64,
) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        crate::deveco_log(format!(
            "ffi: surface_created {width}x{height} density={density_scale:.3} font={font_scale:.3}"
        ));
        runtime.app.notify_surface_created(
            xcomponent,
            native_window,
            width,
            height,
            density_scale,
            font_scale,
        );
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_surface_changed(
    runtime: *const OhosRuntime,
    xcomponent: *mut c_void,
    native_window: *mut c_void,
    width: u32,
    height: u32,
    density_scale: f64,
    font_scale: f64,
) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_surface_changed(
            xcomponent,
            native_window,
            width,
            height,
            density_scale,
            font_scale,
        );
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_surface_destroyed(runtime: *const OhosRuntime) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_surface_destroyed();
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_focus(runtime: *const OhosRuntime, focused: bool) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_focus(focused);
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_visibility(runtime: *const OhosRuntime, visible: bool) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_visibility(visible);
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_low_memory(runtime: *const OhosRuntime) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_low_memory();
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_frame(runtime: *const OhosRuntime) {
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_frame();
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_key(
    runtime: *const OhosRuntime,
    action: u32,
    key_code: u32,
    repeat: bool,
    device_id: i64,
) {
    let Some(action) = OhosKeyAction::from_raw(action) else {
        return;
    };
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_key(action, key_code, repeat, device_id);
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_touch(
    runtime: *const OhosRuntime,
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
    let Some(action) = OhosTouchAction::from_raw(action) else {
        return;
    };
    let Some(source) = OhosPointerSource::from_raw(source) else {
        return;
    };
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime.app.notify_touch(
            action,
            source,
            finger_id,
            x,
            y,
            has_force.then_some(force),
            device_id,
            primary,
        );
    }
}

/// # Safety
///
/// `runtime` must be a pointer previously returned by [`runtime_new`].
pub unsafe fn runtime_mouse(
    runtime: *const OhosRuntime,
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
    let Some(action) = OhosMouseAction::from_raw(action) else {
        return;
    };
    let button = if has_button {
        OhosMouseButton::from_raw(button)
    } else {
        None
    };
    if let Some(runtime) = unsafe { runtime_ref(runtime) } {
        runtime
            .app
            .notify_mouse(action, button, x, y, delta_x, delta_y, device_id, primary);
    }
}

unsafe fn runtime_ref<'a>(runtime: *const OhosRuntime) -> Option<&'a OhosRuntime> {
    unsafe { runtime.as_ref() }
}

fn panic_payload_to_string(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        String::from("non-string panic payload")
    }
}
