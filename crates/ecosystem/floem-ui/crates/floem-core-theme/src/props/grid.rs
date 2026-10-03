/// Neutral props shared by every design system's `Grid`.
///
/// A grid lays out children into a fixed number of equal-width columns,
/// wrapping every `cols` items onto a new row. Styles mirror Vuetify's row/col
/// spacing model without per-column offsets (which this Floem build cannot
/// place absolutely).
#[derive(Debug, Clone)]
pub struct GridProps {
    /// Number of equal-width columns per row.
    pub cols: usize,
    /// Gap between columns within a row.
    pub gap: f32,
    /// Gap between rows.
    pub row_gap: f32,
}

impl Default for GridProps {
    fn default() -> Self {
        Self {
            cols: 12,
            gap: 12.0,
            row_gap: 12.0,
        }
    }
}
