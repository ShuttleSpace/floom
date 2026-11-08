//! sample — a Floom plugin.
//!
//! The public surface is four functions and a capability. A caller does:
//!
//! ```ignore
//! sample::register();
//! floom_platform::plugin("sample").enable()?;
//! let version = sample::version();
//! ```
//!
//! The native half implements the contract in `native/common/floom_sample.h`;
//! this file calls it through `extern "C"`. There is no serialization boundary
//! and no runtime lookup — the symbols resolve when the crate is linked.
//!
//! # How each platform reaches its native half
//!
//! The contract is the same everywhere; only the transport differs.
//!
//! | Platform | Native half | Transport |
//! |---|---|---|
//! | desktop | C | linked into this crate |
//! | Android | Java | called from `src/android.rs` over JNI |
//! | iOS | Swift | `@_cdecl`, linked as a framework |
//! | web | JavaScript | imported by the wasm module |
//! | OpenHarmony | ArkTS | N-API |
//! | TUI | — | none: the terminal has no native surface, and the plugin says so |


/// The capability id this plugin answers with.
///
/// Stable across releases: the Rust side matches it against the enum it was
/// compiled with. Change it only when the capability itself changes.
pub const CAPABILITY_ID: i32 = 0;

/// The ABI version this Rust half was compiled against.
///
/// A native half whose major version differs is refused rather than called:
/// the two halves exchange pointers and integers whose meaning is defined by
/// this number, and a mismatch means the definitions do not line up.
pub const ABI_VERSION: i32 = 1;

/// The native half, when `build.rs` found one for this target.
///
/// `build.rs` emits `floom_plugin_native` only after it has found a native
/// half for the current target, so the declarations below never name a
/// symbol the linker cannot find.
#[cfg(floom_plugin_native)]
mod ffi {
    /// The C ABI, on every platform whose native half is C, Swift, or
    /// Kotlin through a C shim.
    #[cfg(not(target_arch = "wasm32"))]
    mod raw {
        use std::os::raw::{c_char, c_int};

        unsafe extern "C" {
            pub fn floom_sample_version() -> *const c_char;
            pub fn floom_sample_enable() -> c_int;
            pub fn floom_sample_disable();
            pub fn floom_sample_capability() -> c_int;
            pub fn floom_sample_abi_version() -> c_int;
        }
    }

    /// The web's native half is JavaScript, so the wasm module *imports* it
    /// by name rather than linking it.
    #[cfg(target_arch = "wasm32")]
    mod raw {
        use wasm_bindgen::prelude::*;

        /// The module is resolved relative to the crate root and inlined into
        /// the wasm bundle, so the path here and the file on disk have to
        /// agree.
        #[wasm_bindgen(module = "/native/web/js/floom_sample.js")]
        extern "C" {
            #[wasm_bindgen(js_name = floom_sample_version)]
            pub fn floom_sample_version() -> String;
            #[wasm_bindgen(js_name = floom_sample_enable)]
            pub fn floom_sample_enable() -> i32;
            #[wasm_bindgen(js_name = floom_sample_disable)]
            pub fn floom_sample_disable();
            #[wasm_bindgen(js_name = floom_sample_capability)]
            pub fn floom_sample_capability() -> i32;
            #[wasm_bindgen(js_name = floom_sample_abi_version)]
            pub fn floom_sample_abi_version() -> i32;
        }
    }

    /// The native half's version.
    pub fn version() -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // SAFETY: the native half returns a NUL-terminated string that
            // stays valid until the next call into this plugin. Nothing here
            // frees it, and the copy is made before the next call can happen.
            let ptr = unsafe { raw::floom_sample_version() };
            if ptr.is_null() {
                return String::new();
            }
            unsafe { std::ffi::CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned()
        }
        #[cfg(target_arch = "wasm32")]
        {
            raw::floom_sample_version()
        }
    }

    /// Claims the plugin's platform state. `0` is success.
    pub fn enable() -> i32 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // SAFETY: called on the host thread during startup, before any
            // frame. The native half compiled against this header, so the
            // call matches its declaration.
            unsafe { raw::floom_sample_enable() }
        }
        #[cfg(target_arch = "wasm32")]
        {
            raw::floom_sample_enable()
        }
    }

    /// Releases whatever `enable` claimed.
    pub fn disable() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // SAFETY: the native half tolerates a disable without a matching
            // enable.
            unsafe { raw::floom_sample_disable() };
        }
        #[cfg(target_arch = "wasm32")]
        {
            raw::floom_sample_disable();
        }
    }

    /// The capability id, as the native half reports it.
    pub fn capability() -> i32 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // SAFETY: the native half returns a plain integer.
            unsafe { raw::floom_sample_capability() }
        }
        #[cfg(target_arch = "wasm32")]
        {
            raw::floom_sample_capability()
        }
    }

    /// The ABI version the native half was compiled against.
    pub fn abi_version() -> i32 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // SAFETY: the native half returns a plain integer.
            unsafe { raw::floom_sample_abi_version() }
        }
        #[cfg(target_arch = "wasm32")]
        {
            raw::floom_sample_abi_version()
        }
    }
}

/// The OpenHarmony half, and the bridge the host supplies to it.
///
/// # Why the plugin does not depend on N-API
///
/// OpenHarmony exposes OS services only through ArkTS, and ArkTS reaches Rust
/// through N-API — but **a HAP links one N-API module**, and that module
/// belongs to the host. A plugin that exported its own `#[napi]` functions
/// would be a second module, and the host would have to merge it.
///
/// So the plugin owns the *capability* and the host owns the *transport*:
/// the host hands the plugin an [`ArkTsBridge`] over whatever channel it
/// already has, and the plugin's native half reaches ArkTS through it.
///
/// # Why there is a C shim at all
///
/// The C ABI says the native half implements the four entry points, and for
/// OpenHarmony the native half is the C shim in `native/ohos/`. The shim
/// cannot reach ArkTS by itself: the N-API environment lives in Rust, and a C
/// file cannot obtain one. So the shim calls one Rust-exported helper,
/// `floo_sample_arkts_call`, which does the N-API work.
#[cfg(target_env = "ohos")]
pub mod ohos {
    use std::ffi::CStr;
    use std::os::raw::{c_char, c_int};
    use std::sync::{Arc, OnceLock, RwLock};

    /// A handle on the ArkTS half.
    ///
    /// The host implements it over N-API and hands it to
    /// [`register_bridge`]. The trait is object-safe and `Send + Sync`
    /// because the call happens on whichever thread the host's event loop
    /// runs on, and the handle outlives the registration.
    pub trait ArkTsBridge: Send + Sync {
        /// Calls `method` on the bridge, with an optional argument.
        ///
        /// Returns the method's result as text. An `Err` is the bridge's own
        /// message, and it is what the C ABI's `enable` turns into a
        /// non-zero code.
        fn call(&self, method: &str, arg: Option<&str>) -> Result<String, String>;
    }

    fn slot() -> &'static RwLock<Option<Arc<dyn ArkTsBridge>>> {
        static SLOT: OnceLock<RwLock<Option<Arc<dyn ArkTsBridge>>>> = OnceLock::new();
        SLOT.get_or_init(|| RwLock::new(None))
    }

    /// Registers the ArkTS half's bridge.
    ///
    /// Called by the **host**, once, at startup — not by the plugin. A plugin
    /// that registered its own bridge would be a second N-API module, and a
    /// HAP has room for one.
    ///
    /// Replacing an existing bridge is allowed: a host that recreates its
    /// window registers again.
    pub fn register_bridge(bridge: Arc<dyn ArkTsBridge>) {
        let mut guard = slot().write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(bridge);
    }

    /// Whether a bridge has been registered.
    pub fn has_bridge() -> bool {
        slot().read().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// Calls a method on the registered bridge.
    ///
    /// The C shim calls this. `0` is success; a negative code is a failure
    /// the shim turns into the ABI's non-zero return.
    ///
    /// # Safety
    ///
    /// `method` must be a NUL-terminated string, or null. `arg` must be a
    /// NUL-terminated string or null. Both are read for the duration of the
    /// call.
    // Rust 2024 requires the `unsafe` marker on the attribute itself.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn floom_sample_arkts_call(
        method: *const c_char,
        arg: *const c_char,
    ) -> c_int {
        let bridge = { slot().read().unwrap_or_else(|e| e.into_inner()).clone() };
        let Some(bridge) = bridge else {
            return -1;
        };
        if method.is_null() {
            return -2;
        }
        let method = unsafe { CStr::from_ptr(method) }.to_string_lossy().into_owned();
        let arg = if arg.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(arg) }.to_string_lossy().into_owned())
        };
        match bridge.call(&method, arg.as_deref()) {
            Ok(_) => 0,
            Err(_) => -3,
        }
    }
}

/// The plugin's version, as reported by the native layer.
///
/// Falls back to this crate's version when there is no native half, so the
/// number is never a lie: it says what the caller can actually get.
pub fn version() -> String {
    #[cfg(floom_plugin_native)]
    {
        ffi::version()
    }
    #[cfg(not(floom_plugin_native))]
    {
        env!("CARGO_PKG_VERSION").to_string()
    }
}

/// Claims the plugin's platform state.
///
/// Called once by the host, through [`floom_platform::register_plugin`]. It
/// forwards to the native half's `enable`, and the native half's own error
/// message comes back verbatim: the registry puts this string in
/// [`floom_platform::PluginError::Failed`], and a wrapper here would only be
/// stripped again.
pub fn enable() -> Result<(), String> {
    #[cfg(floom_plugin_native)]
    {
        let abi = ffi::abi_version();
        if abi != ABI_VERSION {
            return Err(format!(
                "native half speaks ABI version {abi}, this crate speaks {ABI_VERSION}"
            ));
        }
        let rc = ffi::enable();
        if rc != 0 {
            return Err(format!("native enable failed with code {rc}"));
        }
        Ok(())
    }
    #[cfg(not(floom_plugin_native))]
    {
        Err("this plugin has no native half for the current target".to_string())
    }
}

/// Releases whatever [`enable`] claimed.
///
/// Safe to call without a matching enable, and a no-op when there is no
/// native half.
pub fn disable() {
    #[cfg(floom_plugin_native)]
    {
        ffi::disable();
    }
}

/// The capability id, as the native half reports it.
///
/// `None` when there is no native half: an absent plugin reports no
/// capability, rather than reporting the default one and looking like a
/// plugin that happens to do nothing.
pub fn capability() -> Option<i32> {
    #[cfg(floom_plugin_native)]
    {
        Some(ffi::capability())
    }
    #[cfg(not(floom_plugin_native))]
    {
        None
    }
}

/// Registers this plugin with the process-wide table.
///
/// Called from `main`, after the plugin crate is linked. There is no
/// constructor that does this automatically: a Rust crate has no guaranteed
/// static initialisation, and a plugin that registered itself from a `static`
/// initializer would be the reason a plugin silently went missing.
pub fn register() {
    floom_platform::register_plugin("sample", enable);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_never_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn the_version_comes_from_the_native_half() {
        // The native half reports "sample 0.1.0"; the crate's own version is
        // "0.1.0". They differ on purpose, because this test is the only one
        // that proves the call crossed the boundary rather than returning a
        // constant — every other assertion in this file passes just as well
        // with a stubbed native half.
        #[cfg(floom_plugin_native)]
        assert!(
            version().starts_with("sample "),
            "expected the native half's version, got {:?}",
            version()
        );
        #[cfg(not(floom_plugin_native))]
        assert_eq!(
            version(),
            env!("CARGO_PKG_VERSION"),
            "without a native half the version is the crate's own"
        );
    }

    #[test]
    fn enable_is_idempotent() {
        // Two enables must both behave the same way: a host may enable every
        // plugin at startup and an app may also enable one on demand, and a
        // plugin that rejected the second call would turn that into a crash.
        let first = enable();
        let second = enable();
        assert_eq!(
            first.is_ok(),
            second.is_ok(),
            "the two enables must agree about whether this target has a native half"
        );
        if let Err(detail) = first {
            assert!(
                !detail.is_empty(),
                "a failure has to say something: an empty message is the one a \\
                 caller cannot act on"
            );
        }
    }

    #[test]
    fn registering_makes_the_plugin_findable() {
        register();
        let handle = floom_platform::plugin("sample");
        assert!(handle.is_registered());
        assert!(!handle.is_enabled());
        // The enable is not unwrapped: this test is about the *registration*,
        // and a target with no native half has nothing to enable. Asserting
        // success here would make the test fail for a reason that has nothing
        // to do with what it checks.
        let _ = handle.enable();
        assert!(handle.is_enabled());
    }

    #[test]
    fn capability_agrees_with_the_presence_of_a_native_half() {
        // A plugin with no native half must not claim a capability: a caller
        // that branches on `capability().is_some()` would otherwise take the
        // native path and fail at the first call. Android is the exception
        // written into `src/android.rs`: its half is Java, not a linked C
        // symbol, so `floom_plugin_native` is false there and the capability
        // still exists.
        assert_eq!(
            capability().is_some(),
            cfg!(floom_plugin_native) || cfg!(target_os = "android"),
            "a capability must exist exactly when there is a half to answer for it"
        );
    }
}
