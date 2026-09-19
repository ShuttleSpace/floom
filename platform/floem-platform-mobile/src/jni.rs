//! Minimal Android JNI bridge for mobile capability packages.
//!
//! Stores the `JavaVM` and the activity `jobject` globally so capability
//! packages (vibration, camera, location, …) can reach the JVM from any thread.
//! The concrete `floem-platform-android` backend is expected to call
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

use jni::JavaVM;
use jni::objects::{JObject, JValue};
use jni::sys::{JavaVM as RawJavaVM, jobject};

static JAVA_VM: OnceLock<JavaVM> = OnceLock::new();
static ACTIVITY: OnceLock<jobject> = OnceLock::new();

/// Hand over the `JavaVM` pointer obtained at Android startup.
///
/// Safe to call more than once; the first call wins. A null pointer is ignored.
pub fn set_java_vm(vm: *mut RawJavaVM) {
    if vm.is_null() {
        return;
    }
    let _ = JAVA_VM.get_or_init(|| unsafe { JavaVM::from_raw(vm) });
}

/// Hand over the activity `jobject` obtained at Android startup.
///
/// The object is a global reference owned by the platform backend and valid for
/// the lifetime of the app. A null pointer is ignored.
pub fn set_activity(ptr: jobject) {
    if ptr.is_null() {
        return;
    }
    let _ = ACTIVITY.get_or_init(|| ptr);
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
pub fn activity<'local>(env: &jni::Env<'local>) -> Result<JObject<'local>, String> {
    let ptr = *ACTIVITY
        .get()
        .ok_or_else(|| "Activity not initialised; call set_activity at startup".to_string())?;
    if ptr.is_null() {
        return Err("Activity is null".to_string());
    }
    Ok(unsafe { JObject::from_raw(ptr as jobject) })
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
#[inline]
pub fn obj(value: &JObject) -> JValue {
    JValue::Object(value)
}

/// A null `jobject` cast helper.
#[allow(dead_code)]
pub const fn null_jobject() -> jobject {
    core::ptr::null_mut::<c_void>() as jobject
}
