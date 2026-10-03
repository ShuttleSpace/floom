use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct DefaultsProviderProps {
    /// Flat map of `Component.Prop` -> default value string applied to descendants.
    pub defaults: HashMap<String, String>,
}
