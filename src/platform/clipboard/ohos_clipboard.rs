//! OpenHarmony (HarmonyOS) N-API clipboard bridge.
//!
//! OpenHarmony exposes the system pasteboard **only** through ArkTS
//! (`@kit.BasicServicesKit`'s `pasteboard` module). There is no native Rust
//! clipboard API, so we bridge through N-API — the same role the Android
//! backend's JNI bridge plays for `android.content.ClipboardManager`.
//!
//! The shared handle and the registration entry point live in
//! [`crate::platform::ohos_bridge`] (ArkTS registers one combined object);
//! this module only performs the clipboard-specific calls:
//!
//! 1. At app startup the ArkTS side calls
//!    `register_platform_bridge({ getText, setText, openUrl, openFile, saveFile })`.
//! 2. When Floem needs clipboard text, [`get_contents`] / [`set_contents`]
//!    call back into that object's `getText` / `setText` via N-API.
//!
//! # Status
//! Compiled only for `target_env = "ohos"`. **UNVERIFIED** against a real OHOS
//! SDK build (see `docs/ohos.md`).

#![cfg(target_env = "ohos")]

use napi::bindgen_prelude::JsFunction;
use napi::{JsString, JsUnknown, ValueType};

use crate::platform::ohos_bridge;

/// Read the system clipboard as text, via the registered ArkTS bridge.
pub(crate) fn get_contents() -> Result<String, crate::platform::ClipboardError> {
    // `call_bridge` returns `Option<Option<String>>`: the outer `Option` is
    // `None` when the ArkTS bridge isn't registered; the inner `Option` is
    // `None` when the board holds `null` (empty).
    let text: Option<Option<String>> = ohos_bridge::call_bridge(|_env, obj| {
        let func: JsFunction = obj.get_named_property("getText")?;
        // ArkTS returns `string | null`. `call_without_args` yields a `JsUnknown`
        // that we downcast; a `null` result means the board is empty.
        let result: JsUnknown = func.call_without_args(None)?;
        if result.get_type()? == ValueType::Null {
            Ok(None)
        } else {
            let s: JsString = result.try_into()?;
            Ok(Some(s.into_utf8()?.into_owned()?))
        }
    });
    match text {
        Some(Some(s)) => Ok(s),
        Some(None) => Ok(String::new()),
        None => Err(crate::platform::ClipboardError::NotAvailable),
    }
}

/// Write `text` to the system clipboard, via the registered ArkTS bridge.
pub(crate) fn set_contents(text: &str) -> Result<(), crate::platform::ClipboardError> {
    // The closure runs on the JS thread, so it must own its captures.
    let text = text.to_owned();
    let done: Option<()> = ohos_bridge::call_bridge(move |env, obj| {
        let func: JsFunction = obj.get_named_property("setText")?;
        let arg = env.create_string(&text)?;
        func.call(None, &[arg.into_unknown()])?;
        Ok(())
    });
    done.ok_or(crate::platform::ClipboardError::NotAvailable)
}
