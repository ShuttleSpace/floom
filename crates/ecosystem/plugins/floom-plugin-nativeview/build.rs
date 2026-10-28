//! Build script for the `nativeview` plugin.
//!
//! Compiles the native half and links it into this crate. The native code is
//! compiled *into* the crate, not shipped beside it: there is no runtime
//! plugin discovery and therefore no serialization boundary — a call from the
//! app is a machine call, resolved when this crate is linked.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=native");
    println!("cargo:rustc-check-cfg=cfg(floom_plugin_native)");

    let manifest = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"),
    );
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));

    // The provider ABI comes from the crate that owns it, so the two cannot
    // drift. The path is relative because the header is not a dependency:
    // `floom-native-ffi` is a Rust dependency, and a Rust dependency does
    // not carry its headers.
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

    if target_os != "macos" {
        // The native half is Objective-C and reaches AppKit, so it exists on
        // macOS only. On every other target the plugin has no native half,
        // and says so rather than pretending.
        println!(
            "cargo:warning=floom-plugin-nativeview: the native half is \
             Objective-C and exists on macOS only; the plugin compiles but \
             enable() will report it"
        );
        return;
    }

    let source = manifest
        .join("native")
        .join("macos")
        .join("floom_nativeview.m");
    println!("cargo:rerun-if-changed={}", source.display());

    cc::Build::new()
        .file(&source)
        .include(manifest.join("native").join("common"))
        .include(&out_dir)
        .flag_if_supported("-fobjc-arc")
        .warnings(true)
        .compile("floom_nativeview");

    // AppKit and Foundation: the frameworks the native half reaches. WebKit
    // because a `WKWebView` comes from it — the capability this plugin now
    // really provides, rather than a blank `NSView` standing in for one.
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=WebKit");

    println!("cargo:rustc-cfg=floom_plugin_native");
}
