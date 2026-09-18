//! Native-component code generation for Floem 2.0 (Phase 18).
//!
//! `floem-codegen` is the compiler that turns a declarative
//! [`schema::ComponentSchema`] into native glue (Swift/Kotlin/C/C++), so a
//! Rust-defined native participant can be driven from the framework and from
//! OS code without hand-written FFI boilerplate.
//!
//! The schema lives in [`schema`] (Phase 18.1) and the backend generators that
//! consume it live in [`backends`] (Phase 18.2): C, C++, Swift and Kotlin.
//! [`fixtures`] holds reference schemas shared by tests and compile fixtures.

pub mod backends;
pub mod fixtures;
pub mod schema;

pub use backends::{GeneratedFile, Language, generate, generate_all};
pub use schema::{
    AbiVersion, Access, CommandSchema, ComponentSchema, Composition, EventSchema, Ident, InputMode,
    Literal, Param, PropertySchema, SchemaError, TypeRef, ViewKind,
};
