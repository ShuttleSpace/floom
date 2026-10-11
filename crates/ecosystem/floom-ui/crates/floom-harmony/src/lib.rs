//! HarmonyOS design tokens and capsule control variants.
//!
//! # Licensing boundary
//!
//! Everything here is written from publicly documented HarmonyOS design
//! guidance — the colour roles, the 8vp spacing scale, the 40vp default control
//! height, the capsule corner rule and the interaction-state opacities. **No
//! proprietary assets are included or reproduced**: no fonts, no icons, no
//! vendor SDK, no bitmap. Type is requested by generic family name and colour
//! values are plain sRGB. This is not an official vendor implementation and
//! makes no claim to be one; it is a token table that follows published
//! guidance, which is the extent of what a redistributable crate may do.
//!
//! # What makes it distinct
//!
//! A capsule corner is *derived*, not chosen: the radius is half the control's
//! own height, so the same variant is a different shape at each size step. No
//! other system in this workspace has a size-dependent radius, and the tests
//! pin it.

mod button;
mod gallery;
mod tokens;

pub use button::{ButtonMetrics, button_metrics, button_style, button_styles};
pub use gallery::{GALLERY_COLUMNS, button_gallery, button_gallery_labels};
pub use tokens::{HarmonyColors, HarmonyTheme};

pub use floom_design::{ControlSize, InteractionState, Variant};
pub use floom_theme::ColorScheme;
