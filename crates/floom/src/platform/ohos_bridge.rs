//! Shared OpenHarmony N-API bridge registration.
//!
//! OpenHarmony exposes OS services only through ArkTS, so every OHOS platform
//! bridge (clipboard, external URL, file picker, receive-files) follows the
//! same pattern: at startup the ArkTS side calls [`register_platform_bridge`]
//! with one object that wraps the ArkTS capabilities, and the Rust side calls
//! *back* into that object through N-API via [`call_bridge`].
//!
//! This mirrors the Android JNI bridge (`crate::platform::android_jni`), which
//! also captures a platform handle once and dispatches from it.
//!
//! # Threading
//!
//! N-API values belong to the JS thread that created them: calling
//! `napi_call_function` from any other thread aborts the process
//! (`Fatal: ecma_vm cannot run in multi-thread!`). Floem's OHOS runtime, however,
//! drives the event loop from a **worker** thread (see
//! `floem-winit-ohos`'s `runtime_new`), and every bridge call — clipboard, file
//! picker, external URL, IME, accessibility — originates there.
//!
//! [`call_bridge`] therefore marshals the call back onto the JS thread through a
//! thread-safe function created in [`register_platform_bridge`], and blocks the
//! calling thread until the JS thread answers (or [`BRIDGE_CALL_TIMEOUT`]
//! elapses, after which the call degrades to the "bridge unavailable"
//! `None` result instead of crashing). Calls made *from* the JS thread run
//! inline.
//!
//! # Status
//! Compiled only for `target_env = "ohos"`. The napi module wiring (`Init`
//! forwarding `napi_register_module_v1` into `libentry.so`) is verified on an
//! OHOS emulator; the marshalling path below is verified to keep the render gate
//! green (no cross-thread abort).

#![cfg(target_env = "ohos")]

use std::any::Any;
use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use napi::{Env, JsObject, NapiRaw, Ref, Result as NapiResult, sys};
use napi_derive::napi;

/// How long a cross-thread bridge call waits for the JS thread to run its job.
///
/// Past this the call gives up and reports the bridge as unavailable. This is a
/// safety valve, not an expected path: the JS thread is idle apart from this
/// dispatch queue, so a round trip normally completes well inside a frame.
const BRIDGE_CALL_TIMEOUT: Duration = Duration::from_millis(5000);

/// A persistent handle to the ArkTS platform bridge object registered at startup.
struct PlatformBridge {
    /// The N-API environment captured at registration time.
    env: Env,
    /// A type-erased reference to the JS object so it is not GC'd.
    object: Ref<()>,
}

// SAFETY: `PlatformBridge` is written once on the JS thread and afterwards only
// *read*. Its `Env` is never dereferenced off the JS thread — [`call_bridge`]
// marshals every cross-thread call back to the thread recorded in
// `OWNER_THREAD` — so asserting `Send`/`Sync` for the `static` is sound.
unsafe impl Send for PlatformBridge {}
unsafe impl Sync for PlatformBridge {}

static BRIDGE: OnceLock<PlatformBridge> = OnceLock::new();
/// The thread that owns [`BRIDGE`]'s `env`; calls from it run inline.
static OWNER_THREAD: OnceLock<ThreadId> = OnceLock::new();
/// The thread-safe function that wakes the JS thread, as a `usize` so the
/// `static` stays `Sync` without a wrapper type.
static TS_FUNCTION: OnceLock<usize> = OnceLock::new();

/// One bridge call in flight: the closure to run on the JS thread, plus the slot
/// that thread fills with the closure's result.
struct Request {
    slot: Mutex<Slot>,
    ready: Condvar,
}

struct Slot {
    job: Option<Job>,
    outcome: Option<Result<Box<dyn Any + Send>, String>>,
    done: bool,
}

/// A bridge closure, type-erased so it can travel through the dispatch queue
/// regardless of its return type.
type Job = Box<dyn FnOnce(Env, JsObject) -> Result<Box<dyn Any + Send>, String> + Send>;

// napi-rs gates its `napi_threadsafe_function` bindings behind the `napi4`
// feature, which this crate does not enable, so declare the two we need here.
// OHOS's N-API library exports both unconditionally.
type ThreadsafeFunction = *mut c_void;
type ThreadsafeFunctionCallJs =
    Option<unsafe extern "C" fn(sys::napi_env, sys::napi_value, *mut c_void, *mut c_void)>;

unsafe extern "C" {
    fn napi_create_threadsafe_function(
        env: sys::napi_env,
        func: sys::napi_value,
        async_resource: sys::napi_value,
        async_resource_name: sys::napi_value,
        max_queue_size: usize,
        initial_thread_count: usize,
        thread_finalize_data: *mut c_void,
        thread_finalize_cb: Option<unsafe extern "C" fn(sys::napi_env, *mut c_void, *mut c_void)>,
        context: *mut c_void,
        call_js_cb: ThreadsafeFunctionCallJs,
        result: *mut ThreadsafeFunction,
    ) -> sys::napi_status;

    fn napi_call_threadsafe_function(
        func: ThreadsafeFunction,
        data: *mut c_void,
        is_blocking: i32,
    ) -> sys::napi_status;
}

/// Runs **on the JS thread**: run the posted [`Request`]'s job against the
/// registered bridge object, store its result, and wake the waiting thread.
///
/// # Safety
///
/// `data` must be a pointer produced by `Arc::into_raw` on a [`Request`], i.e.
/// the value handed to `napi_call_threadsafe_function` by [`call_bridge`].
unsafe extern "C" fn dispatch_on_js_thread(
    _env: sys::napi_env,
    _js_callback: sys::napi_value,
    _context: *mut c_void,
    data: *mut c_void,
) {
    if data.is_null() {
        return;
    }
    // Reclaim the strong reference handed over in `call_bridge`; it is dropped
    // at the end of this function, after the job has finished.
    let request = unsafe { Arc::from_raw(data as *const Request) };

    let job = request.slot.lock().ok().and_then(|mut slot| slot.job.take());
    let outcome = match job {
        Some(job) => match BRIDGE.get() {
            // `BRIDGE.env` is this thread's env, so these N-API calls are legal.
            Some(bridge) => match bridge.env.get_reference_value::<JsObject>(&bridge.object) {
                Ok(object) => job(bridge.env, object),
                Err(err) => Err(err.to_string()),
            },
            None => Err("platform bridge not registered".to_owned()),
        },
        None => Err("platform bridge request already dispatched".to_owned()),
    };

    if let Ok(mut slot) = request.slot.lock() {
        slot.outcome = Some(outcome);
        slot.done = true;
    }
    request.ready.notify_all();
}

/// Create the thread-safe function that lets [`call_bridge`] hop onto the JS
/// thread. Must run on the JS thread, i.e. from [`register_platform_bridge`].
///
/// A failure here is not fatal: cross-thread calls then report the bridge as
/// unavailable ([`call_bridge`] returns `None`) instead of crashing.
fn init_dispatch_queue(env: Env) {
    if TS_FUNCTION.get().is_some() {
        return;
    }
    let Ok(resource_name) = env.create_string("floem_platform_bridge") else {
        return;
    };
    let mut handle: ThreadsafeFunction = ptr::null_mut();
    let status = unsafe {
        napi_create_threadsafe_function(
            env.raw(),
            // No callable `func`: the work is delivered through `call_js_cb`.
            ptr::null_mut(),
            ptr::null_mut(),
            resource_name.raw(),
            // 0 = unbounded queue, 1 = this thread participates in the TSFN's
            // lifetime (it is never released: the bridge lives as long as the
            // process).
            0,
            1,
            ptr::null_mut(),
            None,
            ptr::null_mut(),
            Some(dispatch_on_js_thread),
            &mut handle,
        )
    };
    if status == sys::Status::napi_ok && !handle.is_null() {
        let _ = TS_FUNCTION.set(handle as usize);
    }
}

/// Exported to ArkTS. Call **once** during ability / XComponent startup with a
/// single object that implements the union of the platform capabilities:
///
/// ```typescript
/// {
///   getText(): string | null;
///   setText(text: string): void;
///   openUrl(url: string): void;
///   openFile(opts): Promise<Array<string> | null>;   // files
///   saveFile(opts): Promise<string | null>;           // file
///   setAccessibilityTree(tree): void;                 // a11y
///   showKeyboard(): void;                             // IME (see ohos_ime.rs)
///   hideKeyboard(): void;                             // IME
/// }
/// ```
///
/// (`receive_files` is the reverse direction — ArkTS pushes shared files into
/// Rust via [`crate::platform::ohos_receive::on_shared_files`]; IME text flows
/// back through the `on_ime_*` exports in [`crate::platform::ohos_ime`].)
#[napi]
pub fn register_platform_bridge(env: Env, bridge: JsObject) -> NapiResult<()> {
    let object = env.create_reference(bridge)?;
    // Ignore a second registration; keep the first handle (and its queue).
    if BRIDGE.set(PlatformBridge { env, object }).is_err() {
        return Ok(());
    }
    let _ = OWNER_THREAD.set(std::thread::current().id());
    init_dispatch_queue(env);
    Ok(())
}

/// Run `f` against the registered bridge object **on the JS thread**, or return
/// `None` if no bridge has been registered yet (ArkTS startup not reached, or
/// registration failed) or the JS thread did not answer within
/// [`BRIDGE_CALL_TIMEOUT`].
///
/// `f` must be `Send + 'static` because it is handed to the JS thread; its
/// result must be `Send` for the same reason. Any `napi::Error` from `f`, and
/// any timeout, is flattened to `None` so callers fall back to their
/// capability-unavailable error.
pub(crate) fn call_bridge<R>(
    f: impl FnOnce(Env, JsObject) -> NapiResult<R> + Send + 'static,
) -> Option<R>
where
    R: Send + 'static,
{
    let bridge = BRIDGE.get()?;

    // Fast path: already on the thread that owns `env`.
    if OWNER_THREAD.get() == Some(&std::thread::current().id()) {
        let object = bridge.env.get_reference_value::<JsObject>(&bridge.object).ok()?;
        return f(bridge.env, object).ok();
    }

    // Cross-thread path: hand the job to the JS thread and wait for its result.
    let handle = *TS_FUNCTION.get()? as ThreadsafeFunction;
    if handle.is_null() {
        return None;
    }

    let job: Job = Box::new(move |env, object| {
        f(env, object)
            .map(|value| Box::new(value) as Box<dyn Any + Send>)
            .map_err(|err| err.to_string())
    });
    let request = Arc::new(Request {
        slot: Mutex::new(Slot {
            job: Some(job),
            outcome: None,
            done: false,
        }),
        ready: Condvar::new(),
    });

    // Hand the JS thread its own strong reference; `dispatch_on_js_thread`
    // reclaims it with `Arc::from_raw`.
    let raw = Arc::into_raw(Arc::clone(&request)) as *mut c_void;
    // `napi_tsfn_nonblocking` (0): never block the worker if the queue is full.
    let status = unsafe { napi_call_threadsafe_function(handle, raw, 0) };
    if status != sys::Status::napi_ok {
        // The callback will never run, so take the reference back.
        drop(unsafe { Arc::from_raw(raw as *const Request) });
        return None;
    }

    let mut slot = request.slot.lock().ok()?;
    let deadline = Instant::now() + BRIDGE_CALL_TIMEOUT;
    while !slot.done {
        let remaining = deadline.checked_duration_since(Instant::now())?;
        let (guard, timeout) = request.ready.wait_timeout(slot, remaining).ok()?;
        slot = guard;
        if timeout.timed_out() && !slot.done {
            // The JS thread never answered. The job may still run later (it is
            // harmless, and keeps its own reference alive until then).
            return None;
        }
    }
    slot.outcome
        .take()?
        .ok()?
        .downcast::<R>()
        .ok()
        .map(|value| *value)
}