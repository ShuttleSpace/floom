pub struct DataIteratorProps {
    /// Draw a thin divider between consecutive items.
    pub divided: bool,
}

impl Clone for DataIteratorProps {
    fn clone(&self) -> Self {
        Self {
            divided: self.divided,
        }
    }
}

impl std::fmt::Debug for DataIteratorProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataIteratorProps")
            .field("divided", &self.divided)
            .finish()
    }
}

impl Default for DataIteratorProps {
    fn default() -> Self {
        Self { divided: true }
    }
}
