//! The Cupertino component gallery: every button variant × size × state.
//!
//! Almost none of this file is Cupertino's. The sample grid, the layout and the
//! label format live in [`floom_design`] because all four design systems need
//! them and had four byte-identical copies; what remains here is the two things
//! that are actually Cupertino: which theme to read, and how a sample resolves.

use floom_design::{
    CellPaint, DepthField, button_gallery_labels as shared_labels, button_gallery_scene,
};
use floom_scene::Scene;

use crate::button::{button_metrics, button_style};
use crate::tokens::CupertinoTheme;

/// Samples per row: one row per variant, one column per size × state.
///
/// Kept as a Cupertino-side name for [`floom_design::FULL_GRID_COLUMNS`] so the
/// published `floom_cupertino::GALLERY_COLUMNS` path does not break. The value is
/// shared: a gallery that laid out at a different column count per system would be
/// a difference nobody notices until two screenshots are compared.
pub const GALLERY_COLUMNS: usize = floom_design::FULL_GRID_COLUMNS;

/// Paint the full Cupertino button grid.
pub fn button_gallery(theme: &CupertinoTheme) -> Scene {
    button_gallery_scene(theme.colors.grouped_background, |sample| {
        CellPaint::new(
            button_style(theme, sample.variant, sample.size, sample.state),
            button_metrics(theme, sample.size).height as f64,
        )
    })
}

/// A textual rendering of the same grid — one line per sample, carrying the
/// resolved properties rather than pixels.
///
/// This is the layer a reviewer can actually read in a diff: a pixel hash says
/// *something changed*, this says *the outlined hover lost its outline*.
pub fn button_gallery_labels(theme: &CupertinoTheme) -> String {
    shared_labels(
        &format!("scheme={:?} grouped_background={:?}", theme.scheme, theme.colors.grouped_background),
        // **Cupertino alone.** The other three print `elevation`; this
        // system has always printed `opacity`. Preserved, not corrected — see
        // `floom_design::DepthField`.
        DepthField::Opacity,
        |sample| button_style(theme, sample.variant, sample.size, sample.state),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_design::{ControlSize, InteractionState, Variant};

    #[test]
    fn the_column_count_is_the_full_grid() {
        assert_eq!(
            GALLERY_COLUMNS,
            ControlSize::ALL.len() * InteractionState::ALL.len(),
            "GALLERY_COLUMNS and FULL_GRID_COLUMNS must not drift apart"
        );
        assert_eq!(
            GALLERY_COLUMNS,
            floom_design::FULL_GRID_COLUMNS,
            "this alias exists to keep one number; two is one too many"
        );
    }

    #[test]
    fn every_sample_is_labelled_exactly_once() {
        let labels = button_gallery_labels(&CupertinoTheme::light());
        let expected = Variant::ALL.len() * ControlSize::ALL.len() * InteractionState::ALL.len();
        assert_eq!(
            labels.lines().count(),
            expected + 1,
            "header plus {} samples", expected
        );
    }

    #[test]
    fn the_header_names_the_page_colour() {
        let labels = button_gallery_labels(&CupertinoTheme::light());
        let header = labels.lines().next().expect("a header line");
        assert!(header.contains("scheme="), "{header}");
    }

    #[test]
    fn the_gallery_scene_is_not_empty() {
        let scene = button_gallery(&CupertinoTheme::light());
        assert!(
            !scene.is_empty(),
            "a gallery with no nodes means the shared grid produced nothing"
        );
    }
}
