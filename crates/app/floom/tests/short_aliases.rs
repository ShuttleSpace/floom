// Every root-workspace library crate is reachable as floom::<short name>.
// Default-feature aliases (must always resolve):
use floom::app as _;
use floom::core as _;
use floom::layout as _;
use floom::scene as _;
use floom::types as _;
use floom::widgets as _;
use floom::animation as _;
use floom::signal as _;
use floom::accessibility as _;
use floom::dsl as _;
use floom::editor as _;
use floom::editor_core as _;
use floom::inspector as _;
use floom::native as _;
use floom::native_ffi as _;
use floom::platform_mobile as _;
use floom::renderer as _;
use floom::runtime_core as _;
use floom::style as _;
use floom::text as _;
use floom::tsx as _;
use floom::r#virtual as _;
use floom::widgets_data as _;

#[test]
fn default_short_aliases_resolve() {
    // The two merged modules still expose both their hand-written API and the crate.
    let _ = floom::platform::require;
    let _: Option<floom::platform::WindowConfig> = None;
    let _: Option<floom::core::Axis> = None;
}

#[cfg(feature = "full")]
#[test]
fn full_short_aliases_resolve() {
    use floom::accuracy as _;
    use floom::snapshot as _;
    use floom::testkit as _;
    use floom::ui_events_winit as _;
}

// Feature-gated backend aliases: each mirrors its `dep:` entry in Cargo.toml.
// `--all-features` compiles this whole block, so no alias can silently rot.
#[cfg(feature = "desktop")]
use floom::platform_desktop as _;
#[cfg(feature = "mobile")]
use floom::platform_android as _;
#[cfg(feature = "web")]
use floom::platform_web as _;
#[cfg(feature = "web")]
use floom::renderer_web as _;
#[cfg(feature = "tui")]
use floom::platform_tui as _;
#[cfg(feature = "tui")]
use floom::renderer_tui as _;
#[cfg(feature = "embedded")]
use floom::platform_embedded as _;
#[cfg(feature = "ios")]
use floom::platform_ios as _;
#[cfg(feature = "harmony")]
use floom::platform_harmony as _;
#[cfg(feature = "renderer-gpu")]
use floom::renderer_vello as _;
#[cfg(feature = "renderer-software")]
use floom::renderer_software as _;
#[cfg(feature = "devtools")]
use floom::hot_reload as _;
