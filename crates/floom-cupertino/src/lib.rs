//! Cupertino: the iOS-adaptive design system.
//!
//! This is not "Material with different hex codes". Three of its rules are
//! genuinely different in kind, and each is asserted in this crate's tests:
//!
//! 1. **Touch, not pointing.** iOS does not have a hover state, so `Hovered`
//!    resolves exactly like `Rest`. A desktop-hover treatment here would be a
//!    bug, not a feature.
//! 2. **No focus ring on controls.** Keyboard focus on iOS is expressed by
//!    selection, not by a ring around a button, so `Focused` also resolves like
//!    `Rest`.
//! 3. **No shadows.** Depth is expressed with grouped backgrounds and
//!    separators. Every variant reports `elevation == 0`, including the one
//!    standing in for an elevated control.
//!
//! What it does share with Material is the *shape of the work*: a token table,
//! a variant table, and a gallery — which is what [`floem_design`] exists for.

mod button;
mod gallery;
mod tokens;

pub use button::{
    ButtonMetrics, HIG_MIN_TOUCH_TARGET, button_metrics, button_style, button_styles, touch_target,
};
pub use gallery::{GALLERY_COLUMNS, button_gallery, button_gallery_labels};
pub use tokens::{CupertinoColors, CupertinoTheme};

pub use floem_design::{ControlSize, InteractionState, Variant};
pub use floem_theme::ColorScheme;
