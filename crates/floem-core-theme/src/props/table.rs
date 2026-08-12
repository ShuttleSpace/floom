pub struct TableProps {
    /// Column header labels (defines the number of columns).
    pub headers: Vec<String>,
    /// Body rows; each inner vector aligns positionally with `headers`.
    pub rows: Vec<Vec<String>>,
    /// Compact row height.
    pub dense: bool,
    /// Alternate row background for readability.
    pub striped: bool,
}

impl Clone for TableProps {
    fn clone(&self) -> Self {
        Self {
            headers: self.headers.clone(),
            rows: self.rows.clone(),
            dense: self.dense,
            striped: self.striped,
        }
    }
}

impl std::fmt::Debug for TableProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TableProps")
            .field("headers", &self.headers)
            .field("rows", &self.rows)
            .field("dense", &self.dense)
            .field("striped", &self.striped)
            .finish()
    }
}

impl Default for TableProps {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            rows: Vec::new(),
            dense: false,
            striped: false,
        }
    }
}
