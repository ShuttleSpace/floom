//! Build script for the `camera` plugin.
//!
//! Compiles the native half and links it into this crate. The native code is
//! compiled *into* the crate, not shipped beside it: there is no runtime plugin
//! discovery and therefore no serialization boundary — a call from the app is a
//! machine call, resolved when this crate is linked.
//!
//! Each platform directory holds its own native half; only the one matching the
//! active target is compiled. `android/` is built by Gradle into an AAR and
//! reached through JNI, `web/` is pure Rust, and `tui/` is unsupported — those
//! three need no C compilation here.

use std::path::PathBuf;

/// Compile `file` from `dir` with the given compiler flags, and link the listed
/// Apple frameworks (ignored on non-Apple targets).
fn compile_native(manifest: &PathBuf, dir: &str, file: &str, flags: &[&str], frameworks: &[&str]) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));

    // The provider ABI header comes from the crate that owns it, so the two
    // cannot drift. It is copied next to the build output and added to the
    // include path.
    let provider_header = manifest
        .join("..")
        .join("..")
        .join("..")
        .join("floom-native-ffi")
        .join("include")
        .join("floom_native_provider.h");
    println!("cargo:rerun-if-changed={}", provider_header.display());
    let copied = out_dir.join("floom_native_provider.h");
    std::fs::copy(&provider_header, &copied)
        .unwrap_or_else(|e| panic!("copying {}: {e}", provider_header.display()));

    let source = manifest.join(dir).join(file);
    println!("cargo:rerun-if-changed={}", source.display());

    let mut build = cc::Build::new();
    build
        .file(&source)
        .include(manifest.join("common"))
        .include(&out_dir);
    for flag in flags {
        build.flag_if_supported(flag);
    }
    build.warnings(true).compile("floom_camera");

    // Apple frameworks are reached by name; other platforms link their own SDKs
    // through the compiler/linker invoked by `cc`.
    for framework in frameworks {
        println!("cargo:rustc-link-lib=framework={framework}");
    }

    println!("cargo:rustc-cfg=floom_plugin_native");
}

fn main() {
    println!("cargo:rerun-if-changed=common");
    println!("cargo:rustc-check-cfg=cfg(floom_plugin_native)");
    // OpenHarmony is `target_os = "linux"` with `target_env = "ohos"` at the
    // Rust level, but the plugin layer names the platform by the value the
    // OpenHarmony NDK reports, so the compiler must be told about it.
    println!("cargo:rustc-check-cfg=cfg(target_os, values(\"ohos\"))");

    let manifest = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"),
    );
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    match target_os.as_str() {
        "macos" => compile_native(
            &manifest,
            "macos",
            "floom_camera.m",
            &["-fobjc-arc"],
            &["AppKit", "Foundation", "AVFoundation"],
        ),
        "ios" => compile_native(
            &manifest,
            "ios",
            "floom_camera.m",
            &["-fobjc-arc"],
            &["UIKit", "Foundation", "AVFoundation"],
        ),
        "windows" => compile_native(
            &manifest,
            "windows",
            "floom_camera.cpp",
            &["/std:c++17"],
            &[],
        ),
        "linux" => compile_native(&manifest, "linux", "floom_camera.c", &[], &[]),
        "ohos" => compile_native(&manifest, "ohos", "floom_camera.cpp", &["-std=c++17"], &[]),
        // `android` builds its Java half via Gradle; `web` is pure Rust; `tui`
        // is unsupported. None of them compile a C native half here.
        _ => {
            println!(
                "cargo:warning=floom-plugin-camera: no C native half for target_os={target_os}; \
                 the plugin compiles but enable() reports it"
            );
        }
    }
}
