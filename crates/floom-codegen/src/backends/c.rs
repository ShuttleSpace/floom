//! C ABI generator (Phase 18.2, Step 1).
//!
//! Emits a single self-contained C99 header. C is the canonical boundary: the
//! C++, Swift and Kotlin backends all ultimately call these symbols, so this
//! file defines the ABI the others wrap.

use super::{all_type_refs, banner, macro_prefix, snake_case, symbol_prefix, walk_type};
use crate::schema::{Access, ComponentSchema, TypeRef};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// A stable, identifier-safe mangling of a type, used to name helper structs.
pub(super) fn mangle(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Bool => "bool".into(),
        TypeRef::I8 => "i8".into(),
        TypeRef::I16 => "i16".into(),
        TypeRef::I32 => "i32".into(),
        TypeRef::I64 => "i64".into(),
        TypeRef::U8 => "u8".into(),
        TypeRef::U16 => "u16".into(),
        TypeRef::U32 => "u32".into(),
        TypeRef::U64 => "u64".into(),
        TypeRef::F32 => "f32".into(),
        TypeRef::F64 => "f64".into(),
        TypeRef::String => "str".into(),
        TypeRef::Bytes => "bytes".into(),
        TypeRef::Resource(_) => "res".into(),
        TypeRef::Handle(_) => "handle".into(),
        TypeRef::Option(t) => format!("opt_{}", mangle(t)),
        TypeRef::Vec(t) => format!("vec_{}", mangle(t)),
        TypeRef::Custom(i) => snake_case(&i.0),
    }
}

/// The C spelling of a type.
///
/// `Option<T>` maps to the C spelling of `T`: pointer-shaped `T` are naturally
/// nullable (`NULL` = `None`); for value-shaped `T` C has no `None`, which the
/// generated comment flags. `Vec<T>` maps to an emitted `<slice>` struct.
pub(super) fn c_type(ty: &TypeRef) -> String {
    use TypeRef::*;
    match ty {
        Bool => "bool".into(),
        I8 => "int8_t".into(),
        I16 => "int16_t".into(),
        I32 => "int32_t".into(),
        I64 => "int64_t".into(),
        U8 => "uint8_t".into(),
        U16 => "uint16_t".into(),
        U32 => "uint32_t".into(),
        U64 => "uint64_t".into(),
        F32 => "float".into(),
        F64 => "double".into(),
        String => "const char *".into(),
        Bytes => "floem_bytes".into(),
        Resource(_) => "floem_resource_id".into(),
        Handle(_) => "void *".into(),
        Option(t) => c_type(t),
        Vec(t) => format!("floem_slice_{}", mangle(t)),
        // Custom types are only forward-declared (incomplete), so they are
        // passed and stored by pointer, never by value.
        Custom(i) => format!("floem_{} *", snake_case(&i.0)),
    }
}

/// The C spelling of an **event-callback** parameter list.
///
/// Struct-shaped values are expanded into a `(data pointer, length)` pair rather
/// than passed by value: it avoids copying an aggregate across the FFI boundary,
/// and it is the only shape a Swift `@convention(c)` function type can
/// represent (neither Swift structs nor pointers to them are ObjC-representable).
pub(super) fn c_callback_params(params: &[crate::schema::Param]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for p in params {
        let n = snake_case(&p.name.0);
        match &p.ty {
            TypeRef::Bytes => {
                out.push(("const uint8_t *".to_string(), n.clone()));
                out.push(("size_t".to_string(), format!("{n}_len")));
            }
            TypeRef::Vec(inner) => {
                out.push((format!("const {} *", c_type(inner)), n.clone()));
                out.push(("size_t".to_string(), format!("{n}_len")));
            }
            other => out.push((c_type(other), n)),
        }
    }
    out
}

/// The C spelling of `ty` as a *pointer-shaped* value, if it is one — used to
/// decide whether `Option<T>` can be represented as `NULL`.
fn is_nullable(ty: &TypeRef) -> bool {
    matches!(
        ty,
        TypeRef::String | TypeRef::Handle(_) | TypeRef::Custom(_)
    )
}

/// Emit the C ABI header for `schema`.
pub fn generate(schema: &ComponentSchema) -> Vec<super::GeneratedFile> {
    let prefix = symbol_prefix(schema);
    let macro_p = macro_prefix(schema);
    let guard = format!("{macro_p}_ABI_H");

    // Collect which helper types the schema actually needs, so the header is
    // minimal and its shape depends only on the schema.
    let mut need_resource = false;
    let mut need_bytes = false;
    let mut slices: BTreeMap<String, String> = BTreeMap::new(); // mangle -> elem C type
    let mut customs: BTreeSet<String> = BTreeSet::new();
    let mut nullable_value_option = false;

    for ty in all_type_refs(schema) {
        walk_type(ty, &mut |t| match t {
            TypeRef::Resource(_) => need_resource = true,
            TypeRef::Bytes => need_bytes = true,
            TypeRef::Vec(inner) => {
                slices.insert(mangle(inner), c_type(inner));
            }
            TypeRef::Custom(i) => {
                customs.insert(snake_case(&i.0));
            }
            TypeRef::Option(inner) if !is_nullable(inner) => nullable_value_option = true,
            _ => {}
        });
    }

    let mut out = String::new();
    let _ = writeln!(out, "/* Generated by floem-codegen. DO NOT EDIT. */");
    let _ = writeln!(out, "/* {} */", banner(schema));
    let _ = writeln!(out, "#ifndef {guard}");
    let _ = writeln!(out, "#define {guard}");
    let _ = writeln!(out);
    let _ = writeln!(out, "#include <stdbool.h>");
    let _ = writeln!(out, "#include <stddef.h>");
    let _ = writeln!(out, "#include <stdint.h>");
    let _ = writeln!(out);
    let _ = writeln!(out, "#ifdef __cplusplus");
    let _ = writeln!(out, "extern \"C\" {{");
    let _ = writeln!(out, "#endif");
    let _ = writeln!(out);

    let _ = writeln!(out, "#define {macro_p}_ABI_MAJOR {}u", schema.abi.major);
    let _ = writeln!(out, "#define {macro_p}_ABI_MINOR {}u", schema.abi.minor);
    let _ = writeln!(out);

    // Helper types.
    let mut wrote_helper = false;
    if need_resource {
        let _ = writeln!(out, "/* A renderer-owned resource handle. */");
        let _ = writeln!(out, "typedef uint64_t floem_resource_id;");
        wrote_helper = true;
    }
    if need_bytes {
        let _ = writeln!(out, "/* A borrowed byte buffer. */");
        let _ = writeln!(
            out,
            "typedef struct floem_bytes {{ const uint8_t *data; size_t len; }} floem_bytes;"
        );
        wrote_helper = true;
    }
    for (m, elem) in &slices {
        let _ = writeln!(out, "/* A borrowed slice of `{elem}`. */");
        let _ = writeln!(
            out,
            "typedef struct floem_slice_{m} {{ const {elem} *data; size_t len; }} floem_slice_{m};"
        );
        wrote_helper = true;
    }
    for name in &customs {
        let _ = writeln!(
            out,
            "/* Forward declaration: `{name}` is defined by the native side. */"
        );
        let _ = writeln!(out, "typedef struct floem_{name} floem_{name};");
        wrote_helper = true;
    }
    if wrote_helper {
        let _ = writeln!(out);
    }
    if nullable_value_option {
        let _ = writeln!(
            out,
            "/* NOTE: an `Option<value>` field above maps to a plain value type: C has"
        );
        let _ = writeln!(
            out,
            "   no `None` for non-pointer values. Pointer-shaped optional fields use"
        );
        let _ = writeln!(out, "   NULL for `None`. */");
        let _ = writeln!(out);
    }

    let handle = format!("{prefix}_handle");
    let _ = writeln!(
        out,
        "/* Opaque handle to a native `{}` instance. */",
        schema.name.0
    );
    let _ = writeln!(out, "typedef struct {handle} {handle};");
    let _ = writeln!(out);
    // Lifecycle: the framework owns the instance and destroys it explicitly.
    let _ = writeln!(out, "/* Lifecycle. */");
    let _ = writeln!(out, "void {prefix}_destroy({handle} *handle);");
    let _ = writeln!(out);

    // Properties.
    if !schema.properties.is_empty() {
        let _ = writeln!(out, "/* Properties. */");
        for p in &schema.properties {
            let t = c_type(&p.ty);
            let _ = writeln!(out, "/* `{}` ({}) */", p.name.0, access_str(p.access));
            if matches!(p.access, Access::Write | Access::ReadWrite) {
                let _ = writeln!(
                    out,
                    "void {prefix}_set_{}({handle} *handle, {t} value);",
                    snake_case(&p.name.0)
                );
            }
            if matches!(p.access, Access::Read | Access::ReadWrite) {
                let _ = writeln!(
                    out,
                    "{t} {prefix}_get_{}(const {handle} *handle);",
                    snake_case(&p.name.0)
                );
            }
        }
        let _ = writeln!(out);
    }

    // Commands.
    if !schema.commands.is_empty() {
        let _ = writeln!(out, "/* Commands (framework -> native). */");
        for c in &schema.commands {
            let params = params_decl(&c.params);
            let ret = c.ret.as_ref().map(c_type).unwrap_or_else(|| "void".into());
            let sep = if params.is_empty() { "" } else { ", " };
            let _ = writeln!(
                out,
                "{ret} {prefix}_cmd_{}({handle} *handle{sep}{params});",
                snake_case(&c.name.0)
            );
        }
        let _ = writeln!(out);
    }

    // Events.
    if !schema.events.is_empty() {
        let _ = writeln!(out, "/* Events (native -> framework). */");
        for e in &schema.events {
            let fn_ty = format!("{prefix}_event_{}_fn", snake_case(&e.name.0));
            let se = if e.params.is_empty() { "" } else { ", " };
            let _ = writeln!(
                out,
                "typedef void (*{fn_ty})(void *userdata{se}{});",
                params_decl_cb(&e.params)
            );
            let _ = writeln!(
                out,
                "void {prefix}_on_{}({handle} *handle, {fn_ty} callback, void *userdata);",
                snake_case(&e.name.0)
            );
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "#ifdef __cplusplus");
    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "#endif");
    let _ = writeln!(out, "#endif /* {guard} */");

    vec![super::GeneratedFile::new(format!("{prefix}_abi.h"), out)]
}

fn access_str(a: Access) -> &'static str {
    match a {
        Access::Read => "read",
        Access::Write => "write",
        Access::ReadWrite => "read/write",
    }
}

fn params_decl(params: &[crate::schema::Param]) -> String {
    params
        .iter()
        .map(|p| format!("{} {}", c_type(&p.ty), snake_case(&p.name.0)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn params_decl_cb(params: &[crate::schema::Param]) -> String {
    c_callback_params(params)
        .into_iter()
        .map(|(t, n)| format!("{t} {n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn type_mapping_is_stable() {
        assert_eq!(c_type(&TypeRef::String), "const char *");
        assert_eq!(c_type(&TypeRef::U64), "uint64_t");
        assert_eq!(c_type(&TypeRef::Bool), "bool");
        assert_eq!(c_type(&TypeRef::Bytes), "floem_bytes");
        assert_eq!(
            c_type(&TypeRef::Resource(crate::schema::Ident::new("T").unwrap())),
            "floem_resource_id"
        );
        assert_eq!(
            c_type(&TypeRef::Handle(crate::schema::Ident::new("V").unwrap())),
            "void *"
        );
        assert_eq!(
            c_type(&TypeRef::Vec(Box::new(TypeRef::F32))),
            "floem_slice_f32"
        );
        assert_eq!(
            c_type(&TypeRef::Custom(
                crate::schema::Ident::new("GpuDevice").unwrap()
            )),
            "floem_gpu_device *"
        );
    }

    #[test]
    fn header_declares_accessors_commands_events() {
        let s = fixtures::demo_video_player();
        let files = generate(&s);
        assert_eq!(files.len(), 1);
        let h = &files[0].contents;
        assert!(h.contains("typedef struct floem_video_player_handle floem_video_player_handle;"));
        assert!(h.contains("void floem_video_player_set_src(floem_video_player_handle *handle, const char * value);"));
        assert!(h.contains(
            "const char * floem_video_player_get_src(const floem_video_player_handle *handle);"
        ));
        assert!(h.contains(
            "void floem_video_player_cmd_seek(floem_video_player_handle *handle, uint64_t ms);"
        ));
        assert!(h.contains(
            "uint64_t floem_video_player_cmd_position(floem_video_player_handle *handle);"
        ));
        assert!(h.contains("typedef void (*floem_video_player_event_error_fn)(void *userdata, const char * message);"));
        assert!(h.contains("#define FLOEM_VIDEO_PLAYER_ABI_MAJOR 1u"));
        assert!(h.contains("#define FLOEM_VIDEO_PLAYER_ABI_MINOR 0u"));
    }

    #[test]
    fn write_only_property_has_no_getter() {
        let s = fixtures::wide_type_coverage();
        let h = &generate(&s)[0].contents;
        assert!(h.contains("void floem_wide_probe_set_device("));
        assert!(!h.contains("floem_wide_probe_get_device("));
        // read-only property has no setter
        assert!(h.contains("floem_wide_probe_get_maybe_count("));
        assert!(!h.contains("floem_wide_probe_set_maybe_count("));
    }

    #[test]
    fn wide_schema_emits_helper_types() {
        let s = fixtures::wide_type_coverage();
        let h = &generate(&s)[0].contents;
        assert!(h.contains("typedef uint64_t floem_resource_id;"));
        assert!(h.contains(
            "typedef struct floem_bytes { const uint8_t *data; size_t len; } floem_bytes;"
        ));
        assert!(h.contains("floem_slice_f32"));
        assert!(h.contains("typedef struct floem_gpu_device floem_gpu_device;"));
        // the value-shaped Option is flagged
        assert!(h.contains("no `None` for non-pointer values"));
    }
}
