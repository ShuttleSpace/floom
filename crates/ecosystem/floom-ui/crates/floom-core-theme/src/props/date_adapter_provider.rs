#[derive(Debug, Clone)]
pub struct DateAdapterProviderProps {
    /// Identifier of the date adapter used by date-related descendants ("native" by default).
    pub adapter: String,
}

impl Default for DateAdapterProviderProps {
    fn default() -> Self {
        Self {
            adapter: "native".to_string(),
        }
    }
}
