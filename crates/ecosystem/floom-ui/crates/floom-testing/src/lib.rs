#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub name: String,
    pub content: String,
}

impl Snapshot {
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }

    pub fn assert_matches(&self, expected: &str) {
        assert_eq!(self.content, expected, "snapshot `{}` changed", self.name);
    }
}

pub fn component_snapshot(name: impl Into<String>, content: impl Into<String>) -> Snapshot {
    Snapshot::new(name, content)
}
