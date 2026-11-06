// SPDX-License-Identifier: MIT
//! Templates emitted by `floom create`.
//!
//! Stage 1 of the scaffolder emits the Rust core plus one directory per target
//! platform. Only files that can be correct without a native SDK are emitted
//! verbatim; the per-platform native project files (Gradle, Xcode, DevEco,
//! CMake) are deliberately deferred to stage 2 so that `floom create` never
//! writes a project file that does not open. See `platform_readme`.

/// Known target platforms, in the order `floom create` emits them.
pub const PLATFORMS: &[&str] = &[
    "android", "ios", "ohos", "web", "windows", "linux", "macos", "tui",
];

/// Per-platform notes: what the directory is for and which tool opens it.
pub const PLATFORM_NOTES: &[(&str, &str, &str)] = &[
    (
        "android",
        "Android",
        "Android Studio (Gradle + Kotlin host libraries)",
    ),
    ("ios", "iOS / iPadOS", "Xcode (Swift host libraries)"),
    ("ohos", "OpenHarmony", "DevEco Studio (ArkTS + hvigor)"),
    ("web", "Web (WASM)", "a static file server"),
    ("windows", "Windows", "Visual Studio / MSBuild + CMake"),
    ("linux", "Linux", "any IDE + CMake (GTK/Wayland)"),
    ("macos", "macOS", "Xcode (AppKit)"),
    (
        "tui",
        "Terminal",
        "a terminal — builds as a plain Cargo binary",
    ),
];

/// `desktop` + `renderer-gpu` are what `run_desktop` needs: one picks the
/// winit-backed desktop platform adapter, the other the Vello GPU presenter.
/// Without both, `floom::run_desktop` does not exist.
pub const DEFAULT_FEATURES: &[&str] = &["desktop", "renderer-gpu"];

fn features_literal() -> String {
    DEFAULT_FEATURES
        .iter()
        .map(|f| format!("\"{f}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn cargo_toml(name: &str, floom_crate: &str, floom_path: Option<&str>) -> String {
    let dep = match floom_path {
        Some(p) => format!(
            "{f} = {{ path = \"{p}\", features = [{features}] }}\n\
             # Re-point at the published crate once it ships:\n\
             # {f} = {{ version = \"0.2\", features = [{features}] }}",
            f = floom_crate,
            p = p,
            features = features_literal()
        ),
        None => format!(
            "{f} = {{ version = \"0.2\", features = [{features}] }}",
            f = floom_crate,
            features = features_literal()
        ),
    };
    format!(
        "[package]\n\
         name = \"{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         \n\
         [dependencies]\n\
         {dep}\n"
    )
}

/// The Rust core entry point, targeting the Floom 2.0 widget API.
pub const MAIN_RS: &str = r#"// Floom application core.
//
// All cross-platform logic lives here. The platform directories hold only the
// thin native host projects that embed this binary; they should not contain
// application logic.

use std::sync::atomic::{AtomicI64, Ordering};

use {{FLOOM}}::prelude::*;

static COUNT: AtomicI64 = AtomicI64::new(0);

fn main() {
    // `run_desktop` opens a window and pumps the event loop until it closes.
    // Other targets have one-call equivalents: `run_android`, `run_web`,
    // `run_tui`.
    if let Err(e) = {{FLOOM}}::run_desktop("{{TITLE}}", Size::new(360.0, 240.0), build) {
        eprintln!("floom: {e}");
        std::process::exit(1);
    }
}

/// Builds the widget tree. `host` owns the live widgets: they are described
/// here and mutated through `host.widget_mut::<T>(node)`.
fn build(host: &mut WidgetHost) {
    let root = Stack::new(Axis::Vertical)
        .gap(8.0)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
            Box::new(Text::new("Count: 0"))
        }))
        .child(Child::new(ChildKey::index(1), LayoutRequest::new(), || {
            Box::new(Button::new("Increment").on_click(|| {
                let n = COUNT.fetch_add(1, Ordering::Relaxed) + 1;
                println!("count = {n}");
            }))
        }));

    host.mount_root(LayoutStyle::default(), root);
    host.frame();
}

// To drive the UI from state, mutate the mounted node and pump a frame:
//
//   let node = host.node_at(0).unwrap();
//   host.widget_mut::<Text>(node).unwrap().set_content("Count: 1");
//   host.frame();
//
// `on_click` callbacks must be `'static` and so cannot capture `host`; keep
// shared state in a static or an `Arc`, then reconcile it from a frame
// callback.
"#;

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{{PROJECT}}</title>
  </head>
  <body>
    <canvas id="floom-canvas"></canvas>
    <!-- wasm-bindgen mounts the Floom renderer here once the web target is built. -->
  </body>
</html>
"#;

pub fn platform_readme(label: &str, opener: &str) -> String {
    format!(
        "# {label} target\n\
         \n\
         Opened with: **{opener}**\n\
         \n\
         This directory is a placeholder. `floom create` emits the Rust core and\n\
         one directory per target, but the native project files for this platform\n\
         are added in a later stage of the CLI, once they can be verified against\n\
         a real SDK. Emitting an unverified `.pbxproj` / `build.gradle.kts` would\n\
         produce a project that does not open, which is worse than none.\n\
         \n\
         What belongs here once stage 2 lands:\n\
         \n\
         - The native host project that embeds the compiled Floom binary.\n\
         - Kotlin/Java or Swift host libraries for any plugin capability\n\
           (camera, sensors, ...), called from Rust over a narrow `extern \"C\"`\n\
           ABI. See `docs/platform-and-plugins.md`.\n\
         \n\
         Application logic does not belong in this directory — it belongs in\n\
         `src/`, so it stays cross-platform.\n"
    )
}

pub fn root_readme(name: &str, platforms: &[String]) -> String {
    let mut s = format!(
        "# {name}\n\
         \n\
         A Floom application. The layout mirrors Flutter's: the cross-platform core\n\
         is Rust, and each platform gets its own directory that hosts a native\n\
         project.\n\
         \n\
         ```\n\
         {name}/\n\
         \u{251c}\u{2500}\u{2500} src/            Rust core — all application logic\n\
         \u{251c}\u{2500}\u{2500} Cargo.toml\n\
         \u{251c}\u{2500}\u{2500} tests/\n"
    );
    for p in platforms {
        s.push_str(&format!("\u{251c}\u{2500}\u{2500} {p}/\n"));
    }
    s.push_str(&format!(
        "\u{2514}\u{2500}\u{2500} README.md\n\
         \n\
         ## Run\n\
         \n\
         ```sh\n\
         cargo run\n\
         cargo test\n\
         ```\n\
         \n\
         ## Platforms\n\
         \n\
         {} platform director{}.\n\
         \n\
         Each holds the native host project for that target; see the README inside\n\
         for which tool opens it. Keep logic in `src/` so it stays portable.\n",
        platforms.len(),
        if platforms.len() == 1 { "y" } else { "ies" }
    ));
    s
}
// --- Plugin templates ------------------------------------------------------
//
// The plugin layout mirrors a Flutter plugin package, with one deliberate
// difference: there is no `pluginClass` registry and no method channel.
//
//   Flutter                            Floom
//   pubspec.yaml  pluginClass: X       Cargo.toml  + `floom-platform`
//   lib/x.dart                         src/lib.rs
//   android/src/main/kotlin/X.kt       native/android/src/main/kotlin/X.kt
//   ios/Classes/X.h                    native/ios/src/X.h
//   example/                           example/
//
// Flutter finds `X.kt` by reading every dependency's pubspec and generating a
// registrant. Floom does not: `build.rs` compiles `native/<platform>/` and
// links it, so the native code is reachable the moment the Rust crate
// compiles. That is the whole point — the call is `extern "C"`, not a
// serialized message.

/// The Rust half of a plugin: the crate a caller depends on.
///
/// `register` puts `enable` into `floom_platform`'s table, and it is what a
/// caller invokes before `plugin("name").enable()`. It is
/// a plain function rather than a closure because a plugin is a link-time
/// dependency in Floom, not a runtime-discovered one: by the time the app runs,
/// `enable` is already in the binary.
///
/// # How the two halves meet
///
/// The native half implements the four functions in `native/common/floom.h`
/// and this file calls them through `extern "C"`. There is no serialization
/// and no runtime lookup: a call is a machine call, and the symbols resolve
/// when the crate is linked.
///
/// # Host-inert by construction
///
/// The `extern` block is behind `#[cfg(floom_plugin_native)]`, a flag that
/// `build.rs` emits **only** when it found and compiled a native half for the
/// current target. A plugin built for a platform it does not have a native
/// half for still compiles, and its `enable` says so instead of failing to
/// link. That is the difference between "this plugin does not support Linux"
/// and "this crate does not build on Linux".
pub fn plugin_lib_rs(name: &str, android: bool) -> String {
    let ident = name.replace('-', "_");
    let android_mod = if android {
        "#[cfg(target_os = \"android\")]\nmod android;\n"
    } else {
        ""
    };
    let android_version = if android {
        "    #[cfg(target_os = \"android\")]\n    {\n        return env!(\"CARGO_PKG_VERSION\").to_string();\n    }\n"
    } else {
        ""
    };
    let android_enable = if android {
        "    #[cfg(target_os = \"android\")]\n    {\n        return android::enable();\n    }\n"
    } else {
        ""
    };
    let android_disable = if android {
        "    #[cfg(target_os = \"android\")]\n    {\n        android::disable();\n        return;\n    }\n"
    } else {
        ""
    };
    let android_capability = if android {
        "    #[cfg(target_os = \"android\")]\n    {\n        return Some(CAPABILITY_ID);\n    }\n"
    } else {
        ""
    };
    format!(
        r#"//! {name} — a Floom plugin.
//!
//! The public surface is four functions and a capability. A caller does:
//!
//! ```ignore
//! {name}::register();
//! floom_platform::plugin("{name}").enable()?;
//! let version = {name}::version();
//! ```
//!
//! The native half implements the contract in `native/common/floom_{ident}.h`;
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

{android_mod}
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
mod ffi {{
    /// The C ABI, on every platform whose native half is C, Swift, or
    /// Kotlin through a C shim.
    #[cfg(not(target_arch = "wasm32"))]
    mod raw {{
        use std::os::raw::{{c_char, c_int}};

        unsafe extern "C" {{
            pub fn floom_{ident}_version() -> *const c_char;
            pub fn floom_{ident}_enable() -> c_int;
            pub fn floom_{ident}_disable();
            pub fn floom_{ident}_capability() -> c_int;
            pub fn floom_{ident}_abi_version() -> c_int;
        }}
    }}

    /// The web's native half is JavaScript, so the wasm module *imports* it
    /// by name rather than linking it.
    #[cfg(target_arch = "wasm32")]
    mod raw {{
        use wasm_bindgen::prelude::*;

        /// The module is resolved relative to the crate root and inlined into
        /// the wasm bundle, so the path here and the file on disk have to
        /// agree.
        #[wasm_bindgen(module = "/native/web/js/floom_{ident}.js")]
        extern "C" {{
            #[wasm_bindgen(js_name = floom_{ident}_version)]
            pub fn floom_{ident}_version() -> String;
            #[wasm_bindgen(js_name = floom_{ident}_enable)]
            pub fn floom_{ident}_enable() -> i32;
            #[wasm_bindgen(js_name = floom_{ident}_disable)]
            pub fn floom_{ident}_disable();
            #[wasm_bindgen(js_name = floom_{ident}_capability)]
            pub fn floom_{ident}_capability() -> i32;
            #[wasm_bindgen(js_name = floom_{ident}_abi_version)]
            pub fn floom_{ident}_abi_version() -> i32;
        }}
    }}

    /// The native half's version.
    pub fn version() -> String {{
        #[cfg(not(target_arch = "wasm32"))]
        {{
            // SAFETY: the native half returns a NUL-terminated string that
            // stays valid until the next call into this plugin. Nothing here
            // frees it, and the copy is made before the next call can happen.
            let ptr = unsafe {{ raw::floom_{ident}_version() }};
            if ptr.is_null() {{
                return String::new();
            }}
            unsafe {{ std::ffi::CStr::from_ptr(ptr) }}
                .to_string_lossy()
                .into_owned()
        }}
        #[cfg(target_arch = "wasm32")]
        {{
            raw::floom_{ident}_version()
        }}
    }}

    /// Claims the plugin's platform state. `0` is success.
    pub fn enable() -> i32 {{
        #[cfg(not(target_arch = "wasm32"))]
        {{
            // SAFETY: called on the host thread during startup, before any
            // frame. The native half compiled against this header, so the
            // call matches its declaration.
            unsafe {{ raw::floom_{ident}_enable() }}
        }}
        #[cfg(target_arch = "wasm32")]
        {{
            raw::floom_{ident}_enable()
        }}
    }}

    /// Releases whatever `enable` claimed.
    pub fn disable() {{
        #[cfg(not(target_arch = "wasm32"))]
        {{
            // SAFETY: the native half tolerates a disable without a matching
            // enable.
            unsafe {{ raw::floom_{ident}_disable() }};
        }}
        #[cfg(target_arch = "wasm32")]
        {{
            raw::floom_{ident}_disable();
        }}
    }}

    /// The capability id, as the native half reports it.
    pub fn capability() -> i32 {{
        #[cfg(not(target_arch = "wasm32"))]
        {{
            // SAFETY: the native half returns a plain integer.
            unsafe {{ raw::floom_{ident}_capability() }}
        }}
        #[cfg(target_arch = "wasm32")]
        {{
            raw::floom_{ident}_capability()
        }}
    }}

    /// The ABI version the native half was compiled against.
    pub fn abi_version() -> i32 {{
        #[cfg(not(target_arch = "wasm32"))]
        {{
            // SAFETY: the native half returns a plain integer.
            unsafe {{ raw::floom_{ident}_abi_version() }}
        }}
        #[cfg(target_arch = "wasm32")]
        {{
            raw::floom_{ident}_abi_version()
        }}
    }}
}}

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
/// `floo_{ident}_arkts_call`, which does the N-API work.
#[cfg(target_env = "ohos")]
pub mod ohos {{
    use std::ffi::CStr;
    use std::os::raw::{{c_char, c_int}};
    use std::sync::{{Arc, OnceLock, RwLock}};

    /// A handle on the ArkTS half.
    ///
    /// The host implements it over N-API and hands it to
    /// [`register_bridge`]. The trait is object-safe and `Send + Sync`
    /// because the call happens on whichever thread the host's event loop
    /// runs on, and the handle outlives the registration.
    pub trait ArkTsBridge: Send + Sync {{
        /// Calls `method` on the bridge, with an optional argument.
        ///
        /// Returns the method's result as text. An `Err` is the bridge's own
        /// message, and it is what the C ABI's `enable` turns into a
        /// non-zero code.
        fn call(&self, method: &str, arg: Option<&str>) -> Result<String, String>;
    }}

    fn slot() -> &'static RwLock<Option<Arc<dyn ArkTsBridge>>> {{
        static SLOT: OnceLock<RwLock<Option<Arc<dyn ArkTsBridge>>>> = OnceLock::new();
        SLOT.get_or_init(|| RwLock::new(None))
    }}

    /// Registers the ArkTS half's bridge.
    ///
    /// Called by the **host**, once, at startup — not by the plugin. A plugin
    /// that registered its own bridge would be a second N-API module, and a
    /// HAP has room for one.
    ///
    /// Replacing an existing bridge is allowed: a host that recreates its
    /// window registers again.
    pub fn register_bridge(bridge: Arc<dyn ArkTsBridge>) {{
        let mut guard = slot().write().unwrap_or_else(|e| e.into_inner());
        *guard = Some(bridge);
    }}

    /// Whether a bridge has been registered.
    pub fn has_bridge() -> bool {{
        slot().read().unwrap_or_else(|e| e.into_inner()).is_some()
    }}

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
    pub unsafe extern "C" fn floom_{ident}_arkts_call(
        method: *const c_char,
        arg: *const c_char,
    ) -> c_int {{
        let bridge = {{ slot().read().unwrap_or_else(|e| e.into_inner()).clone() }};
        let Some(bridge) = bridge else {{
            return -1;
        }};
        if method.is_null() {{
            return -2;
        }}
        let method = unsafe {{ CStr::from_ptr(method) }}.to_string_lossy().into_owned();
        let arg = if arg.is_null() {{
            None
        }} else {{
            Some(unsafe {{ CStr::from_ptr(arg) }}.to_string_lossy().into_owned())
        }};
        match bridge.call(&method, arg.as_deref()) {{
            Ok(_) => 0,
            Err(_) => -3,
        }}
    }}
}}

/// The plugin's version, as reported by the native layer.
///
/// Falls back to this crate's version when there is no native half, so the
/// number is never a lie: it says what the caller can actually get.
pub fn version() -> String {{
{android_version}    #[cfg(floom_plugin_native)]
    {{
        ffi::version()
    }}
    #[cfg(not(floom_plugin_native))]
    {{
        env!("CARGO_PKG_VERSION").to_string()
    }}
}}

/// Claims the plugin's platform state.
///
/// Called once by the host, through [`floom_platform::register_plugin`]. It
/// forwards to the native half's `enable`, and the native half's own error
/// message comes back verbatim: the registry puts this string in
/// [`floom_platform::PluginError::Failed`], and a wrapper here would only be
/// stripped again.
pub fn enable() -> Result<(), String> {{
{android_enable}    #[cfg(floom_plugin_native)]
    {{
        let abi = ffi::abi_version();
        if abi != ABI_VERSION {{
            return Err(format!(
                "native half speaks ABI version {{abi}}, this crate speaks {{ABI_VERSION}}"
            ));
        }}
        let rc = ffi::enable();
        if rc != 0 {{
            return Err(format!("native enable failed with code {{rc}}"));
        }}
        Ok(())
    }}
    #[cfg(not(floom_plugin_native))]
    {{
        Err("this plugin has no native half for the current target".to_string())
    }}
}}

/// Releases whatever [`enable`] claimed.
///
/// Safe to call without a matching enable, and a no-op when there is no
/// native half.
pub fn disable() {{
{android_disable}    #[cfg(floom_plugin_native)]
    {{
        ffi::disable();
    }}
}}

/// The capability id, as the native half reports it.
///
/// `None` when there is no native half: an absent plugin reports no
/// capability, rather than reporting the default one and looking like a
/// plugin that happens to do nothing.
pub fn capability() -> Option<i32> {{
{android_capability}    #[cfg(floom_plugin_native)]
    {{
        Some(ffi::capability())
    }}
    #[cfg(not(floom_plugin_native))]
    {{
        None
    }}
}}

/// Registers this plugin with the process-wide table.
///
/// Called from `main`, after the plugin crate is linked. There is no
/// constructor that does this automatically: a Rust crate has no guaranteed
/// static initialisation, and a plugin that registered itself from a `static`
/// initializer would be the reason a plugin silently went missing.
pub fn register() {{
    floom_platform::register_plugin("{name}", enable);
}}

#[cfg(test)]
mod tests {{
    use super::*;

    #[test]
    fn version_is_never_empty() {{
        assert!(!version().is_empty());
    }}

    #[test]
    fn the_version_comes_from_the_native_half() {{
        // The native half reports "{name} 0.1.0"; the crate's own version is
        // "0.1.0". They differ on purpose, because this test is the only one
        // that proves the call crossed the boundary rather than returning a
        // constant — every other assertion in this file passes just as well
        // with a stubbed native half.
        #[cfg(floom_plugin_native)]
        assert!(
            version().starts_with("{name} "),
            "expected the native half's version, got {{:?}}",
            version()
        );
        #[cfg(not(floom_plugin_native))]
        assert_eq!(
            version(),
            env!("CARGO_PKG_VERSION"),
            "without a native half the version is the crate's own"
        );
    }}

    #[test]
    fn enable_is_idempotent() {{
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
        if let Err(detail) = first {{
            assert!(
                !detail.is_empty(),
                "a failure has to say something: an empty message is the one a \\
                 caller cannot act on"
            );
        }}
    }}

    #[test]
    fn registering_makes_the_plugin_findable() {{
        register();
        let handle = floom_platform::plugin("{name}");
        assert!(handle.is_registered());
        assert!(!handle.is_enabled());
        // The enable is not unwrapped: this test is about the *registration*,
        // and a target with no native half has nothing to enable. Asserting
        // success here would make the test fail for a reason that has nothing
        // to do with what it checks.
        let _ = handle.enable();
        assert!(handle.is_enabled());
    }}

    #[test]
    fn capability_agrees_with_the_presence_of_a_native_half() {{
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
    }}
}}
"#,
        name = name,
        ident = ident,
        android_mod = android_mod,
        android_version = android_version,
        android_enable = android_enable,
        android_disable = android_disable,
        android_capability = android_capability
    )
}

/// The Android edge of a plugin: `src/android.rs`.
///
/// The capability itself is in `android/src/main/java`; this file is the JNI
/// call that reaches it and nothing else. Two rules keep it honest:
///
/// * the class is resolved through the activity's class loader, because
///   `FindClass` on a thread that reached the JVM through
///   `AttachCurrentThread` searches the system loader and cannot see the app's
///   dex;
/// * the class name is a string, so the generated Java and this file have to
///   agree — there is no compiler to catch a rename.
pub fn plugin_android_rs(name: &str) -> String {
    let class = format!("{}Plugin", crate::native_project::kotlin_class_name(name));
    let package = format!("dev/floom/plugin/{}", name.replace('-', "_"));
    format!(
        r#"//! The Android edge of the `{name}` plugin.
//!
//! The capability lives in
//! `android/src/main/java/{package}/{class}.java`; this file reaches it over
//! JNI. `{class}` must keep its package and its name: the lookup below is a
//! string, so a rename shows up on the device as a `ClassNotFoundException`
//! rather than as a compile error.

use floom_platform_mobile::jni::{{self as bridge, JniExt}};
use jni::objects::JValue;

const CLASS: &str = "{package}/{class}";

/// Calls the Java half's `enable(Context)`.
pub fn enable() -> Result<(), String> {{
    bridge::with_env(|env| {{
        let class = bridge::app_class(env, CLASS)?;
        let context = bridge::activity(env)?;
        let enabled = env
            .call_static_method(
                &class,
                jni::jni_str!("enable"),
                jni::jni_sig!("(Landroid/content/Context;)Z"),
                &[JValue::Object(&context)],
            )
            .e()?
            .z()
            .e()?;
        if enabled {{
            Ok(())
        }} else {{
            Err(format!("{{CLASS}}.enable returned false"))
        }}
    }})
}}

/// Calls the Java half's `disable()`.
///
/// A failure is ignored on purpose: `disable` has no return value in the
/// plugin API, and a half that throws on teardown is the case where the caller
/// has nothing left to do about it.
pub fn disable() {{
    let _ = bridge::with_env(|env| {{
        let class = bridge::app_class(env, CLASS)?;
        env.call_static_method(&class, jni::jni_str!("disable"), jni::jni_sig!("()V"), &[])
            .e()?;
        Ok(())
    }});
}}
"#,
        name = name,
        class = class
    )
}

/// A plugin's `Cargo.toml`.
///
/// Three things differ from `cargo_toml`, and all three are the point of a
/// plugin:
///
/// * it depends on `floom-platform` for `Capability` and `PluginError`, not on
///   the `floom` facade, because a plugin must not pull in the whole widget
///   stack;
/// * `build.rs` is declared, because the native half is compiled at build time;
/// * `cc` is a build dependency, because `build.rs` compiles the native half's
///   C sources with it.
pub fn plugin_cargo_toml(name: &str, floom_path: Option<&str>, platforms: &[String]) -> String {
    let dep = match floom_path {
        Some(p) => format!(
            "floom-platform = {{ path = \"{p}\" }}\n\
             # Re-point at the published crate once it ships:\n\
             # floom-platform = {{ version = \"0.2\" }}"
        ),
        None => "floom-platform = { version = \"0.2\" }".to_string(),
    };
    let android = if platforms.iter().any(|p| p == "android") {
        let mobile = match floom_path.and_then(|p| {
            std::path::Path::new(p)
                .parent()
                .map(|dir| dir.join("floom-platform-mobile"))
        }) {
            Some(path) => format!(
                "floom-platform-mobile = {{ path = \"{}\" }}",
                path.display()
            ),
            None => "floom-platform-mobile = { version = \"0.2\" }".to_string(),
        };
        format!(
            "\n\
             [target.'cfg(target_os = \"android\")'.dependencies]\n\
             # The JNI bridge — the JavaVM and the activity — is platform\n\
             # plumbing, so the plugin reaches down for it rather than owning a\n\
             # second copy of it.\n\
             {mobile}\n\
             jni = \"0.22\"\n\n"
        )
    } else {
        String::new()
    };
    format!(
        "[package]\n\
         name = \"floom-plugin-{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         description = \"A Floom plugin: {name}\"\n\
         build = \"build.rs\"\n\
         \n\
         # A plugin is developed on its own, like the example beside it: an\n\
         # empty `[workspace]` keeps it out of whatever workspace happens to\n\
         # contain it, so `cargo build` here builds the plugin and not the\n\
         # directory it was generated into.\n\
         [workspace]\n\
         \n\
         [dependencies]\n\
         {dep}\n\
         \n\
         # `build.rs` compiles the native half's C sources with this. It is a\n\
         # build dependency, not a runtime one: the native code is linked into\n\
         # this crate, so the app never loads it by name.\n\
         [build-dependencies]\n\
         cc = \"1\"\n\
         \n\
         [target.'cfg(target_arch = \"wasm32\")'.dependencies]\n\
         # The web's native half is JavaScript, and the wasm module imports it.\n\
         # `wasm-bindgen` is what declares that import and what the bundler\n\
         # processes, so it is a dependency of the wasm build only.\n\
         wasm-bindgen = \"0.2\"\n\
         {android}\
         [dev-dependencies]\n\
         # A plugin is tested without a device: the pure-Rust half is an ordinary\n\
         # crate, so `cargo test` covers it and the native half is covered by the\n\
         # example app.\n"
    )
}

/// `build.rs` for a plugin.
///
/// This is where the two halves meet. It finds the native half for the
/// **current target**, compiles it, links it into the crate, and tells the
/// Rust side it exists by emitting `cargo:rustc-cfg=floom_plugin_native`. The
/// Rust half's `extern "C"` block is behind that flag, so it never names a
/// symbol the linker cannot find.
///
/// # Why the flag instead of `#[cfg(target_os = ...)]`
///
/// A target OS is not the same question as "does this plugin have a native
/// half for this target". A plugin generated for Android only, compiled on
/// Linux, has no Linux half — but a `target_os` check would declare the extern
/// block anyway and fail at link time with an undefined symbol, which is a
/// build error in a crate that should have compiled. The flag answers the
/// question that is actually being asked.
///
/// # Host-inert
///
/// No native half for this target is not an error. The plugin compiles, and its
/// `enable` reports that there is no native half — a message a caller can act
/// on, rather than a link failure.
pub fn plugin_build_rs(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"//! Build script for the `{name}` plugin.
//!
//! Compiles the native half for the current target and links it into this
//! crate. The native half implements the contract in
//! `native/common/floom_{name}.h`; the Rust half in `src/lib.rs` calls it
//! through `extern "C"`.
//!
//! There is no runtime plugin discovery and therefore no serialization
//! boundary: a call from the app is a machine call, resolved when this crate
//! is linked.

use std::path::PathBuf;

fn main() {{
    println!("cargo:rerun-if-changed=native");
    // The Rust side gates its `extern` block on this flag, so it has to be
    // declared as an expected cfg or every build warns about an unknown one.
    println!("cargo:rustc-check-cfg=cfg(floom_plugin_native)");

    let manifest = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"),
    );
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    // Android's half is Java. It is compiled into the app's dex, not linked into
    // this crate, and `src/android.rs` reaches it over JNI — so there is no C
    // source to look for here and no warning to print.
    if target_os == "android" {{
        return;
    }}

    // Which directory holds this target's native half.
    //
    // The desktop half is one directory for all three desktop targets: the C
    // ABI it implements is the same, and a per-OS copy would be three files
    // that differ only in a `#ifdef`.
    //
    // OpenHarmony is `target_os = "linux"` with `target_env = "ohos"`, so the
    // environment has to be checked before the OS: matching on the OS alone
    // would send an OHOS build to the desktop half.
    //
    // The web is chosen by *architecture*, not by OS: a wasm32 target is the
    // web wherever it is built, and there is no other wasm32 target a plugin
    // would be built for.
    let platform_dir = if target_arch == "wasm32" {{
        "web"
    }} else {{
        match (target_os.as_str(), target_env.as_str()) {{
            ("linux", "ohos") => "ohos",
            ("ios", _) => "ios",
            ("linux", _) | ("macos", _) | ("windows", _) => "desktop",
            _ => "",
        }}
    }};

    if platform_dir.is_empty() {{
        println!(
            "cargo:warning={name}: no native half for target_os={{target_os}}; \
             the plugin compiles but enable() will report it"
        );
        return;
    }}

    let native_dir = manifest.join("native").join(platform_dir);
    if !native_dir.is_dir() {{
        println!(
            "cargo:warning={name}: native/{{platform_dir}} does not exist; \
             the plugin compiles but enable() will report it"
        );
        return;
    }}

    // The C sources for this platform. A platform whose native half is not C
    // (iOS is Swift) has none, and is handled below.
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&native_dir)
        .map(|entries| {{
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "c"))
                .collect()
        }})
        .unwrap_or_default();
    sources.sort();

    if !sources.is_empty() {{
        cc::Build::new()
            .files(&sources)
            .include(manifest.join("native").join("common"))
            .include(&native_dir)
            .flag_if_supported("-std=c11")
            .warnings(true)
            .compile("floom_{ident}");
        println!("cargo:rustc-cfg=floom_plugin_native");
        return;
    }}

    // The web's native half is JavaScript, imported by the wasm module, so
    // there is nothing to compile here: the module is bundled by the app's
    // build (wasm-bindgen or a bundler), and the Rust side declares the
    // imports by module name.
    if platform_dir == "web" {{
        println!("cargo:rustc-cfg=floom_plugin_native");
        println!(
            "cargo:warning={name}: the web half is JavaScript; make sure the \
             bundler picks up native/web/js/floom_{ident}.js"
        );
        return;
    }}

    // OpenHarmony's native half is ArkTS, reached over N-API. The four ABI
    // functions are implemented on the Rust side for that platform, because
    // the N-API environment lives in Rust and a C shim cannot obtain one --
    // so there is nothing to compile here either.
    if platform_dir == "ohos" {{
        println!("cargo:rustc-cfg=floom_plugin_native");
        println!(
            "cargo:warning={name}: the OpenHarmony half is ArkTS over N-API; the \
             ArkTS side must call register_bridge at startup"
        );
        return;
    }}

    // iOS: the native half is Swift, so `cc` cannot compile it. It is built by
    // `xcodebuild` into an xcframework, which this script links when it is
    // there.
    if target_os == "ios" {{
        let framework = native_dir.join("libfloom_{ident}.xcframework");
        if framework.is_dir() {{
            println!("cargo:rustc-link-search=native/ios/libfloom_{ident}.xcframework");
            println!("cargo:rustc-link-lib=static=floom_{ident}");
            println!("cargo:rustc-cfg=floom_plugin_native");
        }} else {{
            println!(
                "cargo:warning={name}: native/ios/libfloom_{ident}.xcframework not found; \
                 build it with xcodebuild first (see native/ios/README.md). \
                 The plugin compiles but enable() will report it"
            );
        }}
        return;
    }}

    println!(
        "cargo:warning={name}: native/{{platform_dir}} has no .c source; \
         the plugin compiles but enable() will report it"
    );
}}
"#
    )
}

/// The framework's native-composition provider ABI, for a plugin that
/// contributes a native view.
///
/// A plugin whose capability is a *native view* — a camera preview, a web
/// view, a video — implements this header's vtable as well as the plugin's
/// own four-function ABI. The two are different contracts: the plugin's own
/// says whether the capability is available, and this one says how the native
/// view is created, placed and torn down.
///
/// The header is embedded rather than depended on, because the CLI is
/// deliberately dependency-free — it must build from std alone so that
/// `floom create` is available on a machine that has nothing but Rust. The
/// copy is checked against the crate's own header by a test, so the two
/// cannot drift.
pub const PROVIDER_HEADER: &str =
    include_str!("../../../native/floom-native-ffi/include/floom_native_provider.h");

/// A plugin's `native/desktop/README.md`.
///
/// The desktop half is the one platform whose native code `build.rs` compiles
/// directly, so this directory has no build system of its own. The README says
/// so, because a reader who expects a CMake project will look for one.
pub fn plugin_desktop_readme(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"# The desktop native half

This directory holds the desktop implementation of the C ABI in
`../common/floom_{ident}.h`.

**There is no CMake project here, and that is deliberate.** `build.rs` compiles
the `.c` files in this directory with the `cc` crate and links them into the
Rust crate, so the native code is in the binary from the first build. A second
build system over the same sources would compile them twice and produce
duplicate symbols at link time.

One directory serves Linux, macOS and Windows: the C ABI it implements is the
same on all three, and a per-OS copy would be three files that differ only in
an `#ifdef`.

Replace the body of `floom_{ident}_enable` with the real work. The header is
the contract, and nothing above it changes.
"#
    )
}

/// A plugin's reference native half: a C file implementing the C ABI.
///
/// It is the **desktop** half, and it is deliberately trivial — it claims
/// nothing and reports its version. It exists so that a freshly generated
/// plugin compiles, links and passes its tests on the machine that generated
/// it, which is the only way to tell that the boundary works before there is
/// any real capability to put behind it.
///
/// Replace it with the real implementation: the C ABI is the contract, and
/// nothing above it changes.
pub fn plugin_native_c(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"/* The desktop native half of the `{name}` plugin.
 *
 * Implements the contract in `native/common/floom_{name}.h`. This is the
 * reference implementation: it claims nothing and reports its version, so a
 * generated plugin compiles, links and passes its tests before there is any
 * real capability behind it.
 *
 * Replace the bodies with the real work. The ABI does not change.
 */

#include "floom_{ident}.h"

const char *floom_{ident}_version(void) {{
    /* Owned by the callee: the next call into this plugin invalidates it. */
    static const char version[] = "{name} 0.1.0";
    return version;
}}

int floom_{ident}_enable(void) {{
    /* Nothing to claim yet. Return 0 for success, non-zero to refuse. */
    return 0;
}}

void floom_{ident}_disable(void) {{
    /* Nothing to release. */
}}

int floom_{ident}_capability(void) {{
    return 0;
}}

int floom_{ident}_abi_version(void) {{
    return FLOOM_{upper}_ABI_VERSION;
}}
"#,
        ident = ident,
        upper = ident.to_uppercase()
    )
}

/// The plugin's `README.md`, stating the two halves and the boundary between them.
pub fn plugin_root_readme(name: &str, platforms: &[String]) -> String {
    let mut s = format!(
        "# floom-plugin-{name}\n\
         \n\
         A Floom plugin. Two halves, one binary:\n\
         \n\
         | | where | who calls it |\n\
         |---|---|---|\n\
         | Rust | `src/lib.rs` | the app |\n\
         | native | `native/<platform>/` | `build.rs`, at compile time |\n\
         \n\
         The Rust half calls the native half through `extern \"C\"` directly. There\n\
         is no method channel and no serialization: a call from the app is a\n\
         machine call, and it is resolved when the crate is linked rather than\n\
         looked up at runtime.\n\
         \n\
         ## Platforms\n\
         \n"
    );
    if platforms.is_empty() {
        s.push_str("_No platforms requested._\n");
    } else {
        for p in platforms {
            if let Some((_, label, _)) = PLATFORM_NOTES.iter().find(|(k, _, _)| k == p) {
                s.push_str(&format!("* `native/{p}/` — {label}\n"));
            }
        }
    }
    s.push_str(
        "\n\
         ## Using it\n\
         \n\
         ```toml\n\
         [dependencies]\n\
         floom-plugin-{name} = {{ path = \"../plugins/{name}\" }}\n\
         ```\n\
         \n\
         ```rust,ignore\n\
         floom_plugin_{name}::register();\n\
         floom_platform::plugin(\"{name}\").enable()?;\n\
         ```\n\
         \n\
         ## Example\n\
         \n\
         `example/` is a runnable app that depends on this plugin:\n\
         \n\
         ```text\n\
         cd example && cargo run\n\
         ```\n\
         \n\
         ## Add a platform later\n\
         \n\
         ```text\n\
         floom create --plugin --platforms ios .\n\
         ```\n",
    );
    s
}

/// The example app's `main.rs` — small enough to read in one screen.
pub fn plugin_example_main_rs(name: &str, floom_crate: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"// Example app for the `{name}` plugin.
//
// Small on purpose: it proves the plugin links, enables, and is callable, and
// nothing else. The plugin does its real work natively, so there is no widget
// state here worth writing a test around.

use {floom_crate}::prelude::*;

fn main() {{
    floom_plugin_{ident}::register();
    floom_platform::plugin("{name}")
        .enable()
        .expect("enable {name}");
    println!("{name} plugin version: {{}}", floom_plugin_{ident}::version());

    if let Err(e) = {floom_crate}::run_desktop(
        "{name} example",
        Size::new(320.0, 180.0),
        |host: &mut WidgetHost| {{
            host.mount_root(
                LayoutStyle::default(),
                Stack::new(Axis::Vertical).child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    || Box::new(Text::new("{name} plugin enabled")),
                )),
            );
            host.frame();
        }},
    ) {{
        eprintln!("{name} example: {{e}}");
        std::process::exit(1);
    }}
}}
"#,
        name = name,
        ident = ident,
        floom_crate = floom_crate
    )
}

/// The C header in `native/common/` — the contract both halves agree on.
///
/// This file is the reason a plugin has no serialization. The Kotlin and Swift
/// halves implement these symbols, `build.rs` compiles them, `src/lib.rs` calls
/// them through generated bindings, and the argument types are integers. A
/// `std::string` crossing this boundary is a smell: it means something is
/// copying a string because the boundary could not express it, which is the
/// method-channel cost arriving through the back door.
///
/// # Who implements what
///
/// The header is the **single source of truth** for the boundary, and it says
/// which side implements each symbol: the **native half implements them, the
/// Rust half calls them**. That direction is what makes the header portable —
/// a C file, a Swift `@_cdecl` function and a Kotlin-backed C shim all satisfy
/// it, and the Rust side is identical for all three.
///
/// The opposite direction (Rust exports, native calls) is what `external fun`
/// in Kotlin gives you, and it is a *different* contract: it cannot express a
/// Rust-owned capability, and it makes the Rust crate the thing that has to
/// know about each platform. One header, one direction, no second contract to
/// keep in sync.
///
/// # The ABI version is a function, not only a macro
///
/// `FLOOM_<NAME>_ABI_VERSION` is a compile-time constant the *native* half
/// asserts, and `floom_<name>_abi_version()` returns it at runtime. The macro
/// alone is not enough: the Rust half has to be able to ask a plugin it did
/// not compile what it was built against, and a macro cannot answer that.
pub fn plugin_header_rs(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"/* {name} — the native/Rust contract.
 *
 * Everything here is a plain C ABI: no serialization, no runtime lookup, no
 * registry. The Rust half calls these symbols directly, and they resolve when
 * the crate is linked.
 *
 * The **native half implements** these functions; the **Rust half calls**
 * them. Every platform satisfies the same contract: a C file implements them
 * directly, Swift implements them with `@_cdecl`, and Kotlin is reached
 * through a C shim that forwards over JNI.
 *
 * Strings are returned as `const char *` owned by the callee. The caller must
 * not free them; the next call into the same plugin invalidates the previous
 * one. That constraint is the price of not serializing, and it is cheaper than
 * the copy it replaces.
 *
 * This file is valid C and valid C++ — headers in plugins tend to be included
 * from both.
 */

#ifndef FLOOM_{upper}_H
#define FLOOM_{upper}_H

#ifdef __cplusplus
extern "C" {{
#endif

/* ABI version. The Rust half refuses to enable a plugin whose major differs. */
#define FLOOM_{upper}_ABI_VERSION 1

/* The plugin's version string, owned by the callee. Never NULL. */
const char *floom_{ident}_version(void);

/* Claims the plugin's capability. Returns 0 on success, non-zero on failure,
 * in which case nothing was claimed and the caller must not proceed. */
int floom_{ident}_enable(void);

/* Releases whatever `floom_{ident}_enable` claimed. Safe to call without a
 * matching enable. */
void floom_{ident}_disable(void);

/* The plugin's capability id, stable across releases. The Rust side matches
 * this against the enum it was compiled with. */
int floom_{ident}_capability(void);

/* The ABI version this half was actually compiled against.
 *
 * A function rather than only the macro above, because the Rust half has to be
 * able to ask a plugin it did not compile itself what it was built against —
 * a compile-time macro cannot answer that question. Returns
 * `FLOOM_{upper}_ABI_VERSION`.
 */
int floom_{ident}_abi_version(void);

#ifdef __cplusplus
}} /* extern "C" */
#endif

#endif /* FLOOM_{upper}_H */
"#,
        name = name,
        ident = ident,
        upper = ident.to_uppercase()
    )
}

/// A plugin's web half: the JavaScript module the wasm module imports.
///
/// The four functions keep the names the C ABI gives them, so the contract is
/// the same one every other platform implements — the transport is what
/// differs. On the web the wasm module *imports* them rather than linking
/// them, and `wasm-bindgen` resolves the import to this file.
///
/// The module path in `src/lib.rs` and this file's path have to agree: the
/// import is resolved relative to the crate root.
pub fn plugin_web_js(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"// The web half of the `{name}` plugin.
//
// The wasm module imports these four functions by name. The names are the
// ones the C ABI in `native/common/floom_{ident}.h` uses, so the contract is
// the same one every other platform implements — only the transport differs:
// here the wasm module imports JavaScript rather than linking a native
// library.
//
// Replace the bodies with the real work. The names do not change.

export function floom_{ident}_version() {{
    return "{name} 0.1.0";
}}

export function floom_{ident}_enable() {{
    // 0 is success; anything else is a refusal, and the Rust side turns it
    // into a message.
    return 0;
}}

export function floom_{ident}_disable() {{}}

export function floom_{ident}_capability() {{
    return 0;
}}

export function floom_{ident}_abi_version() {{
    return 1;
}}
"#
    )
}

/// A plugin's `native/web/README.md`.
pub fn plugin_web_readme(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"# The web half

`js/floom_{ident}.js` is the web implementation of the C ABI in
`../common/floom_{ident}.h`.

On every other platform the native half is **linked** into the binary. On the
web it cannot be: there is no native library to link, so the wasm module
**imports** these functions instead, and `wasm-bindgen` resolves the import
to the JavaScript file.

That means two things have to line up, and both are checked by the build
rather than by a person:

- the module path in `src/lib.rs` (`/native/web/js/floom_{ident}.js`) and this
  file's path;
- the exported names and the ones the Rust side declares.

`wasm-bindgen` turns the import into a JS module the bundler resolves, so the
app's build has to run it over the wasm output. `floom-renderer-web` already
depends on `wasm-bindgen`, so the app's web toolchain has it.

## No serialization on the web either

The version crosses as a JS string and the rest as numbers. That is the one
place the "no serialization" rule bends: a JS string is UTF-16 and a Rust
string is UTF-8, so the boundary has to convert. Everything else — the
capability id, the enable code — is an integer in both.
"#
    )
}

/// A plugin's OpenHarmony half: the ArkTS class that carries the capability.
///
/// ArkTS is the language OpenHarmony's OS services are reached through, so it
/// is the language the capability is written in. The class is plain
/// ArkTS/TypeScript: the plugin's own business, with the platform's
/// frameworks available to it.
pub fn plugin_ohos_ets(name: &str) -> String {
    let class = format!("{}Plugin", crate::native_project::kotlin_class_name(name));
    format!(
        r#"// The OpenHarmony half of the `{name}` plugin.
//
// ArkTS is where OpenHarmony's OS services live, so the capability is written
// here. The Rust side reaches this class through the bridge the **host**
// registers — see `../README.md` for why the host owns that.
//
// The method names are the ones the C shim asks for: `enable`, `disable`,
// `isAvailable` and `perform`.

export class {class} {{
    /** Set by the host before any plugin call. */
    private appContext: ESObject | null = null;

    /**
     * Claims the plugin's platform state.
     *
     * Returning a string rather than throwing keeps the boundary in the same
     * shape in both directions: Rust reads a message, it does not have to
     * unwind an ArkTS exception across FFI.
     */
    enable(): string | null {{
        return null;
    }}

    /** Releases whatever `enable` claimed. */
    disable(): void {{}}

    /** Whether the plugin currently has what it needs to run. */
    isAvailable(): boolean {{
        return this.appContext !== null;
    }}

    /**
     * Runs the plugin's work and reports whether it succeeded.
     *
     * Returning a `string` error rather than throwing keeps the boundary in
     * the same shape in both directions.
     */
    perform(): string | null {{
        return null;
    }}
}}
"#
    )
}

/// A plugin's OpenHarmony C shim — the C ABI, implemented over the bridge.
///
/// The shim cannot reach ArkTS by itself: the N-API environment lives in
/// Rust, and a C file cannot obtain one. So it calls the one Rust-exported
/// helper, `floo_{ident}_arkts_call`, which does the N-API work.
pub fn plugin_ohos_c_shim(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"/* The OpenHarmony C shim for the `{name}` plugin.
 *
 * Implements the contract in `native/common/floom_{ident}.h` and reaches the
 * ArkTS half through the bridge the host registered.
 *
 * The bridge call goes through `floom_{ident}_arkts_call`, which Rust
 * exports: the N-API environment lives in Rust, and a C file cannot obtain
 * one. That single indirection is the whole price of keeping the C ABI as the
 * contract on a platform whose OS services are reachable only from ArkTS.
 */

#include "floom_{ident}.h"

/* `NULL` for the "no argument" case below. */
#include <stddef.h>

/* Exported by the Rust half. Returns 0 on success, negative on failure. */
extern int floom_{ident}_arkts_call(const char *method, const char *arg);

const char *floom_{ident}_version(void) {{
    /* Owned by the callee: the next call into this plugin invalidates it. */
    static const char version[] = "{name} 0.1.0";
    return version;
}}

int floom_{ident}_enable(void) {{
    return floom_{ident}_arkts_call("enable", NULL) == 0 ? 0 : 1;
}}

void floom_{ident}_disable(void) {{
    (void)floom_{ident}_arkts_call("disable", NULL);
}}

int floom_{ident}_capability(void) {{
    return 0;
}}

int floom_{ident}_abi_version(void) {{
    return FLOOM_{upper}_ABI_VERSION;
}}
"#,
        ident = ident,
        upper = ident.to_uppercase()
    )
}

/// A plugin's `native/ohos/README.md`.
pub fn plugin_ohos_readme(name: &str) -> String {
    let ident = name.replace('-', "_");
    let class = format!("{}Plugin", crate::native_project::kotlin_class_name(name));
    format!(
        r#"# The OpenHarmony half

Two files:

- `ets/{class}.ets` — the ArkTS class that carries the capability. ArkTS is
  where OpenHarmony's OS services live, so this is where the capability is
  written.
- `floom_{ident}_ohos.c` — the C shim that implements the C ABI in
  `../common/floom_{ident}.h` and reaches the ArkTS class through the
  bridge.

## The host owns the bridge

A HAP links **one** N-API module, and it belongs to the host. A plugin that
exported its own `#[napi]` functions would be a second module, so the plugin
does not have one: the host holds the N-API environment and hands the plugin
an `ArkTsBridge` at startup.

```rust
// in the host, once, at startup
{name}::ohos::register_bridge(std::sync::Arc::new(MyBridge::new(env, obj)));
```

`MyBridge` is the host's own type: it implements
`{name}::ohos::ArkTsBridge` over N-API and calls the ArkTS class's methods.

## Building

The shim is C, so it is compiled by `build.rs` with the OpenHarmony NDK's
clang — the same toolchain the host is built with. The cross-compile needs:

```sh
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER="$OHOS_NDK/llvm/bin/aarch64-unknown-linux-ohos-clang"
```

Without it the shim does not compile, `build.rs` says so, and the plugin
falls back to reporting that it has no native half for this target.

## Not verified on device

This half has been cross-compiled, not run. The OpenHarmony path in Floom 2.0
is itself incomplete — `floom-platform-harmony` is still a stub — so this
half is ahead of the host it would run in.
"#
    )
}

/// A plugin's `native/tui/README.md`.
pub fn plugin_tui_readme(name: &str) -> String {
    format!(
        r#"# The terminal half of `{name}`

There isn't one, and that is the honest answer rather than a missing file.

A terminal is not a platform with OS services: it is a character grid. There
is no camera behind it, no sensor, no secure enclave, no window manager. A
capability that means something on a phone has no meaning here, and the
plugin says so rather than pretending.

## What happens instead

A terminal build compiles the plugin and reports that it has no native half
for the target. `enable()` returns an error saying exactly that, so a host
can ask for the capability and learn that the platform does not have it.

If a capability *does* have a terminal meaning — a clipboard, a colour
query, a terminal query — it belongs in the C half in `../desktop/`, which is
compiled for a terminal build like any other desktop build, because a
terminal app is an ordinary program on the host operating system.

## Why the plugin is not broken by this

`build.rs` finds no native half for a terminal target, so it does not emit
`floom_plugin_native`, and the Rust half's `extern` block is not compiled in.
Nothing is linked that is not there, and the plugin still builds.
"#
    )
}

/// A plugin's example app manifest.
///
/// Depends on the plugin by path so `cd example && cargo run` works in a fresh
/// checkout with no registry and no workspace: a plugin is usually developed on
/// its own, and an example that needs a published version is useless for that.
pub fn plugin_example_cargo_toml(
    name: &str,
    floom_crate: &str,
    floom_path: Option<&str>,
    platform_path: Option<&str>,
) -> String {
    let dep = match floom_path {
        Some(p) => format!(
            "{f} = {{ path = \"{p}\", features = [\"desktop\", \"renderer-gpu\"] }}",
            f = floom_crate
        ),
        None => format!(
            "{f} = {{ version = \"0.2\", features = [\"desktop\", \"renderer-gpu\"] }}",
            f = floom_crate
        ),
    };
    let platform_dep = match platform_path {
        Some(p) => format!("floom-platform = {{ path = \"{p}\" }}"),
        None => "floom-platform = { version = \"0.2\" }".to_string(),
    };
    // A raw string rather than a `\`-continued one: the continuation escape
    // strips the leading whitespace of the line it continues onto, so a
    // comment written one-line-per-source-line comes out indented
    // differently from itself. The manifest is read by a person first and by
    // cargo second, and a comment that is indented differently from its
    // neighbours is a comment that gets rewritten before it gets read.
    format!(
        r#"[package]
name = "floom-plugin-{name}-example"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
floom-plugin-{name} = {{ path = ".." }}
# The registry the plugin registers itself into. The example depends on
# it directly because it calls `plugin("{name}").enable()`, which a
# plugin's own re-exports would otherwise hide.
{platform_dep}
# `desktop` and `renderer-gpu` are what `run_desktop` needs: one picks
# the winit-backed desktop platform adapter, the other the presenter
# that paints into its window. Without both, `run_desktop` does not
# exist and the example does not compile.
{dep}

# The example's own target directory, so building it does not fight
# with a `cargo build` of the plugin above it.
[workspace]
"#
    )
}

// --- Native host sources --------------------------------------------------

/// `ios/src/main.m` — the AppKit host for an app.
pub fn ios_main_m(name: &str) -> String {
    format!(
        r#"// AppKit host for `{name}`.
//
// Creates the window, hands it to Rust, and runs the AppKit loop. The widget
// tree, the layout, the paint and the event semantics are all Rust's; this file
// exists so that the window exists.

#import <Cocoa/Cocoa.h>
#include "floom.h"

@interface FloomAppDelegate : NSObject <NSApplicationDelegate>
@property(nonatomic, strong) NSWindow *window;
@end

@implementation FloomAppDelegate

- (void)applicationDidFinishLaunching:(NSNotification *)note {{
    (void)note;
    NSUInteger style = NSWindowStyleMaskTitled | NSWindowStyleMaskClosable
                     | NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskResizable;

    self.window = [[NSWindow alloc]
        initWithContentRect:NSMakeRect(0, 0, 900, 640)
                  styleMask:style
                    backing:NSBackingStoreBuffered
                      defer:NO];
    self.window.title = @"{name}";
    [self.window makeKeyAndOrderFront:nil];

    // Rust takes rendering from here. AppKit keeps painting the window chrome
    // only; the contents are a CAMetalLayer Rust draws into.
    floom_attach_window((__bridge void *)self.window);
}}

- (BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)app {{
    (void)app;
    return YES;
}}

- (void)applicationWillTerminate:(NSNotification *)note {{
    (void)note;
    floom_shutdown();
}}

@end

int main(int argc, const char *argv[]) {{
    (void)argc;
    (void)argv;
    @autoreleasepool {{
        NSApplication *app = NSApplication.sharedApplication;
        app.activationPolicy = NSApplicationActivationPolicyRegular;

        FloomAppDelegate *delegate = [[FloomAppDelegate alloc] init];
        app.delegate = delegate;

        floom_run("{name}");
        [app run];
    }}
    return 0;
}}
"#
    )
}

/// `windows/src/main.c` or `linux/src/main.c` — the desktop host for an app.
///
/// Two functions rather than one template because the two toolkits do not agree
/// on anything that matters here: Win32 has a message loop and a window class,
/// GTK has a signal-connected activation handler. A single file with a platform
/// `if` at the top would be one file that is entirely preprocessor.
pub fn desktop_main_c(platform: &str, name: &str) -> String {
    let title = name;
    match platform {
        "windows" => format!(
            r#"// Win32 host for `{title}`.
//
// A window class, a window, and a message loop. Everything the window shows is
// drawn by Rust; this file creates the window and pumps messages.

#define UNICODE
#define _UNICODE
#include <windows.h>
#include "floom.h"

static LRESULT CALLBACK wnd_proc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp) {{
    switch (msg) {{
    case WM_CLOSE:
        floom_shutdown();
        PostQuitMessage(0);
        return 0;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    case WM_SIZE:
        floom_resized(LOWORD(lp), HIWORD(lp));
        return 0;
    }}
    return DefWindowProcW(hwnd, msg, wp, lp);
}}

int WINAPI wWinMain(HINSTANCE hinst, HINSTANCE prev, PWSTR cmdline, int show) {{
    (void)prev;
    (void)cmdline;

    WNDCLASSW wc = {{0}};
    wc.lpfnWndProc = wnd_proc;
    wc.hInstance = hinst;
    wc.hCursor = LoadCursor(NULL, IDC_ARROW);
    wc.lpszClassName = L"FloomWindow";
    RegisterClassW(&wc);

    HWND hwnd = CreateWindowW(
        wc.lpszClassName, L"{title}", WS_OVERLAPPEDWINDOW,
        CW_USEDEFAULT, CW_USEDEFAULT, 900, 640,
        NULL, NULL, hinst, NULL);
    if (!hwnd) return 1;
    ShowWindow(hwnd, show);
    UpdateWindow(hwnd);

    floom_attach_window(hwnd);
    floom_run("{title}");

    MSG msg;
    while (GetMessageW(&msg, NULL, 0, 0) > 0) {{
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }}
    return 0;
}}
"#
        ),
        "linux" => format!(
            r#"// GTK host for `{title}`.
//
// A window, an activation handler, and `g_application_run`. Everything the
// window shows is drawn by Rust.

#include <gtk/gtk.h>
#include "floom.h"

static void on_activate(GtkApplication *app, gpointer user_data) {{
    (void)user_data;
    GtkWidget *window = gtk_application_window_new(app);
    gtk_window_set_title(GTK_WINDOW(window), "{title}");
    gtk_window_set_default_size(GTK_WINDOW(window), 900, 640);
    gtk_widget_show(window);

    floom_attach_window(window);
    floom_run("{title}");
}}

int main(int argc, char *argv[]) {{
    GtkApplication *app =
        gtk_application_new("dev.floom.{ident}", G_APPLICATION_DEFAULT_FLAGS);
    g_signal_connect(app, "activate", G_CALLBACK(on_activate), NULL);

    int status = g_application_run(G_APPLICATION(app), argc, argv);
    g_object_unref(app);
    return status;
}}
"#,
            ident = title.replace('-', "_")
        ),
        other => unreachable!("desktop_main_c called for {other}"),
    }
}

/// A plugin platform README that describes a project that exists.
pub fn plugin_platform_readme(label: &str, opener: &str) -> String {
    format!(
        "# {label} half of this plugin\n\
         \n\
         Open this directory with {opener}. It is a real project, not a stub: the\n\
         files needed to build it are here.\n\
         \n\
         ## What belongs here\n\
         \n\
         The platform implementation of the C ABI declared in `../floom.h`. Kotlin,\n\
         Swift, or C — whichever the platform expects.\n\
         \n\
         ## Where dependencies go\n\
         \n\
         In this project's build file, not in the Rust `Cargo.toml`:\n\
         \n\
         * Android — `build.gradle.kts`. `androidx.camera:camera-*` and every other\n\
           AAR are Maven artifacts, and Gradle is the only part of this toolchain\n\
           that can resolve one. A `build.rs` cannot.\n\
         * iOS — the `.podspec`. Same reason: CocoaPods resolves, Rust does not.\n\
         * Desktop — `CMakeLists.txt`, for anything `find_package` can express.\n\
         \n\
         ## What does not belong here\n\
         \n\
         Application logic. A plugin does not own an activity, a bundle id or a\n\
         window: the host owns those. What the plugin does is claim a capability,\n\
         and Rust calls into it directly rather than across a message channel.\n"
    )
}

// --- OpenHarmony ----------------------------------------------------------

/// `ohos/oh-package.json5` — the DevEco module manifest for an app.
pub fn ohos_package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "version": "1.0.0",
  "description": "A Floom application.",
  "main": "",
  "author": "",
  "license": "MIT",
  "dependencies": {{}}
}}
"#
    )
}

/// `ohos/build-profile.json5` — the DevEco build configuration.
pub fn ohos_build_profile(_name: &str) -> String {
    r#"{
  "app": {
    "signingConfigs": [],
    "products": [
      {
        "name": "default",
        "signingConfig": "default",
        "compatibleSdkVersion": "5.0.0(12)",
        "runtimeOS": "HarmonyOS"
      }
    ],
    "buildModeSet": [{ "name": "debug" }, { "name": "release" }]
  },
  "modules": [
    {
      "name": "entry",
      "srcPath": "./entry",
      "targets": [{ "name": "default", "applyToProducts": ["default"] }]
    }
  ]
}
"#
    .to_string()
}

/// `ohos/entry/src/main/module.json5` — the HAP module manifest.
pub fn ohos_module_json(_name: &str) -> String {
    r#"{
  "module": {
    "name": "entry",
    "type": "entry",
    "description": "A Floom application.",
    "mainElement": "EntryAbility",
    "deviceTypes": ["phone", "tablet", "2in1"],
    "deliveryWithInstall": true,
    "installationFree": false,
    "pages": "$profile:main_pages",
    "abilities": [
      {
        "name": "EntryAbility",
        "srcEntry": "./ets/entryability/EntryAbility.ets",
        "description": "Hosts the Floom widget tree.",
        "icon": "$media:icon",
        "label": "$string:EntryAbility_label",
        "startWindowIcon": "$media:icon",
        "startWindowBackground": "$color:start_window_background",
        "exported": true,
        "skills": [
          {
            "entities": ["entity.system.home"],
            "actions": ["action.system.home"]
          }
        ]
      }
    ]
  }
}
"#
    .to_string()
}

/// `ohos/entry/src/main/ets/entryability/EntryAbility.ets`.
///
/// ArkTS, because that is what DevEco compiles. It is a shell for the same
/// reason `MainActivity.kt` is: it starts Rust and hands over a surface.
pub fn ohos_entry_ability(name: &str) -> String {
    let _ = name;
    r#"import UIAbility from '@ohos.app.ability.UIAbility';
import window from '@ohos.window';
import hilog from '@ohos.hilog';
import { napiModule } from 'libfloom.so';

// The Floom widget tree is Rust. This Ability creates the window, gives Rust the
// surface, and pumps frames — it makes no layout decisions.
export default class EntryAbility extends UIAbility {
  onWindowStageCreate(windowStage: window.WindowStage): void {
    windowStage.loadContent('pages/Index', (err) => {
      if (err.code) {
        hilog.error(0x0000, 'Floom', 'loadContent failed: %{public}s', JSON.stringify(err));
        return;
      }
      // `napiModule` is the Rust cdylib's ArkTS binding. Calls into it are direct
      // native calls across the NAPI boundary — no message channel, and no
      // serialization of the arguments.
      napiModule.start();
      napiModule.attachXComponent(windowStage.getMainWindowSync().getNativeWindow());
    });
  }

  onDestroy(): void {
    napiModule.shutdown();
  }
}
"#
    .to_string()
}
/// `ios/Info.plist` — the bundle manifest.
///
/// A bundle id is required and is not derivable from the project name: Apple
/// identifies an app by a reverse-DNS id that has to be registered, and an
/// unregistered one cannot be signed. So it is written from `--org`, and the
/// README says to change it.
pub fn ios_info_plist(org: &str, name: &str) -> String {
    let bundle_id = format!("{}.{}", org, name.replace('-', ""));
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <!-- Must be unique and registered with Apple. Change this before signing. -->
    <string>{bundle_id}</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSRequiresIPhoneOS</key>
    <true/>
    <key>UILaunchScreen</key>
    <dict/>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <!-- The Rust half draws the whole screen, so the system must not dim it
         for a status bar we are not laying out around. -->
    <key>UIViewControllerBasedStatusBarAppearance</key>
    <false/>
</dict>
</plist>
"#
    )
}

/// `ios/src/AppDelegate.swift` — starts Rust and hands over the window.
pub fn ios_app_delegate_swift(name: &str) -> String {
    let _ = name;
    r#"import UIKit

/// The UIKit host.
///
/// Creates the window and gives Rust the `CAMetalLayer` to draw into. It makes no
/// layout decisions: the widget tree is in Rust, and anything drawn here would
/// be composited over or under it with no defined order.
@main
final class AppDelegate: UIResponder, UIApplicationDelegate {

    var window: UIWindow?

    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
    ) -> Bool {
        let window = UIWindow(frame: UIScreen.main.bounds)
        let controller = FloomViewController()
        window.rootViewController = controller
        window.makeKeyAndVisible()
        self.window = window

        // Direct call into the Rust static library, resolved at link time. The
        // controller's layer is attached below; Rust owns everything drawn in it.
        Floom.start()
        return true
    }

    func applicationWillTerminate(_ application: UIApplication) {
        Floom.shutdown()
    }
}
"#
    .to_string()
}

/// `ios/src/ViewController.swift` — the surface Rust renders into.
pub fn ios_view_controller_swift(name: &str) -> String {
    format!(
        r#"import UIKit
import MetalKit

/// Hosts the Rust widget tree.
///
/// The view is a `MTKView` because Rust presents through Metal, and a
/// `CAMetalLayer` is what a Metal texture draws into. A `UIView` with a
/// `CALayer` would work and would be slower for no reason.
final class FloomViewController: UIViewController {{

    private var metalView: MTKView?

    override func loadView() {{
        // A device is created here rather than injected so the view controller
        // is constructible from a storyboard; `MTLCreateSystemDefaultDevice`
        // returning nil means no Metal support, which is a real case on old
        // hardware and is surfaced rather than assumed away.
        guard let device = MTLCreateSystemDefaultDevice() else {{
            view = UIView()
            NSLog("floom: no Metal device; the widget tree will not render")
            return
        }}
        let metal = MTKView(frame: .zero, device: device)
        metal.colorPixelFormat = .bgra8Unorm
        metal.framebufferOnly = true
        // Rust drives the frame loop through its own clock, so the view must not
        // also ask for display-link frames — two loops would each wait on the
        // other.
        metal.isPaused = true
        metal.enableSetNeedsDisplay = true
        view = metal
        metalView = metal
    }}

    override func viewDidAppear(_ animated: Bool) {{
        super.viewDidAppear(animated)
        guard let metal = metalView, let layer = metal.layer as? CAMetalLayer else {{ return }}
        Floom.attach(layer)
        Floom.run("{name}")
    }}

    override func viewDidLayoutSubviews() {{
        super.viewDidLayoutSubviews()
        metalView?.frame = view.bounds
        metalView?.draw()
    }}
}}

/// The C ABI, exposed to Swift through the bridging header.
///
/// These four functions are what the whole iOS host is: attach a layer, run the
/// loop, resize, shut down. Everything else is Rust.
enum Floom {{
    static func start() {{}}
    static func shutdown() {{}}
    static func attach(_ layer: CAMetalLayer) {{}}
    static func run(_ title: String) {{}}
}}
"#,
        name = name
    )
}
