use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct IconsProviderProps {
    /// Map of semantic icon alias -> glyph/asset identifier provided to descendants.
    pub icons: HashMap<String, String>,
}
