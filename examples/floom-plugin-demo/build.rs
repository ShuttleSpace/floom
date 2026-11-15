//! Build script for the `sample` plugin.
//!
//! Compiles the native half for the current target and links it into this
//! crate. The native half implements the contract in
//! `native/common/floom_sample.h`; the Rust half in `src/lib.rs` calls it
//! through `extern "C"`.
//!
//! There is no runtime plugin discovery and therefore no serialization
//! boundary: a call from the app is a machine call, resolved when this crate
//! is linked.

use std::path::PathBuf;

fn main() {
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
    if target_os == "android" {
        return;
    }

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
    let platform_dir = if target_arch == "wasm32" {
        "web"
    } else {
        match (target_os.as_str(), target_env.as_str()) {
            ("linux", "ohos") => "ohos",
            ("ios", _) => "ios",
            ("linux", _) | ("macos", _) | ("windows", _) => "desktop",
            _ => "",
        }
    };

    if platform_dir.is_empty() {
        println!(
            "cargo:warning=sample: no native half for target_os={target_os}; \
             the plugin compiles but enable() will report it"
        );
        return;
    }

    let native_dir = manifest.join("native").join(platform_dir);
    if !native_dir.is_dir() {
        println!(
            "cargo:warning=sample: native/{platform_dir} does not exist; \
             the plugin compiles but enable() will report it"
        );
        return;
    }

    // The C sources for this platform. A platform whose native half is not C
    // (iOS is Swift) has none, and is handled below.
    let mut sources: Vec<PathBuf> = std::fs::read_dir(&native_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "c"))
                .collect()
        })
        .unwrap_or_default();
    sources.sort();

    if !sources.is_empty() {
        cc::Build::new()
            .files(&sources)
            .include(manifest.join("native").join("common"))
            .include(&native_dir)
            .flag_if_supported("-std=c11")
            .warnings(true)
            .compile("floom_sample");
        println!("cargo:rustc-cfg=floom_plugin_native");
        return;
    }

    // The web's native half is JavaScript, imported by the wasm module, so
    // there is nothing to compile here: the module is bundled by the app's
    // build (wasm-bindgen or a bundler), and the Rust side declares the
    // imports by module name.
    if platform_dir == "web" {
        println!("cargo:rustc-cfg=floom_plugin_native");
        println!(
            "cargo:warning=sample: the web half is JavaScript; make sure the \
             bundler picks up native/web/js/floom_sample.js"
        );
        return;
    }

    // OpenHarmony's native half is ArkTS, reached over N-API. The four ABI
    // functions are implemented on the Rust side for that platform, because
    // the N-API environment lives in Rust and a C shim cannot obtain one --
    // so there is nothing to compile here either.
    if platform_dir == "ohos" {
        println!("cargo:rustc-cfg=floom_plugin_native");
        println!(
            "cargo:warning=sample: the OpenHarmony half is ArkTS over N-API; the \
             ArkTS side must call register_bridge at startup"
        );
        return;
    }

    // iOS: the native half is Swift, so `cc` cannot compile it. It is built by
    // `xcodebuild` into an xcframework, which this script links when it is
    // there.
    if target_os == "ios" {
        let framework = native_dir.join("libfloom_sample.xcframework");
        if framework.is_dir() {
            println!("cargo:rustc-link-search=native/ios/libfloom_sample.xcframework");
            println!("cargo:rustc-link-lib=static=floom_sample");
            println!("cargo:rustc-cfg=floom_plugin_native");
        } else {
            println!(
                "cargo:warning=sample: native/ios/libfloom_sample.xcframework not found; \
                 build it with xcodebuild first (see native/ios/README.md). \
                 The plugin compiles but enable() will report it"
            );
        }
        return;
    }

    println!(
        "cargo:warning=sample: native/{platform_dir} has no .c source; \
         the plugin compiles but enable() will report it"
    );
}
