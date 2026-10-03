//! Fluent: the Windows desktop design system.
//!
//! Fluent is the counterweight to [`floem_cupertino`]. Same vocabulary, three
//! opposite answers, each asserted here:
//!
//! 1. **Pointing, not touch.** A mouse exists, so `Hovered` *must* differ from
//!    `Rest`. Sharing Cupertino's "hover is a no-op" rule would be the bug.
//! 2. **A visible keyboard focus ring.** Desktop keyboard navigation is
//!    first-class, so `Focused` draws a ring, and the ring has to be visible
//!    against the fill it surrounds.
//! 3. **A shadow ladder exists.** Depth is a real axis, and the raised variant
//!    casts one — unlike Cupertino, which nests corners instead.
//!
//! Its neutral ramp is the other Fluent signature: surfaces are layers
//! (`Layer`, `LayerAlt`, `CardBackground`) rather than one background plus
//! translucency.

mod button;
mod gallery;
mod tokens;

pub use button::{ButtonMetrics, button_metrics, button_style, button_styles};
pub use gallery::{GALLERY_COLUMNS, button_gallery, button_gallery_labels};
pub use tokens::{FluentColors, FluentTheme};

pub use floem_design::{ControlSize, InteractionState, Variant};
pub use floem_theme::ColorScheme;
