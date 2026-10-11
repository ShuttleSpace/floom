//! The spine every design system in `design-systems/` shares.
//!
//! A design system is three things: a token table, a mapping from *control
//! variants* to style properties, and the behaviour that distinguishes the
//! variants at interaction time. Only the first is specific to Material,
//! Cupertino, Fluent or Harmony — the vocabulary ([`Variant`], [`ControlSize`],
//! [`InteractionState`]), the colour arithmetic ([`contrast_ratio`],
//! [`tonal_ramp`]) and the gallery fixture ([`gallery_scene`]) are identical in
//! all four cases.
//!
//! They live here rather than being copied per system because four copies of a
//! variant table drift: one system learns a `Disabled` rule the others do not,
//! and the difference is invisible until someone renders them side by side.
//! That is the same failure this workspace already hit with a duplicated
//! accessibility role table.
//!
//! What this crate deliberately does *not* do is claim more colour science than
//! it implements — see [`tonal_ramp`] for exactly which derivation is used.

mod gallery;
mod style;
mod variant;

// The colour maths now lives in `floom-types`, and is re-exported here so that
// the design systems keep importing it from the crate they already depend on.
//
// The reason it moved: `floom-accuracy` needs `contrast_ratio` to check whether
// a rendered frame meets WCAG, and reaching into this crate for it would have
// meant depending on every design system to evaluate one sRGB formula. See
// `floom_types::color_math` for why it is not a design-system concern.
pub use floom_types::color_math::{
    TonalPalette, WCAG_AA_LARGE, WCAG_AA_NORMAL, WCAG_AAA_NORMAL, composite_over, contrast_ratio,
    lerp, meets_contrast, most_contrasting, readable_on, relative_luminance, tonal_ramp,
    with_alpha,
};
pub use gallery::{
    CELL_H, CELL_W, CellPaint, DepthField, FULL_GRID_COLUMNS, GAP, GallerySample, SHADOW_ALPHA,
    button_gallery_labels, button_gallery_scene, gallery_scene, gallery_size,
};
pub use style::{Props, StyleSetBuilder, StyleSetExt};
pub use variant::{ControlSize, InteractionState, Variant};
