//! Shared Rust for every platform half of the camera plugin.

/// The capability id this plugin reports. Stable across releases; the host
/// reads it through [`crate::capability`].
pub const CAPABILITY: i32 = 0;

/// The name the host looks the plugin up by — the crate name without the
/// `floom-plugin-` prefix.
pub const PLUGIN_NAME: &str = "camera";
