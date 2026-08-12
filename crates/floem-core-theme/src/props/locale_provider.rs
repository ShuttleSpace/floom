#[derive(Debug, Clone)]
pub struct LocaleProviderProps {
    /// BCP-47 locale tag provided to descendants (e.g. "en", "zh-CN").
    pub locale: String,
}

impl Default for LocaleProviderProps {
    fn default() -> Self {
        Self {
            locale: "en".to_string(),
        }
    }
}
