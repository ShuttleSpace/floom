//! Build script for the `webview` plugin.
//!
//! Compiles the native half and links it into this crate. The native code is
//! compiled *into* the crate, not shipped beside it: there is no runtime
//! plugin discovery and therefore no serialization boundary — a call from the
//! app is a machine call, resolved when this crate is linked.
//!
//! Each platform's half lives in its own peer directory under `native/` —
//! `native/macos/` (Objective-C), `native/linux/` (C11 + GTK) — and
//! `native/common/` holds the C ABI header both of them implement, so one
//! contract cannot drift from the other.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=native");

    println!("cargo:rustc-check-cfg=cfg(floom_plugin_native)");
    // Platform/backend cfgs this crate will use on not-yet-built targets.
    println!("cargo:rustc-check-cfg=cfg(target_os, values(\"ohos\"))");
    println!("cargo:rustc-check-cfg=cfg(feature, values(\"tui\"))");

    let manifest = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by cargo"),
    );
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let common = manifest.join("native").join("common");

    // The provider ABI comes from the crate that owns it, so the two cannot
    // drift. The path is relative because the header is not a dependency:
    // `floom-native-ffi` is a Rust dependency, and a Rust dependency does
    // not carry its headers.
    let provider_header = manifest
        .join("..")
        .join("..")
        .join("..")
        .join("native")
        .join("floom-native-ffi")
        .join("include")
        .join("floom_native_provider.h");
    println!("cargo:rerun-if-changed={}", provider_header.display());
    let copied = out_dir.join("floom_native_provider.h");
    std::fs::copy(&provider_header, &copied)
        .unwrap_or_else(|e| panic!("copying {}: {e}", provider_header.display()));

    match target_os.as_str() {
        "macos" => {
            let source = manifest
                .join("native")
                .join("macos")
                .join("floom_webview.m");
            println!("cargo:rerun-if-changed={}", source.display());

            cc::Build::new()
                .file(&source)
                .include(&common)
                .include(&out_dir)
                .flag_if_supported("-fobjc-arc")
                .warnings(true)
                .compile("floom_webview");

            // AppKit and Foundation: the frameworks the native half reaches. WebKit
            // because a `WKWebView` comes from it — the capability this plugin now
            // really provides, rather than a blank `NSView` standing in for one.
            println!("cargo:rustc-link-lib=framework=AppKit");
            println!("cargo:rustc-link-lib=framework=Foundation");
            println!("cargo:rustc-link-lib=framework=WebKit");
        }
        "ios" => {
            let source = manifest.join("native").join("ios").join("floom_webview.m");
            println!("cargo:rerun-if-changed={}", source.display());

            cc::Build::new()
                .file(&source)
                .include(&common)
                .include(&out_dir)
                .flag_if_supported("-fobjc-arc")
                .warnings(true)
                .compile("floom_webview");

            // UIKit is the view hierarchy, WebKit the `WKWebView` the half
            // builds into it. The frameworks are iOS set names, not macOS's.
            println!("cargo:rustc-link-lib=framework=UIKit");
            println!("cargo:rustc-link-lib=framework=Foundation");
            println!("cargo:rustc-link-lib=framework=WebKit");
        }
        "linux" => {
            let source = manifest
                .join("native")
                .join("linux")
                .join("floom_webview.c");
            println!("cargo:rerun-if-changed={}", source.display());

            let mut build = cc::Build::new();
            build
                .file(&source)
                .include(&common)
                .include(&out_dir)
                .warnings(true);

            // GTK3 + WebKit2GTK 4.1 are what the Linux half reaches. Their
            // headers live in a versioned include dir
            // (`/usr/include/webkitgtk-4.1`), which a bare `cc` does not
            // search, so pkg-config supplies both the include path and the two
            // `-l` flags. A host without them gets one sentence naming the
            // packages, not a wall of missing-header errors.
            // `javascriptcoregtk-4.1` is not a dependency of webkit2gtk-4.1's
            // own `--libs`, but the file reaches `jsc_value_*` through the
            // webkit2 header, so it has to be asked for by name.
            let pkgs = ["gtk+-3.0", "webkit2gtk-4.1", "javascriptcoregtk-4.1"];
            for pkg in pkgs {
                for dir in pkg_config_includes(pkg) {
                    build.include(dir);
                }
            }
            build.compile("floom_webview");

            // One flag per library the three packages bring, rather than a
            // hand-kept list: the previous list was missing gobject-2.0 and
            // javascriptcoregtk-4.1, which is a link error, not a warning.
            let mut seen: Vec<String> = Vec::new();
            for pkg in pkgs {
                for lib in pkg_config_libs(pkg) {
                    if seen.contains(&lib) {
                        continue;
                    }
                    println!("cargo:rustc-link-lib=dylib={lib}");
                    seen.push(lib);
                }
            }
        }
        "windows" => {
            let source = manifest
                .join("native")
                .join("windows")
                .join("floom_webview.cpp");
            println!("cargo:rerun-if-changed={}", source.display());

            cc::Build::new()
                .file(&source)
                .include(&common)
                .include(&out_dir)
                .warnings(true)
                .compile("floom_webview");

            // Win32 UI plumbing plus WebView2; the loader static lib is the
            // WebView2 NuGet piece, resolved by the host's own build.
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=ole32");
            println!("cargo:rustc-link-lib=WebView2LoaderStatic");
        }
        other => {
            // No compiled native half on this target (ohos, or a host os we
            // have not grown one for; web and tui never get one). The plugin
            // still compiles, and `enable()` reports the absence honestly
            // rather than pretending.
            println!(
                "cargo:warning=floom-plugin-webview: no compiled native half for target `{other}`; \
                 enable() and provider() report it"
            );
        }
    }

    println!("cargo:rustc-cfg=floom_plugin_native");
}

/// The include directories `pkg-config --cflags <pkg>` reports.
///
/// Only the `-I` entries matter here: `cc` compiles one `.c` and the linker
/// flags are printed above. A missing package is a build failure with one
/// readable sentence, which is the most useful thing a build script can say at
/// that point. Defined on every host because `match` arms are type-checked
/// everywhere even when only one of them runs; it is only ever *called* for the
/// Linux target.
fn pkg_config_includes(pkg: &str) -> Vec<String> {
    let output = std::process::Command::new("pkg-config")
        .args(["--cflags", pkg])
        .output()
        .unwrap_or_else(|e| {
            panic!("floom-plugin-webview: `pkg-config --cflags {pkg}` did not run ({e}); install pkg-config to build the Linux half")
        });
    if !output.status.success() {
        panic!(
            "floom-plugin-webview: {pkg} is not installed -- apt install libgtk-3-dev libwebkit2gtk-4.1-dev (the Linux half needs GTK3 and WebKit2GTK 4.1)"
        );
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|flag| flag.strip_prefix("-I").map(str::to_string))
        .collect()
}

/// The library names `pkg-config --libs <pkg>` reports.
fn pkg_config_libs(pkg: &str) -> Vec<String> {
    let output = std::process::Command::new("pkg-config")
        .args(["--libs", pkg])
        .output()
        .unwrap_or_else(|e| {
            panic!("floom-plugin-webview: `pkg-config --libs {pkg}` did not run ({e})")
        });
    if !output.status.success() {
        panic!("floom-plugin-webview: {pkg} is not installed -- see pkg_config_includes above")
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|flag| flag.strip_prefix("-l").map(str::to_string))
        .collect()
}
