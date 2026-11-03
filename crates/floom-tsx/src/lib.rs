//! `floom-tsx`: a compile-time TSX front end for Floom.
//!
//! TSX source is parsed with `swc_ecma_parser` (build-time only — it never ships
//! in the application binary) and lowered into a `floom_ui_ir::UiDocument`. The
//! document then flows through the existing `floom-dsl` emitter, which produces a
//! Rust builder program drawn by Floom's own renderers. `floom-tsx` replaces
//! only the front-end parser; everything downstream is reused unchanged.
//!
//! ```ignore
//! let doc = floom_tsx::compile("<Stack><Text label=\"hi\" /></Stack>").unwrap();
//! let rust = floom_dsl::emit::emit(&doc, &floom_dsl::EmitOptions::default()).unwrap();
//! ```

pub mod app;
pub mod decl;
pub mod error;
pub mod lower;
pub mod parser;

pub use app::{CompiledApp, Generated, generate};
pub use error::{Result, TsxError};
pub use lower::AppConfig;

use std::path::Path;

use floom_ui_ir::{IrVersion, UiDocument};

use crate::parser::parse;

/// Compile a TSX source string into a validated `UiDocument`.
///
/// The source may be a full component module (`export function Counter() { … }`)
/// or a bare JSX expression (`<Stack>…</Stack>`), which is wrapped as a default
/// export.
pub fn compile(source: &str) -> Result<UiDocument> {
    let parsed = parse(source)?;
    lower::lower(&parsed)
}

/// Compile a project of `.tsx` files into one `UiDocument`.
///
/// Each file is lowered independently; their nodes and bindings are merged and
/// the first file's root becomes the document root. A real multi-component app
/// would namespace components, but for the build.rs integration a single root is
/// what `mount_root` consumes.
pub fn compile_project(paths: &[std::path::PathBuf]) -> Result<UiDocument> {
    let mut merged = UiDocument {
        version: IrVersion::CURRENT,
        root: floom_ui_ir::NodeKey::new("root"),
        bindings: Vec::new(),
        nodes: Vec::new(),
    };
    for path in paths {
        let source = std::fs::read_to_string(path)
            .map_err(|e| TsxError::new(format!("cannot read {}: {e}", path.display())))?;
        let doc = compile(&source)?;
        if merged.nodes.is_empty() {
            merged.root = doc.root;
        }
        merged.bindings.extend(doc.bindings);
        merged.nodes.extend(doc.nodes);
    }
    merged
        .validate()
        .map_err(|e| TsxError::new(format!("IR validation failed: {e}")))?;
    Ok(merged)
}

/// Generate TypeScript declaration source (`.d.ts`) so TSX authors get editor
/// completions for the widgets and props Floom exposes.
pub fn generate_dts(doc: &UiDocument) -> String {
    decl::generate(doc)
}

/// Convenience: read a `.tsx` file and compile it.
pub fn compile_file(path: &Path) -> Result<UiDocument> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| TsxError::new(format!("cannot read {}: {e}", path.display())))?;
    compile(&source)
}
