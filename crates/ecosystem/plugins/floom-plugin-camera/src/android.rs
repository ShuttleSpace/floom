//! Android half of the camera plugin — camera capability.
//!
//! Drives a `FloomCameraView` (TextureView + Camera2) through JNI. The view
//! is a native participant in the Floom scene: Floom controls its geometry
//! through the provider ABI, Camera2 renders live frames into it.
//!
//! The Java half (`native/android/FloomCameraView.java`) owns the Android
//! View and the Camera2 lifecycle; this crate only issues JNI calls to open,
//! place, hide and close it.

use std::borrow::Cow;
use std::ffi::c_void;

use floom_native_ffi::CProviderError;
use floom_native_ffi::floom_native::ClipSupport;
use floom_native_ffi::floom_native::{
    NativeCapabilities, NativeCompositionMode, NativeError, NativeState, NativeViewKind,
    NativeViewProvider,
};
use floom_platform_android::jni as jni_helpers;

use crate::common::PLUGIN_NAME;

/// A camera provider: wraps a Java `FloomCameraView` instance and drives it
/// through JNI.
///
/// The Java object is held as a global JNI reference so it outlives individual
/// JNI calls. The provider itself is created and dropped on the host thread,
/// but JNI calls are made on whichever thread `with_env` attaches — Camera2
/// and View operations are all on the UI thread, and `with_env` attaches the
/// current thread each time, which is what the Android view hierarchy expects.
pub struct CameraProvider {
    /// The `FloomCameraView` instance, as a global reference.
    ///
    /// Stored with a `'static` lifetime via `unsafe` transmute — this is sound
    /// because JNI global references are not tied to any particular `Env`
    /// lifetime; they remain valid until `DeleteGlobalRef` is called, which
    /// happens when this `Global` is dropped.
    view: Option<jni::objects::Global<jni::objects::JObject<'static>>>,
}

impl CameraProvider {
    pub fn new() -> Self {
        Self { view: None }
    }

    /// Create the Java `FloomCameraView`, add it to the window's decor view,
    /// and start the Camera2 preview.
    fn create_view(
        &mut self,
        env: &mut jni::Env<'_>,
        activity: jni::objects::JObject<'_>,
    ) -> Result<(), NativeError> {
        let class = env
            .find_class(jni::jni_str!("dev/floom/plugin/camera/FloomCameraView"))
            .map_err(|e| {
                NativeError::Provider(format!("find FloomCameraView class: {e}").into())
            })?;

        let context = env
            .call_method(
                &activity,
                jni::jni_str!("getContext"),
                jni::jni_sig!("()Landroid/content/Context;"),
                &[],
            )
            .map_err(|e| NativeError::Provider(format!("get Context: {e}").into()))?
            .l()
            .map_err(|e| NativeError::Provider(format!("cast: {e}").into()))?;

        let view = env
            .new_object(
                class,
                jni::jni_sig!("(Landroid/content/Context;Landroid/app/Activity;)V"),
                &[
                    jni::JValue::Object(&context),
                    jni::JValue::Object(&activity),
                ],
            )
            .map_err(|e| NativeError::Provider(format!("new FloomCameraView: {e}").into()))?;

        let view_global = env
            .new_global_ref(&view)
            .map_err(|e| NativeError::Provider(format!("new global ref: {e}").into()))?;
        // SAFETY: JNI global references are not tied to any Env lifetime;
        // they remain valid until DeleteGlobalRef is called (when the
        // Global is dropped). The phantom lifetime in JObject<'_> is
        // therefore safe to extend to 'static.
        let view_global: jni::objects::Global<jni::objects::JObject<'static>> =
            unsafe { std::mem::transmute(view_global) };
        self.view = Some(view_global);

        // Add the view to the window's decor view.
        let window = env
            .call_method(
                &activity,
                jni::jni_str!("getWindow"),
                jni::jni_sig!("()Landroid/view/Window;"),
                &[],
            )
            .map_err(|e| NativeError::Provider(format!("get Window: {e}").into()))?
            .l()
            .map_err(|e| NativeError::Provider(format!("cast: {e}").into()))?;

        let decor_view = env
            .call_method(
                &window,
                jni::jni_str!("getDecorView"),
                jni::jni_sig!("()Landroid/view/View;"),
                &[],
            )
            .map_err(|e| NativeError::Provider(format!("get DecorView: {e}").into()))?
            .l()
            .map_err(|e| NativeError::Provider(format!("cast: {e}").into()))?;

        let layout_params_class = env
            .find_class(jni::jni_str!("android/view/ViewGroup$LayoutParams"))
            .map_err(|e| NativeError::Provider(format!("find LayoutParams: {e}").into()))?;
        let layout_params = env
            .new_object(
                layout_params_class,
                jni::jni_sig!("(II)V"),
                &[
                    jni::JValue::Int(-1), // MATCH_PARENT
                    jni::JValue::Int(-1),
                ],
            )
            .map_err(|e| NativeError::Provider(format!("new LayoutParams: {e}").into()))?;

        env.call_method(
            &decor_view,
            jni::jni_str!("addView"),
            jni::jni_sig!("(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"),
            &[
                jni::JValue::Object(&view),
                jni::JValue::Object(&layout_params),
            ],
        )
        .map_err(|e| NativeError::Provider(format!("addView: {e}").into()))?;

        Ok(())
    }

    /// Call `openCamera()` on the Java view.
    fn open_camera(&mut self, env: &mut jni::Env<'_>) -> Result<(), NativeError> {
        let view = self
            .view
            .as_ref()
            .ok_or_else(|| NativeError::Provider("no camera view".into()))?;
        env.call_method(view, jni::jni_str!("openCamera"), jni::jni_sig!("()V"), &[])
            .map_err(|e| NativeError::Provider(format!("call openCamera: {e}").into()))?;
        Ok(())
    }

    /// Call `closeCamera()` on the Java view.
    fn close_camera(&mut self, env: &mut jni::Env<'_>) {
        if let Some(view) = &self.view {
            let _ = env.call_method(
                view,
                jni::jni_str!("closeCamera"),
                jni::jni_sig!("()V"),
                &[],
            );
        }
    }

    /// Call `setPosition(x, y, width, height)` on the Java view.
    fn set_position(
        &mut self,
        env: &mut jni::Env<'_>,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<(), NativeError> {
        let view = self
            .view
            .as_ref()
            .ok_or_else(|| NativeError::Provider("no camera view".into()))?;
        env.call_method(
            view,
            jni::jni_str!("setPosition"),
            jni::jni_sig!("(FFFF)V"),
            &[
                jni::JValue::Float(x as f32),
                jni::JValue::Float(y as f32),
                jni::JValue::Float(width as f32),
                jni::JValue::Float(height as f32),
            ],
        )
        .map_err(|e| NativeError::Provider(format!("call setPosition: {e}").into()))?;
        Ok(())
    }

    /// Call `setVisible(boolean)` on the Java view.
    fn set_visible(&mut self, env: &mut jni::Env<'_>, visible: bool) -> Result<(), NativeError> {
        let view = self
            .view
            .as_ref()
            .ok_or_else(|| NativeError::Provider("no camera view".into()))?;
        env.call_method(
            view,
            jni::jni_str!("setVisible"),
            jni::jni_sig!("(Z)V"),
            &[jni::JValue::Bool(visible)],
        )
        .map_err(|e| NativeError::Provider(format!("call setVisible: {e}").into()))?;
        Ok(())
    }
}

impl NativeViewProvider for CameraProvider {
    fn capabilities(&self) -> NativeCapabilities {
        NativeCapabilities {
            modes: Cow::Borrowed(&[NativeCompositionMode::Overlay]),
            snapshot: false,
            input: floom_native_ffi::floom_native::InputModes::ALL,
            focusable: false,
            clip: ClipSupport::Platform,
        }
    }

    fn create(
        &mut self,
        _slot: floom_native_ffi::floom_native::NativeSlot,
        kind: NativeViewKind,
    ) -> Result<(), NativeError> {
        if kind != NativeViewKind::Custom("camera") {
            return Err(NativeError::Provider(
                format!("this plugin builds camera views only, not {kind:?}").into(),
            ));
        }

        jni_helpers::with_env(|env| -> Result<(), String> {
            let activity = jni_helpers::activity(env)
                .map_err(|e| "get activity: ".to_string() + &e.to_string())?;
            self.create_view(env, activity).map_err(|e| e.to_string())?;
            self.open_camera(env).map_err(|e| e.to_string())
        })
        .map_err(|e| NativeError::Provider(e.into()))?;

        Ok(())
    }

    fn destroy(&mut self, _slot: floom_native_ffi::floom_native::NativeSlot) {
        let _ = jni_helpers::with_env(|env| -> Result<(), String> {
            self.close_camera(env);
            Ok(())
        });
        self.view = None;
    }

    fn apply(&mut self, _slot: floom_native_ffi::floom_native::NativeSlot, state: NativeState) {
        let _ = jni_helpers::with_env(|env| -> Result<(), String> {
            let rect = state.rect;
            self.set_position(env, rect.x0, rect.y0, rect.x1 - rect.x0, rect.y1 - rect.y0)
                .map_err(|e| e.to_string())?;
            self.set_visible(env, state.visible)
                .map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    fn focus(&mut self, _slot: floom_native_ffi::floom_native::NativeSlot) {
        // Camera view does not take focus.
    }

    fn blur(&mut self, _slot: floom_native_ffi::floom_native::NativeSlot) {
        // Nothing to do.
    }

    fn snapshot(
        &mut self,
        _slot: floom_native_ffi::floom_native::NativeSlot,
    ) -> Option<floom_native_ffi::floom_native::NativeSnapshot> {
        None
    }

    fn restore(&mut self, _slot: floom_native_ffi::floom_native::NativeSlot) {
        // Nothing was frozen, so there is nothing to restore.
    }
}

/// The plugin's version.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Claims the plugin's platform state.
///
/// On Android there is no C native half to initialise: the Java half is
/// created lazily on first [`create`](NativeViewProvider::create).
pub fn enable() -> Result<(), String> {
    Ok(())
}

/// Releases whatever [`enable`] claimed.
pub fn disable() {}

/// Registers this plugin with the process-wide table.
pub fn register() {
    floom_platform::register_plugin(PLUGIN_NAME, enable);
}

/// The plugin's capability id.
pub fn capability() -> i32 {
    0
}

/// Send a JSON message from Rust to the web view's JS.
///
/// Not supported on Android yet: the camera provider does not host a web view.
pub fn post_message(
    _slot: floom_native_ffi::floom_native::NativeSlot,
    _json: &str,
) -> Result<(), String> {
    Err("the camera provider does not host a web view".to_string())
}

/// Register the Rust callback invoked when JS calls
/// `window.floomNative.postMessage(json)`. No-op on Android until a web view
/// provider exists; kept for API parity with macOS.
pub fn set_message_callback(
    _cb: Option<
        unsafe extern "C" fn(floom_native_ffi::abi::FloomNativeSlot, *const std::ffi::c_char),
    >,
) {
}

/// Loads `url` into the web participant bound to `slot`.
///
/// Not supported on Android: the camera provider does not load URLs.
pub fn load_url(
    _slot: floom_native_ffi::floom_native::NativeSlot,
    _url: &str,
) -> Result<(), String> {
    Err("the camera provider does not load URLs".to_string())
}

/// The provider for this plugin.
///
/// `host_window` is the platform's opaque window handle: an `ANativeWindow *`
/// on Android. It is not used by the Java half — the view is added to the
/// Activity's window through JNI — but the signature is the same as on
/// macOS so the host code is identical.
///
/// # Safety
///
/// `host_window` must be a live handle for as long as the returned provider
/// is driven.
pub unsafe fn provider(
    host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    let _ = host_window;
    Ok(Box::new(CameraProvider::new()))
}

/// The provider, boxed, for a host that wants the trait object.
///
/// # Safety
///
/// Same contract as [`provider`]: `host_window` must be a live handle for
/// as long as the returned provider is driven.
pub unsafe fn boxed_provider(
    host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    unsafe { provider(host_window) }
}

/// The diagnostics the native half publishes.
///
/// Plain numbers, so a host can check what the native side did without
/// knowing anything about Android. The counts are cumulative since the
/// process started.
pub mod diagnostics {
    use floom_native_ffi::floom_native::NativeSlot;

    /// How many participants the native half created.
    pub fn created() -> i32 {
        0 // The camera provider does not expose a creation counter yet.
    }

    /// How many `create` calls were refused for a kind this plugin cannot
    /// build.
    pub fn refused() -> i32 {
        0 // Not tracked yet.
    }

    /// How many participants are alive right now.
    pub fn live() -> i32 {
        0 // Not tracked yet.
    }

    /// Whether the participant in `slot` holds a real `WKWebView`.
    pub fn slot_is_web(_slot: NativeSlot) -> bool {
        false // Camera views are not web views.
    }

    /// Whether the participant in `slot` is really in the window's content
    /// view.
    pub fn slot_is_attached(_slot: NativeSlot) -> bool {
        false // Not tracked yet.
    }
}
