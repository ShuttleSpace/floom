//! Open external URLs / apps on OpenHarmony via an ArkTS `startAbility` Want.
//!
//! OpenHarmony has no `open`/`xdg-open` equivalent; opening a URL means asking
//! the ArkTS `AbilityContext` to `startAbility` with a `Want` that has
//! `action: "ohos.want.action.viewData"` and the URL as `uri`. This is
//! fire-and-forget (no result needed), so it is wired through the shared N-API
//! bridge in [`crate::platform::ohos_bridge`] — the same role Android's
//! `ACTION_VIEW` intent plays (`android_external.rs`).
//!
//! # Status
//! Compiled only for `target_env = "ohos"`. **UNVERIFIED** against a real OHOS
//! SDK build (see `docs/ohos.md`).

#![cfg(target_env = "ohos")]

use napi::bindgen_prelude::JsFunction;

use crate::platform::ohos_bridge;

/// Open `url` in the default external handler (browser for `http(s)://`,
/// appropriate app for other schemes).
///
/// Returns an error string on failure (e.g. the bridge was not registered via
/// `register_platform_bridge`, or the ArkTS `startAbility` call failed).
pub(crate) fn open_url(url: &str) -> Result<(), String> {
    // The closure runs on the JS thread, so it must own its captures.
    let url = url.to_owned();
    let opened: Option<()> = ohos_bridge::call_bridge(move |env, obj| {
        let func: JsFunction = obj.get_named_property("openUrl")?;
        let arg = env.create_string(&url)?;
        // `call<V>(this: Option<&JsObject>, args: &[V])`: single arg wrapped as
        // `JsUnknown` (which is `NapiRaw`). The `?` propagates `napi::Error`
        // directly — no `.map_err(to_string)`, which would not type-check
        // (`napi::Error` has no `From<String>`).
        func.call(None, &[arg.into_unknown()])?;
        Ok(())
    });
    opened.ok_or_else(|| {
        "OpenHarmony open_url bridge not registered (call register_platform_bridge)".to_string()
    })
}
