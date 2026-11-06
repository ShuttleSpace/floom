//! Web half of the camera plugin — camera capability, pure Rust via web-sys.
//!
//! On the web there is no C native half to compile and no JNI to drive: the
//! "native view" is a DOM `<video>` element fed by `getUserMedia`, and the
//! capability is reached directly from Rust through `web-sys`. This is the
//! Rust-where-feasible half of the platform pattern: the other platforms hand
//! their pointer to a native half, the browser *is* the native half.

use floom_native_ffi::CProviderError;
use floom_native_ffi::floom_native::{
    ClipSupport, InputModes, NativeCapabilities, NativeCompositionMode, NativeError, NativeSlot,
    NativeState, NativeViewKind, NativeViewProvider,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Document, HtmlElement, HtmlMediaElement, HtmlVideoElement, Window};

use crate::common::PLUGIN_NAME;

/// The web camera's live participants, keyed by `(generation, slot index)`.
pub struct WebCameraProvider {
    videos: RefCell<HashMap<(u32, u32), HtmlVideoElement>>,
}

impl WebCameraProvider {
    fn new() -> Self {
        Self {
            videos: RefCell::new(HashMap::new()),
        }
    }

    fn document(window: &Window) -> Result<Document, NativeError> {
        window
            .document()
            .ok_or_else(|| NativeError::Provider("no document on the web".into()))
    }
}

fn stream_video(video: &HtmlVideoElement) -> Result<(), NativeError> {
    let window = web_sys::window().ok_or_else(|| NativeError::Provider("no window".into()))?;
    let devices = window
        .navigator()
        .media_devices()
        .map_err(|_| NativeError::Provider("no MediaDevices in this browser".into()))?;

    let mut constraints = web_sys::MediaStreamConstraints::new();
    let _ = constraints.video(&true.into());
    let promise = devices
        .get_user_media_with_constraints(&constraints)
        .map_err(|e| NativeError::Provider(format!("getUserMedia refused: {e:?}").into()))?;

    let video = video.clone();
    spawn_local(async move {
        if let Ok(stream) = JsFuture::from(promise).await {
            if let Ok(stream) = stream.dyn_into::<web_sys::MediaStream>() {
                let media: &HtmlMediaElement = video.unchecked_ref();
                media.set_src_object(Some(&stream));
                let _ = video.play();
            }
        }
    });
    Ok(())
}

impl NativeViewProvider for WebCameraProvider {
    fn capabilities(&self) -> NativeCapabilities {
        NativeCapabilities {
            modes: std::borrow::Cow::Borrowed(&[NativeCompositionMode::Overlay]),
            snapshot: false,
            input: InputModes::PASSTHROUGH_ONLY,
            focusable: false,
            clip: ClipSupport::Platform,
        }
    }

    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        if !matches!(kind, NativeViewKind::Custom("camera")) {
            return Err(NativeError::Provider(
                "the web camera builds `Custom(\"camera\")` only".into(),
            ));
        }
        let window = web_sys::window().ok_or_else(|| NativeError::Provider("no window".into()))?;
        let document = Self::document(&window)?;
        let element: HtmlVideoElement = document
            .create_element("video")
            .map_err(|e| NativeError::Provider(format!("create video element: {e:?}").into()))?
            .dyn_into()
            .map_err(|_| NativeError::Provider("video is not an HtmlVideoElement".into()))?;

        stream_video(&element)?;
        let _ = document.body().map(|body| body.append_child(&element));

        self.videos
            .borrow_mut()
            .insert((slot.generation, slot.id.index()), element);
        Ok(())
    }

    fn destroy(&mut self, slot: NativeSlot) {
        let video = self
            .videos
            .borrow_mut()
            .remove(&(slot.generation, slot.id.index()));
        if let Some(video) = video {
            let media: Option<&HtmlMediaElement> = video.dyn_ref::<HtmlMediaElement>();
            if let Some(media) = media {
                let _ = media.pause();
            }
            video.remove();
        }
    }

    fn apply(&mut self, slot: NativeSlot, state: NativeState) {
        let Some(video) = self
            .videos
            .borrow()
            .get(&(slot.generation, slot.id.index()))
            .cloned()
        else {
            return;
        };
        let Some(element) = video.dyn_ref::<HtmlElement>() else {
            return;
        };
        let style = element.style();
        // The rect crosses in logical coordinates; CSS consumes CSS pixels,
        // which is the same space.
        let _ = style.set_property("position", "fixed");
        let _ = style.set_property("left", &format!("{}px", state.rect.x0));
        let _ = style.set_property("top", &format!("{}px", state.rect.y0));
        let _ = style.set_property(
            "width",
            &format!("{}px", (state.rect.x1 - state.rect.x0) as i64),
        );
        let _ = style.set_property(
            "height",
            &format!("{}px", (state.rect.y1 - state.rect.y0) as i64),
        );
        let _ = style.set_property(
            "visibility",
            if state.visible { "visible" } else { "hidden" },
        );
        let _ = style.set_property("z-index", &state.z.to_string());
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
}

/// Claims the plugin's platform state — the browser's MediaDevices, if present.
pub fn enable() -> Result<(), String> {
    let Some(window) = web_sys::window() else {
        return Err("no window on the web".to_string());
    };
    if window.navigator().media_devices().is_err() {
        return Err("no MediaDevices in this browser".to_string());
    }
    // A getUserMedia refusal later is a user-permission thing at request time,
    // not at capability time; the presence of the API is what counts.
    Ok(())
}

/// Releases whatever [`enable`] claimed — nothing, on the web.
pub fn disable() {}

/// The provider for this plugin.
///
/// `host_window` is unused on the web: the browser window is global, reached
/// through `web_sys::window()`.
///
/// # Safety
///
/// `host_window` must satisfy the same contract as the native halves.
pub unsafe fn provider(
    _host_window: *const c_void,
) -> Result<Box<dyn NativeViewProvider>, CProviderError> {
    Ok(Box::new(WebCameraProvider::new()))
}

/// Registers this plugin with the process-wide table.
pub fn register() {
    floom_platform::register_plugin(PLUGIN_NAME, enable);
}

/// The diagnostics the web half publishes.
pub mod diagnostics {
    /// How many participants the web camera can serve: unbounded, one DOM
    /// element each.
    pub fn live() -> i32 {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.query_selector_all("video").ok())
            .and_then(|l| l.length().try_into().ok())
            .unwrap_or(0)
    }
}
