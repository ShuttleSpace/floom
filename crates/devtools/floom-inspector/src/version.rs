// SPDX-License-Identifier: MIT
//! Wire-format versioning with concrete compatibility rules.

use serde::{Deserialize, Serialize};

/// The protocol version a snapshot was written with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
}

impl ProtocolVersion {
    /// The version this crate writes.
    pub const fn current() -> Self {
        Self { major: 1, minor: 0 }
    }

    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Whether a snapshot at `other` can be served to a reader at `self`.
    ///
    /// Same major, `other.minor <= self.minor`. A newer minor is refused for a
    /// concrete reason: the reader cannot faithfully round-trip fields it has
    /// never heard of, so serving the snapshot would silently drop data.
    pub fn accepts(&self, other: ProtocolVersion) -> bool {
        other.major == self.major && other.minor <= self.minor
    }

    /// [`Self::accepts`] with the rejection explained.
    pub fn accepts_err(&self, other: ProtocolVersion) -> Result<(), String> {
        if other.major != self.major {
            return Err(format!(
                "protocol major mismatch: reader is {self}, snapshot is {other}"
            ));
        }
        if other.minor > self.minor {
            return Err(format!(
                "snapshot is a newer minor than the reader understands: reader is \
                 {self}, snapshot is {other}; upgrade the reader to load it"
            ));
        }
        Ok(())
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}
