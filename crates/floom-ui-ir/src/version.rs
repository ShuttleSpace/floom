//! The IR version, the reader's acceptance rules, and the migration table.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::document::UiDocument;
use crate::error::IrError;

/// Version of the UI IR.
///
/// `major` gates structural compatibility: a bump may rename a property, change
/// a value's shape, or add a node kind, and none of those can be faked from the
/// other side. `minor` is for additive change only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct IrVersion {
    pub major: u32,
    pub minor: u32,
}

impl IrVersion {
    /// The version this build writes and reads natively.
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Whether a document written at `doc` can be read by a reader at `self`.
    ///
    /// Three rules, each with a reason rather than an arbitrary threshold:
    ///
    /// 1. **A different major is refused.** The structural vocabulary changed.
    /// 2. **A newer minor is refused.** We cannot round-trip a field we do not
    ///    know about, and dropping it would corrupt the one document everybody
    ///    else is deriving from. Refusing is the only honest option; the error
    ///    names both versions so the author knows what to upgrade.
    /// 3. **An older minor is upgraded** through [`crate::migrate`], which
    ///    fails loudly if no step exists rather than treating the document as
    ///    if it were current.
    pub fn check(self, doc: Self) -> Result<(), IrError> {
        if doc.major != self.major {
            return Err(IrError::MajorMismatch {
                found: doc.major,
                supported: self.major,
            });
        }
        if doc.minor > self.minor {
            return Err(IrError::VersionTooNew {
                found: doc,
                supported: self,
            });
        }
        Ok(())
    }
}

impl fmt::Display for IrVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

/// One step that brings a document from `from_minor` to `from_minor + 1`.
pub struct MigrationStep {
    /// The minor this step reads. It produces `from_minor + 1`.
    pub from_minor: u32,
    /// The rewrite itself. Infallible: a step that cannot succeed is a missing
    /// step, and belongs in the table's absence rather than in a `Result`.
    pub apply: fn(&mut UiDocument),
}

impl fmt::Debug for MigrationStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MigrationStep")
            .field("from_minor", &self.from_minor)
            .finish_non_exhaustive()
    }
}

/// The steps this build ships.
///
/// Empty while [`IrVersion::CURRENT`]`minor` is `0`, which is exactly right:
/// there is no older minor in existence, so there is nothing to upgrade. The
/// machinery below is not speculative — [`migrate_with`] is the only way a
/// document of a different minor ever becomes current, and the tests drive it
/// with synthetic tables. When the first real step is added, add the fixture
/// for the older-minor document it upgrades at the same time; the assertion in
/// `tests/fixtures.rs` deliberately trips until that is done.
pub const MIGRATIONS: &[MigrationStep] = &[];

/// Upgrade `doc` from `from` to `to` using `table`.
///
/// Returns the `from_minor` of every step that ran, in order, so a caller can
/// distinguish "read as written" from "read, then rewritten".
pub fn migrate_with(
    table: &[MigrationStep],
    doc: &mut UiDocument,
    from: IrVersion,
    to: IrVersion,
) -> Result<Vec<u32>, IrError> {
    if from.major != to.major {
        return Err(IrError::MajorMismatch {
            found: from.major,
            supported: to.major,
        });
    }
    let mut applied = Vec::new();
    let mut minor = from.minor;
    while minor < to.minor {
        let Some(step) = table.iter().find(|s| s.from_minor == minor) else {
            return Err(IrError::NoMigrationPath { minor });
        };
        (step.apply)(doc);
        applied.push(minor);
        minor += 1;
    }
    doc.version = to;
    Ok(applied)
}

/// Upgrade `doc` to the current version with the shipped table.
pub fn migrate(doc: &mut UiDocument, from: IrVersion) -> Result<Vec<u32>, IrError> {
    migrate_with(MIGRATIONS, doc, from, IrVersion::CURRENT)
}
