//! The Cupertino component gallery.

use floem_design::{CellPaint, ControlSize, GallerySample, InteractionState, gallery_scene};
use floem_scene::Scene;

use crate::button::{button_metrics, button_style};
use crate::tokens::CupertinoTheme;

pub const GALLERY_COLUMNS: usize = ControlSize::ALL.len() * InteractionState::ALL.len();

/// Paint the full Cupertino button grid.
pub fn button_gallery(theme: &CupertinoTheme) -> Scene {
    let samples = GallerySample::full_grid();
    gallery_scene(
        &samples,
        theme.colors.grouped_background,
        GALLERY_COLUMNS,
        |sample| {
            CellPaint::new(
                button_style(theme, sample.variant, sample.size, sample.state),
                button_metrics(theme, sample.size).height as f64,
            )
        },
    )
}

/// A textual rendering of the same grid.
pub fn button_gallery_labels(theme: &CupertinoTheme) -> String {
    let mut out = format!(
        "scheme={:?} grouped_background={:?}\n",
        theme.scheme, theme.colors.grouped_background
    );
    for sample in GallerySample::full_grid() {
        let p = button_style(theme, sample.variant, sample.size, sample.state);
        out.push_str(&format!(
            "{:<18} bg={:?} fg={:?} border={}x{:?} opacity={:?} radius={:?} padding={:?} font={:?}\n",
            sample.label(),
            p.background,
            p.foreground,
            p.border_width.unwrap_or(0.0),
            p.border_color,
            p.opacity,
            p.radius,
            p.padding,
            p.font_size,
        ));
    }
    out
}
