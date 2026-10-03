//! Tier-2 advanced data widgets for Floom 2.0 (Phase 20).
//!
//! The phase splits in two:
//!
//! - **Data views** — [`model`] holds the headless data layer (cells, sorting,
//!   selection, an edit history) and [`data_grid`] / [`tree`] are its
//!   virtualized views. Both draw only their visible window, and both report
//!   the window explicitly so a retained-mode host can recycle a bounded pool
//!   of element-backed rows via [`floom_virtual`].
//! - **Visualization and transport** — [`chart`] computes renderer-neutral
//!   geometry and an accessible summary; [`media`] is the transport control
//!   cluster and its semantic parts.
//!
//! Nothing here reaches a concrete renderer, a windowing system or a GPU
//! backend. Chart geometry and every hit test are plain arithmetic, so they can
//! be asserted exactly without a window — which is also why this crate stays
//! green when a paint path changes underneath it.

#![forbid(unsafe_code)]

pub mod chart;
pub mod data_grid;
pub mod media;
pub mod model;
pub mod tree;

pub use chart::{
    AxisSpec, Bar, Chart, ChartSeries, ChartSummary, DataPoint, LinearScale, Marker, OrderedF64,
    Segment, SeriesKind, StrokeColumn, Tick,
};
pub use data_grid::{DataGrid, RowPool};
pub use media::{MediaControls, MediaKind, MediaLayout, MediaPart, MediaState, format_time};
pub use model::{
    CellEdit, CellValue, Column, ColumnId, ColumnKind, EditHistory, EditSession, RowId, RowKey,
    SelectionMode, SelectionModel, SortDirection, SortSpec, TableModel, VecTable, sorted_order,
};
pub use tree::{FlatRow, Tree, TreeNode, TreeNodeId, VecTree};
