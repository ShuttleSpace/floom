// SPDX-License-Identifier: MIT
//! Minimal Android JNI bridge for mobile capability packages.
//!
//! Stores the `JavaVM` and the activity `jobject` globally so capability
//! packages (vibration, camera, location, …) can reach the JVM from any thread.
//! The concrete `floom-platform-android` backend is expected to call
//! [`set_java_vm`] and [`set_activity`] during application startup — before any
//! capability is invoked — to hand over the handles it already obtained from
//! `android-activity`.
//!
//! This is a deliberately small subset of `gpui-mobile`'s `android/jni.rs`,
//! ported under its Apache-2.0 option: only the helpers the worked
//! [`crate::packages::vibration`] example needs.
#![allow(unsafe_code)]

use std::ffi::c_void;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use jni::JavaVM;
use jni::objects::{JObject, JValue};
use jni::sys::{JavaVM as RawJavaVM, jobject};

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();

/// The activity handle, as handed over by the platform.
///
/// `AtomicPtr` rather than `OnceLock<jobject>` because a raw pointer is neither
/// `Send` nor `Sync`, so `OnceLock<jobject>` cannot be a `static` at all. The
/// atomic also gives "first writer wins" for free via `compare_exchange`, which
/// is the semantics the setter documents.
static ACTIVITY: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

/// Hand over the `JavaVM` pointer obtained at Android startup.
///
/// Takes a bare `*mut c_void` because that is what
/// `android_activity::AndroidApp::vm_as_ptr` hands out: the platform backend
/// should not need a `jni` dependency (and the matching) just to forward it.
/// The cast is to `jni::sys::JavaVM`, i.e. the JNI `JavaVM*`, which is what the
/// pointer already is.
///
/// Safe to call more than once; the first call wins. A null pointer is ignored.
pub fn set_java_vm(vm: *mut c_void) {
    if vm.is_null() {
        return;
    }
    let vm = vm as *mut RawJavaVM;
    let _ = JAVA_VM.get_or_init(|| unsafe { JavaVM::from_raw(vm) });
}

/// Hand over the activity `jobject` obtained at Android startup.
///
/// Also takes `*mut c_void` for the same reason: it comes from
/// `AndroidApp::activity_as_ptr`. The handle is the framework's **global**
/// reference and must outlive the app, so it is *never* released here — see
/// [`activity`] for why that matters when reading it back. A null pointer is
/// ignored, and a later non-null call is ignored too (first writer wins).
pub fn set_activity(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let _ = ACTIVITY.compare_exchange(
        std::ptr::null_mut(),
        ptr,
        Ordering::AcqRel,
        Ordering::Acquire,
    );
}

/// Whether the handover has happened. Useful for a backend (or an example) that
/// wants to report the bridge state rather than discovering it from a failed
/// capability call.
pub fn is_initialised() -> bool {
    JAVA_VM.get().is_some() && !ACTIVITY.load(Ordering::Acquire).is_null()
}

/// Run a closure with an `Env` attached to the current thread.
///
/// In jni 0.22 the `attach_current_thread` API is closure-based; the thread is
/// auto-detached when the closure returns if it was not already attached.
pub fn with_env<T>(f: impl FnOnce(&mut jni::Env) -> Result<T, String>) -> Result<T, String> {
    let vm = JAVA_VM
        .get()
        .ok_or_else(|| "JavaVM not initialised; call set_java_vm at startup".to_string())?;
    let mut result: Option<Result<T, String>> = None;
    vm.attach_current_thread(|env: &mut jni::Env| -> Result<(), jni::errors::Error> {
        result = Some(f(env));
        Ok(())
    })
    .map_err(|e| e.to_string())?;
    result.unwrap()
}

/// Get the Activity as a [`JObject`] bound to the given env's local frame.
///
/// # Why this is not just a cast
///
/// The stored handle is a JNI **global** reference owned by the Android
/// framework. `JObject` is an *owning* wrapper: dropping one calls
/// `DeleteLocalRef`, and deleting a global reference that way is undefined
/// behaviour. So the raw handle is wrapped only to borrow it, and the wrapper
/// is deliberately leaked (never dropped) while a genuine **local** reference —
/// which this function's caller owns and which is released with the frame — is
/// minted from it.
///
/// This mirrors what `android-activity` itself does when it needs a class
/// reference out of its `GlobalRef`.
pub fn activity<'local>(env: &mut jni::Env<'local>) -> Result<JObject<'local>, String> {
    let raw = ACTIVITY.load(Ordering::Acquire);
    if raw.is_null() {
        return Err("Activity not initialised; call set_activity at startup".to_string());
    }
    // Safety: `raw` is a valid global reference for the app's lifetime, which
    // outlives this frame. The wrapper is leaked immediately below, so nothing
    // ever treats the global reference as a local one.
    let borrowed = unsafe { JObject::from_raw(env, raw as jobject) };
    let local = env.new_local_ref(&borrowed).e()?;
    std::mem::forget(borrowed);
    Ok(local)
}

/// Resolve a class from the app's own dex by its JNI name
/// (`dev/floom/plugin/battery/BatteryPlugin`).
///
/// # Why not `FindClass`
///
/// `FindClass` resolves against the class loader of the current JNI frame. A
/// thread that reached the JVM through `AttachCurrentThread` has no Java frame,
/// so it falls back to the system loader, which cannot see the application's
/// classes. A capability called from a worker thread would then fail to find a
/// class that is in the APK. The activity is already in hand, and the loader
/// that loaded it is the one that can see every plugin class, so the lookup
/// goes through that.
pub fn app_class<'local>(
    env: &mut jni::Env<'local>,
    name: &str,
) -> Result<jni::objects::JClass<'local>, String> {
    let activity = activity(env)?;
    // `Context.getClassLoader()` (not `activity.getClass().getClassLoader()`):
    // the latter asks `java.lang.Class`, which reports the loader that loaded
    // the framework's `NativeActivity` — the boot loader, which cannot see the
    // app's dex. `Context` is overridden to return the app's own loader.
    let loader = env
        .call_method(
            &activity,
            jni::jni_str!("getClassLoader"),
            jni::jni_sig!("()Ljava/lang/ClassLoader;"),
            &[],
        )
        .e()?
        .l()
        .e()?;
    if loader.is_null() {
        return Err(format!("{name}: the activity has no class loader"));
    }
    let name = env.new_string(name).map(JObject::from).e()?;
    env.call_method(
        &loader,
        jni::jni_str!("loadClass"),
        jni::jni_sig!("(Ljava/lang/String;)Ljava/lang/Class;"),
        &[JValue::Object(&name)],
    )
    .e()?
    .l()
    .and_then(|class| env.cast_local::<jni::objects::JClass>(class))
    .e()
}

/// Convenience: `Result::err()` mapped to `String`, for the jni 0.22 API which
/// returns rich error types.
pub trait JniExt {
    type Ok;
    fn e(self) -> Result<Self::Ok, String>;
}

impl<T, E: std::fmt::Display> JniExt for Result<T, E> {
    type Ok = T;
    fn e(self) -> Result<T, String> {
        self.map_err(|e| e.to_string())
    }
}

/// Build a `JValue::Object` from a `JObject` (helper for call signatures).
///
/// `JValue::Object` borrows, so the lifetimes have to be threaded by hand: the
/// object must outlive the `JValue`, and `JValue`'s own parameter is the
/// borrow's lifetime.
#[inline]
pub fn obj<'a>(value: &'a JObject<'a>) -> JValue<'a> {
    JValue::Object(value)
}

/// A null `jobject` cast helper.
#[allow(dead_code)]
pub const fn null_jobject() -> jobject {
    core::ptr::null_mut::<c_void>() as jobject
}
