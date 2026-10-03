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