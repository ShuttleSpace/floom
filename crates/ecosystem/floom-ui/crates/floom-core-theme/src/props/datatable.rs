/// Horizontal alignment of a table column's cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlign {
    Start,
    Center,
    End,
}

impl Default for ColumnAlign {
    fn default() -> Self {
        ColumnAlign::Start
    }
}

/// A single column definition for the data table.
#[derive(Debug, Clone)]
pub struct DataTableColumn {
    /// Header label shown at the top of the column.
    pub title: String,
    /// How cells in this column are aligned.
    pub align: ColumnAlign,
}

impl Default for DataTableColumn {
    fn default() -> Self {
        Self {
            title: String::new(),
            align: ColumnAlign::Start,
        }
    }
}

/// Neutral, design-system-agnostic props for the data table.
pub struct DataTableProps {
    /// Column headers + alignment.
    pub columns: Vec<DataTableColumn>,
    /// Row-major cell text. Each inner `Vec` must align with `columns`.
    pub rows: Vec<Vec<String>>,
    /// Called with the row index when a row is clicked.
    pub on_row_click: Option<Box<dyn Fn(usize)>>,
    /// Render alternating row backgrounds.
    pub striped: bool,
    /// Tighter row spacing.
    pub dense: bool,
}

impl Clone for DataTableProps {
    fn clone(&self) -> Self {
        Self {
            columns: self.columns.clone(),
            rows: self.rows.clone(),
            on_row_click: None,
            striped: self.striped,
            dense: self.dense,
        }
    }
}

impl std::fmt::Debug for DataTableProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataTableProps")
            .field("columns", &self.columns)
            .field("rows", &self.rows)
            .field("striped", &self.striped)
            .field("dense", &self.dense)
            .field(
                "on_row_click",
                &self.on_row_click.as_ref().map(|_| "Fn(usize)"),
            )
            .finish()
    }
}

impl Default for DataTableProps {
    fn default() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            on_row_click: None,
            striped: false,
            dense: false,
        }
    }
}
