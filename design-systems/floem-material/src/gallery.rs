//! The Material component gallery: every button variant × size × state.

use floem_design::{CellPaint, ControlSize, GallerySample, InteractionState, gallery_scene};
use floem_scene::Scene;

use crate::button::{button_metrics, button_style};
use crate::tokens::MaterialTheme;

/// Samples per row: one row per variant, one column per size × state.
pub const GALLERY_COLUMNS: usize = ControlSize::ALL.len() * InteractionState::ALL.len();

/// Paint the full Material button grid.
pub fn button_gallery(theme: &MaterialTheme) -> Scene {
    let samples = GallerySample::full_grid();
    gallery_scene(&samples, theme.roles.surface, GALLERY_COLUMNS, |sample| {
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
    let mut out = format!(
        "scheme={:?} surface={:?}\n",
        theme.scheme, theme.roles.surface
    );
    for sample in GallerySample::full_grid() {
        let p = button_style(theme, sample.variant, sample.size, sample.state);
        out.push_str(&format!(
            "{:<18} bg={:?} fg={:?} border={}x{:?} elevation={:?} radius={:?} padding={:?} font={:?}\n",
            sample.label(),
            p.background,
            p.foreground,
            p.border_width.unwrap_or(0.0),
            p.border_color,
            p.elevation,
            p.radius,
            p.padding,
            p.font_size,
        ));
    }
    out
}
