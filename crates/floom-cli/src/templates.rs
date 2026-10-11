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
        .map(|f| format!("\"{f}\"") )
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
/// `register` puts `enable` into [`floom_platform`]'s table, and it is what a
/// caller invokes before `plugin("name").enable()`. It is
/// a plain function rather than a closure because a plugin is a link-time
/// dependency in Floom, not a runtime-discovered one: by the time the app runs,
/// `enable` is already in the binary.
pub fn plugin_lib_rs(name: &str) -> String {
    format!(
        r#"//! {name} — a Floom plugin.
//!
//! The public surface is three functions and a capability. A caller does:
//!
//! ```ignore
//! {name}::register();
//! floom_platform::plugin("{name}").enable()?;
//! let version = {name}::version();
//! ```
//!
//! Everything below would normally be an `extern "C"` call into `native/`. It is
//! stubbed so that `floom create --plugin` produces a crate that compiles and
//! tests on a machine with no Android SDK, no Xcode and no NDK — the same reason
//! Flutter emits no Gradle project for platforms it was not asked for.

/// The plugin's version, as reported by the native layer.
///
/// Statically known until `native/` is wired up. `unimplemented!` would be a
/// lie in a template: the app that runs `floom create --plugin` and prints this
/// should see a number.
pub fn version() -> &'static str {{
    env!("CARGO_PKG_VERSION")
}}

/// Claims the plugin's platform state.
///
/// Called once by the host, through [`floom_platform::register_plugin`]. A real
/// implementation forwards to the native registration:
/// ```ignore
/// // SAFETY: called on the host thread during startup, before any frame.
/// let rc = unsafe {{ {name}_ffi::floom_{name}_enable() }};
/// if rc != 0 {{
///     return Err("native enable failed".to_string());
/// }}
/// Ok(())
/// ```
///
/// Returns the plugin's own message on failure, not a wrapped type: the
/// registry puts this string in [`floom_platform::PluginError::Failed`] and a
/// wrapper here would only be stripped again.
pub fn enable() -> Result<(), String> {{
    Ok(())
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
    fn version_is_the_crate_version() {{
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }}

    #[test]
    fn enable_is_idempotent() {{
        // Two enables must both succeed: a host may enable every plugin at
        // startup and an app may also enable one on demand, and a plugin that
        // rejected the second call would turn that into a crash.
        enable().unwrap();
        enable().unwrap();
    }}

    #[test]
    fn registering_makes_the_plugin_findable() {{
        register();
        let handle = floom_platform::plugin("{name}");
        assert!(handle.is_registered());
        assert!(!handle.is_enabled());
        handle.enable().unwrap();
        assert!(handle.is_enabled());
    }}
}}
"#,
        name = name
    )
}

/// A plugin's `Cargo.toml`.
///
/// Two things differ from `cargo_toml`, and both are the point of a plugin:
///
/// * it depends on `floom-platform` for `Capability` and `PluginError`, not on
///   the `floom` facade, because a plugin must not pull in the whole widget
///   stack;
/// * `build.rs` is declared, because the native half is compiled at build time.
pub fn plugin_cargo_toml(name: &str, floom_path: Option<&str>) -> String {
    let dep = match floom_path {
        Some(p) => format!(
            "floom-platform = {{ path = \"{p}\" }}\n\
             # Re-point at the published crate once it ships:\n\
             # floom-platform = {{ version = \"0.2\" }}"
        ),
        None => "floom-platform = { version = \"0.2\" }".to_string(),
    };
    format!(
        "[package]\n\
         name = \"floom-plugin-{name}\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         description = \"A Floom plugin: {name}\"\n\
         build = \"build.rs\"\n\
         \n\
         [dependencies]\n\
         {dep}\n\
         \n\
         # Uncomment once native/ is populated. `bindgen` runs on the C header\n\
         # in native/common, and the generated module is what the `extern \"C\"`\n\
         # calls in src/lib.rs go through.\n\
         # [build-dependencies]\n\
         # bindgen = \"0.71\"\n\
         \n\
         [dev-dependencies]\n\
         # A plugin is tested without a device: the pure-Rust half is an ordinary\n\
         # crate, so `cargo test` covers it and the native half is covered by the\n\
         # example app.\n"
    )
}

/// `build.rs` for a plugin.
///
/// Deliberately a no-op rather than an absent file: the manifest declares
/// `build = "build.rs"`, so the file has to exist, and a no-op build script that
/// prints what it *would* do is more useful to whoever opens the generated
/// project than an empty file.
pub fn plugin_build_rs(name: &str) -> String {
    format!(
        r#"//! Build script for the `{name}` plugin.
//!
//! Currently a no-op: `native/` is not populated yet, so there is nothing to
//! compile and nothing to link. When it is, this is where each platform is
//! compiled and linked into the same binary as the Rust half.

fn main() {{
    // The header `bindgen` would read, once native/common/floom_{name}.h exists.
    println!("cargo:rerun-if-changed=native/common/floom_{name}.h");

    // Per platform, once the native sources land:
    //
    //   android/  -> `cargo-ndk` or a `cmake` invocation building a `.so` for
    //                each ABI; `println!("cargo:rustc-link-lib=...")`.
    //   ios/      -> `xcodebuild -create-xcframework`, linked as a framework.
    //   ohos/     -> the OHOS NDK, same shape as android.
    //
    // The native code is compiled *into this crate*, not shipped beside it.
    // There is no runtime plugin discovery and therefore no serialization
    // boundary: `src/lib.rs` calls `extern "C"` directly.
}}
"#
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
pub fn plugin_header_rs(name: &str) -> String {
    let ident = name.replace('-', "_");
    format!(
        r#"/* {name} — the native/Rust contract.
 *
 * Everything here is a plain C ABI: no serialization, no runtime lookup, no
 * registry. The Rust half calls these symbols directly, and they resolve when
 * the crate is linked.
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
        Some(p) => format!("{f} = {{ path = \"{p}\" }}", f = floom_crate),
        None => format!("{f} = {{ version = \"0.2\" }}", f = floom_crate),
    };
    let platform_dep = match platform_path {
        Some(p) => format!("floom-platform = {{ path = \"{p}\" }}"),
        None => "floom-platform = { version = \"0.2\" }".to_string(),
    };
    format!(
        "[package]\n\
         name = \"floom-plugin-{name}-example\"\n\
         version = \"0.1.0\"\n\
         edition = \"2024\"\n\
         publish = false\n\
         \n\
         [dependencies]\n\
         floom-plugin-{name} = {{ path = \"..\" }}\n\
         # The registry the plugin registers itself into. The example depends on
         # it directly because it calls `plugin(\"{name}\").enable()`, which a
\
         # plugin's own re-exports would otherwise hide.
\
         {platform_dep}\n\
         {dep}\n\
         \n\
         # The example's own target directory, so building it does not fight\n\
         # with a `cargo build` of the plugin above it.\n\
         [workspace]\n"
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
    format!(
        r#"{{
  "app": {{
    "signingConfigs": [],
    "products": [
      {{
        "name": "default",
        "signingConfig": "default",
        "compatibleSdkVersion": "5.0.0(12)",
        "runtimeOS": "HarmonyOS"
      }}
    ],
    "buildModeSet": [{{ "name": "debug" }}, {{ "name": "release" }}]
  }},
  "modules": [
    {{
      "name": "entry",
      "srcPath": "./entry",
      "targets": [{{ "name": "default", "applyToProducts": ["default"] }}]
    }}
  ]
}}
"#,
    )
}

/// `ohos/entry/src/main/module.json5` — the HAP module manifest.
pub fn ohos_module_json(_name: &str) -> String {
    format!(
        r#"{{
  "module": {{
    "name": "entry",
    "type": "entry",
    "description": "A Floom application.",
    "mainElement": "EntryAbility",
    "deviceTypes": ["phone", "tablet", "2in1"],
    "deliveryWithInstall": true,
    "installationFree": false,
    "pages": "$profile:main_pages",
    "abilities": [
      {{
        "name": "EntryAbility",
        "srcEntry": "./ets/entryability/EntryAbility.ets",
        "description": "Hosts the Floom widget tree.",
        "icon": "$media:icon",
        "label": "$string:EntryAbility_label",
        "startWindowIcon": "$media:icon",
        "startWindowBackground": "$color:start_window_background",
        "exported": true,
        "skills": [
          {{
            "entities": ["entity.system.home"],
            "actions": ["action.system.home"]
          }}
        ]
      }}
    ]
  }}
}}
"#
    )
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
