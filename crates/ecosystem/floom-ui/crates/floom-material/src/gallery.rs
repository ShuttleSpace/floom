
use floom_design::{
    CellPaint, DepthField, button_gallery_labels as shared_labels, button_gallery_scene,
};
use floom_scene::Scene;

use crate::button::{button_metrics, button_style};
use crate::tokens::MaterialTheme;

/// Samples per row: one row per variant, one column per size × state.
///
/// Kept as a Material-side name for [`floom_design::FULL_GRID_COLUMNS`] so the
/// published `floom_material::GALLERY_COLUMNS` path does not break. The value is
/// shared: a gallery that laid out at a different column count per system would be
/// a difference nobody notices until two screenshots are compared.
pub const GALLERY_COLUMNS: usize = floom_design::FULL_GRID_COLUMNS;

/// Paint the full Material button grid.
pub fn button_gallery(theme: &MaterialTheme) -> Scene {
    button_gallery_scene(theme.roles.surface, |sample| {
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
pub fn button_gallery_labels(theme: &MaterialTheme) -> String {
    shared_labels(
        &format!(
            "scheme={:?} surface={:?}",
            theme.scheme, theme.roles.surface
        ),
        DepthField::Elevation,
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
        let labels = button_gallery_labels(&crate::tokens::MaterialTheme::light());
        // The header plus one line per (variant, size, state).
        let expected = Variant::ALL.len() * ControlSize::ALL.len() * InteractionState::ALL.len();
        assert_eq!(
            labels.lines().count(),
            expected + 1,
            "header plus {} samples",
            expected
        );
    }

    #[test]
    fn the_header_carries_the_scheme_and_surface() {
        let theme = crate::tokens::MaterialTheme::light();
        let labels = button_gallery_labels(&theme);
        let header = labels.lines().next().expect("a header line");
        assert!(header.contains("scheme="), "{header}");
        assert!(header.contains("surface="), "{header}");
    }

    #[test]
    fn the_gallery_scene_is_not_empty() {
        let scene = button_gallery(&crate::tokens::MaterialTheme::light());
        assert!(
            !scene.is_empty(),
            "a gallery with no nodes means the shared grid produced nothing"
        );
    }
}
