//! Android half of the `webview` plugin — web view capability.
//!
//! Drives a `FloomWebView` (an `android.webkit.WebView` wrapped for Floom) through
//! JNI. The view is a native participant in the Floom scene: Floom controls its
//! geometry through the provider ABI, WebView renders the page.
//!
//! The Java half (`android/FloomWebView.java`) owns the Android view and the
//! WebView lifecycle; this crate only issues JNI calls to create, place, load and
//! evaluate it.

use std::borrow::Cow;
use std::ffi::c_void;

use floom_native_ffi::CProviderError;
use floom_native_ffi::floom_native::ClipSupport;
use floom_native_ffi::floom_native::NativeSlot;
use floom_native_ffi::floom_native::{
    NativeCapabilities, NativeCapture, NativeCompositionMode, NativeError, NativeSnapshot,
    NativeState, NativeViewKind, NativeViewProvider,
};
use floom_platform_android::jni as jni_helpers;

/// The Java `FloomWebView` class, as `ClassLoader.loadClass` expects it.
const CLASS: &str = "dev.floom.plugin.webview.FloomWebView";

/// A web-view provider: wraps a Java `FloomWebView` instance and drives it
/// through JNI.
///
/// The Java object is held as a global JNI reference so it outlives individual
/// JNI calls. The provider itself is created and dropped on the host thread, but
/// JNI calls are made on whichever thread `with_env` attaches — View operations
/// are all on the UI thread, and `with_env` attaches the current thread each time,
/// which is what the Android view hierarchy expects.
pub struct WebViewProvider {
    /// The `FloomWebView` instance, as a global reference.
    ///
    /// Stored with a `'static` lifetime via `unsafe` transmute — this is sound
    /// because JNI global references are not tied to any particular `Env`
    /// lifetime; they remain valid until `DeleteGlobalRef` is called, which
    /// happens when this `Global` is dropped.
    view: Option<jni::objects::Global<jni::objects::JObject<'static>>>,
}

impl WebViewProvider {
    pub fn new() -> Self {
        Self { view: None }
    }

    /// Create the Java `FloomWebView`, add it to the window's decor view, and
    /// start with a blank document.
    fn create_view(
        &mut self,
        env: &mut jni::Env<'_>,
        activity: jni::objects::JObject<'_>,
    ) -> Result<(), NativeError> {
        let class = env
            .find_class(jni::jni_str!(CLASS))
            .map_err(|e| NativeError::Provider(format!("find FloomWebView class: {e}").into()))?;

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
            .map_err(|e| NativeError::Provider(format!("new FloomWebView: {e}").into()))?;

        let view_global = env
            .new_global_ref(&view)
            .map_err(|e| NativeError::Provider(format!("new global ref: {e}").into()))?;
        // SAFETY: JNI global references are not tied to any Env lifetime;
        // they remain valid until `DeleteGlobalRef` is called (when the Global
        // is dropped). The phantom lifetime in JObject<'_> is therefore safe to
        // extend to 'static.
        let view_global: jni::objects::Global<jni::objects::JObject<'static>> =
            unsafe { std::mem::transmute(view_global) };
        self.view = Some(view_global);

        // Add the view to the window's decor view, filling it.
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

        // Start with a blank document so the web view is never an empty view.
        self.load(env, "about:blank")
            .map_err(|e| NativeError::Provider(e.to_string()))?;

        Ok(())
    }

    /// Call `loadUrl(String)` on the Java web view.
    fn load(&mut self, env: &mut jni::Env<'_>, url: &str) -> Result<(), NativeError> {
        let view = self
            .view
            .as_ref()
            .ok_or_else(|| NativeError::Provider("no web view".into()))?;
        env.call_object_method(
            view,
            jni::jni_str!("loadUrl"),
            jni::jni_sig!("(Ljava/lang/String;)V"),
            &[jni::JValue::String(&jni::jni_str!(url))],
        )
        .map_err(|e| NativeError::Provider(format!("call loadUrl: {e}").into()))?;
        Ok(())
    }

    /// Call `evaluateJs(String)` on the Java web view.
    fn evaluate(&mut self, env: &mut jni::Env<'_>, js: &str) -> Result<(), NativeError> {
        let view = self
            .view
            .as_ref()
            .ok_or_else(|| NativeError::Provider("no web view".into()))?;
        env.call_object_method(
            view,
            jni::jni_str!("evaluateJs"),
            jni::jni_sig!("(Ljava/lang/String;)V"),
            &[jni::JValue::String(&jni::jni_str!(js))],
        )
        .map_err(|e| NativeError::Provider(format!("call evaluateJs: {e}").into()))?;
        Ok(())
    }

    /// Call `setPosition(float,float,float,float)` on the Java web view.
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
            .ok_or_else(|| NativeError::Provider("no web view".into()))?;
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

    /// Call `setVisible(boolean)` on the Java web view.
    fn set_visible(&mut self, env: &mut jni::Env<'_>, visible: bool) -> Result<(), NativeError> {
        let view = self
            .view
            .as_ref()
            .ok_or_else(|| NativeError::Provider("no web view".into()))?;
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

impl NativeViewProvider for WebViewProvider {
    fn capabilities(&self) -> NativeCapabilities {
        NativeCapabilities {
            modes: Cow::Borrowed(&[NativeCompositionMode::Overlay]),
            snapshot: false,
            input: floom_native_ffi::floom_native::InputModes::ALL,
            focusable: true,
            clip: ClipSupport::Provider,
        }
    }

    fn create(&mut self, _slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        if kind != NativeViewKind::Web {
            return Err(NativeError::Provider(
                format!("this plugin builds web views only, not {kind:?}").into(),
            ));
        }

        jni_helpers::with_env(|env| -> Result<(), String> {
            let activity = jni_helpers::activity(env).map_err(|e| format!("get activity: {e}"))?;
            self.create_view(env, activity).map_err(|e| e.to_string())?;
            Ok(())
        })
        .map_err(|e| NativeError::Provider(e.into()))?;

        Ok(())
    }

    fn destroy(&mut self, _slot: NativeSlot) {
        let _ = jni_helpers::with_env(|env| -> Result<(), String> {
            self.view = None;
            Ok(())
        });
    }

    fn apply(&mut self, _slot: NativeSlot, state: NativeState) {
        let _ = jni_helpers::with_env(|env| -> Result<(), String> {
            let rect = state.rect;
            self.set_position(env, rect.x0, rect.y0, rect.x1 - rect.x0, rect.y1 - rect.y0)
                .map_err(|e| e.to_string())?;
            self.set_visible(env, state.visible)
                .map_err(|e| e.to_string())?;
            Ok(())
        });
    }

    fn focus(&mut self, _slot: NativeSlot) {}

    fn blur(&mut self, _slot: NativeSlot) {}

    fn snapshot(
        &mut self,
        _slot: NativeSlot,
    ) -> Option<floom_native_ffi::floom_native::NativeSnapshot> {
        None
    }

    fn restore(&mut self, _slot: NativeSlot) {}

    fn take_captures(&mut self) -> Vec<floom_native_ffi::floom_native::NativeCapture> {
        Vec::new()
    }
}

/// The plugin's version.
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Claims the plugin's platform state.
///
/// On Android there is no C native half to initialise; the Java half is created
/// lazily on first [`create`](NativeViewProvider::create).
pub fn enable() -> Result<(), String> {
    Ok(())
}

/// Releases whatever [`enable`] claimed.
pub fn disable() {}

/// The plugin's capability id. The web-view capability claims no platform id yet.
pub fn capability() -> i32 {
    0
}

/// Send a JSON message from Rust to the web view's JS.
pub fn post_message(slot: NativeSlot, json: &str) -> Result<(), String> {
    Err(format!(
        "post_message is not wired for the android web view yet: {json}"
    ))
}

/// Register the Rust callback invoked when JS calls
/// `window.floomNative.postMessage(json)`. No-op until a web-view callback
/// channel exists; kept for API parity with macOS.
pub fn set_message_callback(
    _cb: Option<
        unsafe extern "C" fn(floom_native_ffi::abi::FloomNativeSlot, *const std::ffi::c_char),
    >,
) {
}

/// Loads `url` into the web participant bound to `slot`.
pub fn load_url(slot: NativeSlot, url: &str) -> Result<(), String> {
    Err(format!(
        "load_url is not wired for the android web view yet: {url}"
    ))
}

/// The provider for this plugin.
///
/// `host_window` is the platform's opaque window handle: an `ANativeWindow *` on
/// Android. It is not used by the Java half — the view is added to the Activity's
/// window through JNI — but the signature is the same as on macOS so the host code
/// is identical.
///
/// # Safety
///
/// `host_window` must be a live handle for as long as the returned provider is
/// driven.
pub unsafe fn provider(
    host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    let _ = host_window;
    Ok(Box::new(WebViewProvider::new()))
}

/// The provider, boxed, for a host that wants the trait object.
///
/// # Safety
///
/// Same contract as [`provider`]: `host_window` must be a live handle for as
/// long as the returned provider is driven.
pub unsafe fn boxed_provider(
    host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    unsafe { provider(host_window) }
}

/// The diagnostics the native half publishes.
pub mod diagnostics {
    use super::*;

    /// How many participants the native half created.
    pub fn created() -> i32 {
        0
    }

    /// How many create calls were refused for a kind this plugin cannot build.
    pub fn refused() -> i32 {
        0
    }

    /// How many participants are alive right now.
    pub fn live() -> i32 {
        0
    }

    /// Whether the participant in `slot` holds a real web view.
    pub fn slot_is_web(_slot: NativeSlot) -> bool {
        false
    }

    /// Whether the participant in `slot` is really in the window's content view.
    pub fn slot_is_attached(_slot: NativeSlot) -> bool {
        false
    }
}
