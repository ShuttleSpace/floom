//! Material 3 tokens and control variants.
//!
//! Two palettes ship here and they are not the same thing:
//!
//! * [`MaterialTheme::light`] / [`MaterialTheme::dark`] are the published
//!   Material 3 baseline scheme — specific hex values chosen by Material's own
//!   process, reproduced so a Floem app looks like Material rather than *like a
//!   ramp seeded with Material's purple*.
//! * [`MaterialTheme::from_seed`] derives a scheme from any seed colour using
//!   [`floem_design::tonal_ramp`]. Its on-colours are chosen by measured
//!   contrast instead of being copied, because the baseline's on-colours were
//!   chosen against the baseline's primary and do not transfer.
//!
//! Keeping both is the point: the static palette proves the implementation can
//! express a real design system, and the derived one proves the plumbing works
//! for a seed nobody hand-tuned.

mod button;
mod gallery;
mod tokens;

pub use button::{ButtonMetrics, button_metrics, button_style, button_styles};
pub use gallery::{GALLERY_COLUMNS, button_gallery, button_gallery_labels};
pub use tokens::{MaterialRoles, MaterialTheme};

pub use floem_design::{ControlSize, InteractionState, Variant};
pub use floem_theme::ColorScheme;
