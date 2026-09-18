//! Versioned, serializable UI intermediate representation (Phase 22.1).
//!
//! The IR is the *source of truth* a UI is authored in when it does not come
//! from Rust source: the DSL lowers into it ([`floem-dsl`]), the hot-reload
//! protocol diffs two of them ([`floem-hot-reload`]), and the inspector walks
//! it to explain a running tree. It therefore has to be two things at once:
//!
//! - **Stable on the wire.** Every document carries its [`IrVersion`], and the
//!   reader's acceptance rules are explicit ([`IrVersion::check`]). A document
//!   is parsed, validated and re-serialized byte for byte by the fixtures, so a
//!   field rename cannot pass unnoticed.
//! - **Cheap to reason about offline.** A node's identity is a [`NodeKey`], a
//!   stable string the runtime can match across a reload — not an index, which
//!   would silently move when a sibling is inserted.
//!
//! Like `floem-codegen`, this crate is runtime-free: it depends on the L0
//! vocabulary ([`floem_types::Color`], for the bridge) and nothing else, so a
//! tool that only reads documents never links a window.

#![forbid(unsafe_code)]

mod design_contract;
mod document;
mod error;
mod importer;
mod node;
mod schema;
mod value;
mod version;

pub use design_contract::{
    Accessibility, Animation, ColorRole, ContractDiff, ContractValue, ContractVersion,
    DesignContract, DiffEntry, Dimensions, Radius, Spacing, StateStyle, TypeStyle, compare,
};
pub use document::{LoadOutcome, UiDocument};
pub use error::IrError;
pub use importer::{
    AutoLayout, DesignAxis, DesignFile, DesignNode, DesignRole, DesignSize, DesignStroke,
    ImportedDesign, ImporterError, TextSpec, import,
};
pub use node::{
    DEFAULT_SLOT, EventBinding, EventName, NativeKind, NodeKey, NodeKind, Prop, Slot, UiNode,
};
pub use schema::{
    PropClass, PropSpec, all_prop_names, emits, events_for, prop_is_declared, prop_spec,
};
pub use value::{Binding, BindingKind, PropValue, Rgba, ValueType};
pub use version::{IrVersion, MIGRATIONS, MigrationStep, migrate, migrate_with};
