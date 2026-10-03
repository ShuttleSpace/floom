//! The protocol's error type.

/// Every way a snapshot can be malformed or unusable.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolError {
    /// The snapshot's version is outside what the reader accepts.
    UnsupportedVersion {
        reader: crate::ProtocolVersion,
        snapshot: crate::ProtocolVersion,
    },
    /// The tree snapshot contradicts itself.
    InvalidTree { reason: String },
    /// The root id does not name a node in the tree.
    MissingRoot { root: u64 },
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion { reader, snapshot } => {
                write!(
                    f,
                    "snapshot version {snapshot} is not accepted by reader {reader}"
                )
            }
            Self::InvalidTree { reason } => write!(f, "invalid tree snapshot: {reason}"),
            Self::MissingRoot { root } => {
                write!(f, "root id {root} does not name a node in the tree")
            }
        }
    }
}

impl std::error::Error for ProtocolError {}
