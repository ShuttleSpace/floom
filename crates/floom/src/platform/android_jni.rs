#![cfg(target_os = "android")]

//! Shared JNI handles for the Android backend.
//!
//! Floom's Android backend reaches system services (clipboard, content
//! resolver, intents, …) through JNI from the app's `Activity`. The `JavaVM`
//! and the JNI *global* reference to the `Activity` are captured once from the
//! `AndroidApp` at startup and stored here, so every Android-only subsystem
//! (clipboard, external-URL opening, file dialogs) can attach a thread-local
//! `JNIEnv` without re-passing the handles around.

use jni::{objects::JObject, sys::jobject, JavaVM};
use parking_lot::Mutex;

static ANDROID_JNI: Mutex<Option<AndroidJni>> = Mutex::new(None);

pub(crate) struct AndroidJni {
    vm: JavaVM,
    /// JNI *global* reference to the `Activity` (a `Context`). Must NOT be
    /// deleted on drop — android-activity owns it for the app lifetime.
    activity: JObject<'static>,
}

impl AndroidJni {
    /// Capture the JNI handles.
    ///
    /// # Safety
    /// `vm_ptr` must be a valid `JavaVM*` (from `AndroidApp::vm_as_ptr`) and
    /// `activity_ptr` a valid JNI global reference to the `Activity` (from
    /// `AndroidApp::activity_as_ptr`). Must be called exactly once at startup,
    /// before any Android subsystem touches JNI.
    pub(crate) unsafe fn init(vm_ptr: *mut std::ffi::c_void, activity_ptr: jobject) {
        let vm = match unsafe { JavaVM::from_raw(vm_ptr as *mut jni::sys::JavaVM) } {
            Ok(vm) => vm,
            Err(e) => {
                // No logging dependency is guaranteed here; fail silently.
                let _ = e;
                return;
            }
        };
        let activity = unsafe { JObject::from_raw(activity_ptr as jobject) };
        *ANDROID_JNI.lock() = Some(Self { vm, activity });
    }

    /// Attach the current thread to the JVM and hand the caller an env plus the
    /// `Activity` object. Returns `None` if the JNI bridge was never
    /// initialized (e.g. built without `Application::new_android`).
    pub(crate) fn with_env<R>(
        f: impl FnOnce(&mut jni::JNIEnv, &JObject<'static>) -> R,
    ) -> Option<R> {
        let guard = ANDROID_JNI.lock();
        let jni = guard.as_ref()?;
        let activity = &jni.activity;
        let mut env_guard = jni.vm.attach_current_thread().ok()?;
        // `AttachGuard` derefs mutably to `JNIEnv`; reborrow for the callback.
        let env: &mut jni::JNIEnv = &mut *env_guard;
        let result = f(env, activity);
        // A failed JNI call leaves the Java exception *pending*. Best-effort
        // callers swallow the `Err` and move on, but a pending exception is a
        // landmine: the next JNI call on this thread makes ART abort the whole
        // process ("No pending exception expected: …"). Concretely, an app
        // without the optional `FloomAccessibilityView` Java class aborted on
        // its first hardware key event, because the ClassNotFoundException
        // thrown during a11y injection at startup was never cleared. Every
        // `with_env` closure therefore gets a clean exception state on exit.
        if env.exception_check().unwrap_or(false) {
            env.exception_clear();
        }
        Some(result)
    }
}
