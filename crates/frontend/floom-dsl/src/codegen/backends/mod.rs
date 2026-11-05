// SPDX-License-Identifier: MIT
//! Native glue generators (Phase 18.2).
//!
//! Each backend consumes a validated [`ComponentSchema`] and emits one or more
//! deterministic source files. Output is a pure function of the schema — no
//! timestamps, no filesystem state, no hashing order — so the generators are
//! snapshot-testable and reproducible across machines.
//!
//! The generated files describe the *native side* of a native component: the
//! opaque handle, its property accessors, the commands the framework issues
//! into native code, and the event callbacks native code raises back. They are
//! the counterpart of the Rust [`floom_types`] native-composition ABI.

use crate::codegen::schema::{ComponentSchema, SchemaError, TypeRef};

pub mod c;
pub mod cpp;
pub mod kotlin;
pub mod swift;

/// Target language for generated glue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// C99 header (the canonical FFI boundary every other language wraps).
    C,
    /// C++17 RAII facade over the C ABI (used by the Harmony/OHOS side).
    Cpp,
    /// Swift glue with `@_silgen_name` C shims.
    Swift,
    /// Kotlin glue with `external` JNI declarations.
    Kotlin,
}

impl Language {
    /// All languages the codegen backend knows how to emit.
    pub const ALL: [Language; 4] = [
        Language::C,
        Language::Cpp,
        Language::Swift,
        Language::Kotlin,
    ];

    /// The lowercase name used in diagnostics and CLI output.
    pub fn as_str(self) -> &'static str {
        match self {
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Swift => "swift",
            Language::Kotlin => "kotlin",
        }
    }
}

/// One generated source file: a path relative to the generation root, plus its
/// contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub path: String,
    pub contents: String,
}

impl GeneratedFile {
    pub fn new(path: impl Into<String>, contents: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            contents: contents.into(),
        }
    }
}

/// Generate the glue files for `schema` in `lang`.
///
/// The schema is validated first; an invalid schema yields `Err` and no files,
/// so a backend never emits code for a component whose names collide.
pub fn generate(
    schema: &ComponentSchema,
    lang: Language,
) -> Result<Vec<GeneratedFile>, SchemaError> {
    schema.validate()?;
    Ok(match lang {
        Language::C => c::generate(schema),
        Language::Cpp => cpp::generate(schema),
        Language::Swift => swift::generate(schema),
        Language::Kotlin => kotlin::generate(schema),
    })
}

/// Generate every supported language for a schema, concatenated in
/// [`Language::ALL`] order. Convenience for tooling that emits a full SDK.
pub fn generate_all(schema: &ComponentSchema) -> Result<Vec<GeneratedFile>, SchemaError> {
    let mut files = Vec::new();
    for lang in Language::ALL {
        files.extend(generate(schema, lang)?);
    }
    Ok(files)
}

/// Convert a `PascalCase` / `camelCase` identifier to `snake_case`.
///
/// `VideoPlayer` → `video_player`, `WebView` → `web_view`,
/// `HTTPServer` → `http_server`. Deterministic and total.
pub fn snake_case(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 4);
    for (i, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1).copied();
            let boundary = match prev {
                None => false,
                Some(p) => {
                    p.is_ascii_lowercase()
                        || p.is_ascii_digit()
                        || (p.is_ascii_uppercase() && next.is_some_and(|n| n.is_ascii_lowercase()))
                }
            };
            if boundary {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// `snake_case` uppercased — for include guards and C macros.
pub fn upper_snake(s: &str) -> String {
    snake_case(s).to_ascii_uppercase()
}

/// The symbol prefix for a component: `floom_` + snake-cased name.
pub fn symbol_prefix(schema: &ComponentSchema) -> String {
    format!("floom_{}", snake_case(&schema.name.0))
}

/// The macro prefix for a component: `FLOOM_` + upper-snake-cased name.
pub fn macro_prefix(schema: &ComponentSchema) -> String {
    format!("FLOOM_{}", upper_snake(&schema.name.0))
}

/// Visit `ty` and every type nested under it (`Option`/`Vec`).
pub fn walk_type(ty: &TypeRef, f: &mut impl FnMut(&TypeRef)) {
    f(ty);
    match ty {
        TypeRef::Option(inner) | TypeRef::Vec(inner) => walk_type(inner, f),
        _ => {}
    }
}

/// Every type reference appearing anywhere in the schema (properties, command
/// parameters and returns, event parameters).
pub fn all_type_refs(schema: &ComponentSchema) -> Vec<&TypeRef> {
    let mut v = Vec::new();
    for p in &schema.properties {
        v.push(&p.ty);
    }
    for c in &schema.commands {
        for p in &c.params {
            v.push(&p.ty);
        }
        if let Some(r) = &c.ret {
            v.push(r);
        }
    }
    for e in &schema.events {
        for p in &e.params {
            v.push(&p.ty);
        }
    }
    v
}

/// Whether any type in the schema satisfies `pred` (searching nested types).
pub fn any_type(schema: &ComponentSchema, pred: &impl Fn(&TypeRef) -> bool) -> bool {
    let mut hit = false;
    for ty in all_type_refs(schema) {
        walk_type(ty, &mut |t| {
            if pred(t) {
                hit = true;
            }
        });
    }
    hit
}

/// A short human-readable description of a schema's identity, used in the
/// generated file banner so a reader can tell which component a file came from.
pub fn banner(schema: &ComponentSchema) -> String {
    format!(
        "component `{}` abi {}.{} kind={:?} composition={:?} input={:?}",
        schema.name.0,
        schema.abi.major,
        schema.abi.minor,
        schema.kind,
        schema.composition,
        schema.input
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::fixtures;

    #[test]
    fn snake_case_is_deterministic_and_readable() {
        assert_eq!(snake_case("VideoPlayer"), "video_player");
        assert_eq!(snake_case("WebView"), "web_view");
        assert_eq!(snake_case("HTTPServer"), "http_server");
        assert_eq!(snake_case("already_snake"), "already_snake");
        assert_eq!(snake_case("A"), "a");
        assert_eq!(upper_snake("VideoPlayer"), "VIDEO_PLAYER");
    }

    #[test]
    fn prefixes_are_derived_consistently() {
        let s = fixtures::demo_video_player();
        assert_eq!(symbol_prefix(&s), "floom_video_player");
        assert_eq!(macro_prefix(&s), "FLOOM_VIDEO_PLAYER");
    }

    #[test]
    fn generate_all_emits_every_language() {
        let s = fixtures::demo_video_player();
        let files = generate_all(&s).unwrap();
        // C(1) + Cpp(1) + Swift(2) + Kotlin(1)
        assert_eq!(
            files.len(),
            5,
            "files: {:?}",
            files.iter().map(|f| &f.path).collect::<Vec<_>>()
        );
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"floom_video_player_abi.h"));
        assert!(paths.contains(&"floom_video_player_glue.hpp"));
        assert!(paths.contains(&"VideoPlayer.swift"));
        assert!(paths.contains(&"VideoPlayerShims.swift"));
        assert!(paths.contains(&"VideoPlayer.kt"));
    }

    #[test]
    fn invalid_schema_produces_no_files() {
        let mut s = fixtures::demo_video_player();
        s.commands.push(s.commands[0].clone()); // duplicate name
        for lang in Language::ALL {
            assert!(generate(&s, lang).is_err());
        }
    }

    #[test]
    fn generate_is_deterministic() {
        let s = fixtures::demo_video_player();
        for lang in Language::ALL {
            let a = generate(&s, lang).unwrap();
            let b = generate(&s, lang).unwrap();
            assert_eq!(a, b, "{} output must be stable", lang.as_str());
        }
    }
}
