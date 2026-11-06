//! Build script for the `location` plugin.
//!
//! Compiles the native half for the current target and links it into this
//! crate. The native half implements the contract in
//! `native/common/floom_location.h`; the Rust half in `src/lib.rs` calls it
//! through `extern "C"`.
//!
//! This plugin's target arms are Rust modules, so `native/<platform>/` holds
//! no C source today and this script compiles nothing. It still emits the
//! `floom_plugin_native` check-cfg and watches `native/`, so adding a C half
//! later is a matter of dropping the source in — the Rust side already gates
//! its `extern` block on the flag.

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

    // Android's half is Java, compiled into the app's dex and reached over
    // JNI — there is no C source to look for.
    if target_os == "android" {
        return;
    }

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
        return;
    }

    let native_dir = manifest.join("native").join(platform_dir);
    if !native_dir.is_dir() {
        return;
    }

    let sources: Vec<PathBuf> = std::fs::read_dir(&native_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "c"))
                .collect()
        })
        .unwrap_or_default();

    if sources.is_empty() {
        return;
    }

    cc::Build::new()
        .files(&sources)
        .include(manifest.join("native").join("common"))
        .include(&native_dir)
        .flag_if_supported("-std=c11")
        .warnings(true)
        .compile("floom_location");
    println!("cargo:rustc-cfg=floom_plugin_native");
}
