//! The HarmonyOS component gallery.

use floom_design::{CellPaint, ControlSize, GallerySample, InteractionState, gallery_scene};
use floom_scene::Scene;

use crate::button::{button_metrics, button_style};
use crate::tokens::HarmonyTheme;

pub const GALLERY_COLUMNS: usize = ControlSize::ALL.len() * InteractionState::ALL.len();

/// Paint the full HarmonyOS button grid.
pub fn button_gallery(theme: &HarmonyTheme) -> Scene {
    let samples = GallerySample::full_grid();
    gallery_scene(
        &samples,
        theme.colors.background,
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
pub fn button_gallery_labels(theme: &HarmonyTheme) -> String {
    let mut out = format!(
        "scheme={:?} background={:?}\n",
        theme.scheme, theme.colors.background
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
