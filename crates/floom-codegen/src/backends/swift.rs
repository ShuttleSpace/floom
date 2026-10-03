//! Swift glue generator (Phase 18.2, Step 2).
//!
//! Emits two files: `<Name>Shims.swift` (the C-layout mirror structs and the
//! `@_silgen_name` bindings to the C ABI) and `<Name>.swift` (the public Swift
//! class). Event callbacks recover the owning instance from the `userdata`
//! pointer, so the registered closures capture no context and convert to C
//! function pointers.

use super::c::mangle;
use super::{all_type_refs, banner, snake_case, symbol_prefix, walk_type};
use crate::schema::{Access, ComponentSchema, Param, TypeRef};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

/// The Swift type seen by callers.
fn api_type(ty: &TypeRef) -> String {
    use TypeRef::*;
    match ty {
        Bool => "Bool".into(),
        I8 => "Int8".into(),
        I16 => "Int16".into(),
        I32 => "Int32".into(),
        I64 => "Int64".into(),
        U8 => "UInt8".into(),
        U16 => "UInt16".into(),
        U32 => "UInt32".into(),
        U64 => "UInt64".into(),
        F32 => "Float".into(),
        F64 => "Double".into(),
        String => "String".into(),
        Bytes => "FloomBytes".into(),
        Resource(_) => "UInt64".into(),
        // A C `void *` and a pointer to an incomplete `struct` are both
        // optional pointers on the Swift side.
        Handle(_) => "UnsafeMutableRawPointer?".into(),
        Option(t) => format!("{}?", api_type(t)),
        Vec(t) => format!("FloomSlice{}", mirror_suffix(t)),
        Custom(_) => "OpaquePointer?".into(),
    }
}

/// The C-compatible Swift type used in `@_silgen_name` shim signatures.
///
/// `Option<T>` deliberately drops the `?`: the C parameter is the same as for
/// `T`, and pointer-shaped `T` are already optional in Swift.
fn shim_type(ty: &TypeRef) -> String {
    use TypeRef::*;
    match ty {
        String => "UnsafePointer<CChar>?".into(),
        Handle(_) => "UnsafeMutableRawPointer?".into(),
        Custom(_) => "OpaquePointer?".into(),
        Bytes | Vec(_) => api_type(ty),
        Option(inner) => shim_type(inner),
        other => api_type(other),
    }
}

/// One expanded event-callback parameter.
///
/// `shim` is the C-callback type (must be ObjC-representable), `closure` is the
/// type the caller's `on<Event>` closure sees, and `arg` converts the shim value
/// into the closure argument.
struct EvParam {
    name: String,
    shim: String,
    closure: String,
    arg: String,
}

/// Expand an event's parameters for the Swift layer.
///
/// Struct-shaped values become `(pointer, count)` pairs, matching the C callback
/// typedef; `String` becomes a `String` at the closure boundary.
fn swift_event_params(params: &[Param]) -> Vec<EvParam> {
    let mut out = Vec::new();
    for p in params {
        let n = snake_case(&p.name.0);
        match &p.ty {
            TypeRef::Bytes => {
                out.push(EvParam {
                    name: n.clone(),
                    shim: "UnsafePointer<UInt8>?".into(),
                    closure: "UnsafePointer<UInt8>?".into(),
                    arg: n.clone(),
                });
                out.push(EvParam {
                    name: format!("{n}_len"),
                    shim: "Int".into(),
                    closure: "Int".into(),
                    arg: format!("{n}_len"),
                });
            }
            TypeRef::Vec(inner) => {
                let ptr = format!("UnsafePointer<{}>?", api_type(inner));
                out.push(EvParam {
                    name: n.clone(),
                    shim: ptr.clone(),
                    closure: ptr,
                    arg: n.clone(),
                });
                out.push(EvParam {
                    name: format!("{n}_len"),
                    shim: "Int".into(),
                    closure: "Int".into(),
                    arg: format!("{n}_len"),
                });
            }
            TypeRef::String => out.push(EvParam {
                name: n.clone(),
                shim: "UnsafePointer<CChar>?".into(),
                closure: "String".into(),
                arg: format!("({n}.map {{ String(cString: $0) }} ?? \"\")"),
            }),
            other => {
                let t = shim_type(other);
                out.push(EvParam {
                    name: n.clone(),
                    shim: t.clone(),
                    closure: t,
                    arg: n,
                });
            }
        }
    }
    out
}

/// Suffix for the concrete slice mirror struct of a `Vec<T>` element.
fn mirror_suffix(elem: &TypeRef) -> String {
    mangle(elem)
        .split('_')
        .map(|part| {
            let mut c = part.chars();
            match c.next() {
                Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Recover the C mangling from a slice suffix (`F32` → `f32`).
fn mangle_of_suffix(suffix: &str) -> String {
    let mut out = String::new();
    for ch in suffix.chars() {
        if ch.is_ascii_uppercase() && !out.is_empty() {
            out.push('_');
        }
        out.push(ch.to_ascii_lowercase());
    }
    out
}

/// Convert a shim-level expression into the API-level value.
fn shim_to_api(expr: &str, ty: &TypeRef) -> String {
    match ty {
        TypeRef::String => format!("({expr}.map {{ String(cString: $0) }} ?? \"\")"),
        _ => expr.to_string(),
    }
}

fn params_api(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| format!("_ {}: {}", snake_case(&p.name.0), api_type(&p.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn params_shim(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| format!("_ {}: {}", snake_case(&p.name.0), shim_type(&p.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn args(params: &[Param]) -> String {
    params
        .iter()
        .map(|p| snake_case(&p.name.0))
        .collect::<Vec<_>>()
        .join(", ")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
        None => String::new(),
    }
}

fn access_str(a: Access) -> &'static str {
    match a {
        Access::Read => "read",
        Access::Write => "write",
        Access::ReadWrite => "read/write",
    }
}

/// Emit the Swift glue for `schema`: shims file + class file.
pub fn generate(schema: &ComponentSchema) -> Vec<super::GeneratedFile> {
    let prefix = symbol_prefix(schema);
    let name = &schema.name.0;

    // Which mirror structs must be emitted, and their element API types.
    let mut need_bytes = false;
    let mut elem_by_suffix: BTreeMap<String, String> = BTreeMap::new();
    for ty in all_type_refs(schema) {
        walk_type(ty, &mut |t| {
            if let TypeRef::Vec(inner) = t {
                elem_by_suffix.insert(mirror_suffix(inner), api_type(inner));
            }
            if matches!(t, TypeRef::Bytes) {
                need_bytes = true;
            }
        });
    }
    let slices: BTreeSet<String> = elem_by_suffix.keys().cloned().collect();

    // --- shims file ---
    let mut shims = String::new();
    let _ = writeln!(shims, "// Generated by floom-codegen. DO NOT EDIT.");
    let _ = writeln!(shims, "// {}", banner(schema));
    let _ = writeln!(
        shims,
        "// C-layout mirror types and `@_silgen_name` bindings to the C ABI."
    );
    let _ = writeln!(shims, "import Foundation");
    let _ = writeln!(shims);
    if need_bytes {
        let _ = writeln!(shims, "/// Mirror of `floom_bytes`.");
        let _ = writeln!(
            shims,
            "public struct FloomBytes {{ public var data: UnsafePointer<UInt8>?; public var len: Int }}"
        );
        let _ = writeln!(shims);
    }
    for suffix in &slices {
        let elem = &elem_by_suffix[suffix];
        let _ = writeln!(
            shims,
            "/// Mirror of `floom_slice_{}`.",
            mangle_of_suffix(suffix)
        );
        let _ = writeln!(
            shims,
            "public struct FloomSlice{suffix} {{ public var data: UnsafePointer<{elem}>?; public var len: Int }}"
        );
        let _ = writeln!(shims);
    }
    let _ = writeln!(shims, "@_silgen_name(\"{prefix}_destroy\")");
    let _ = writeln!(shims, "func _{prefix}_destroy(_ handle: OpaquePointer?)");
    let _ = writeln!(shims);
    for p in &schema.properties {
        let n = snake_case(&p.name.0);
        if matches!(p.access, Access::Write | Access::ReadWrite) {
            let _ = writeln!(shims, "@_silgen_name(\"{prefix}_set_{n}\")");
            let _ = writeln!(
                shims,
                "func _{prefix}_set_{n}(_ handle: OpaquePointer?, _ value: {})",
                shim_type(&p.ty)
            );
            let _ = writeln!(shims);
        }
        if matches!(p.access, Access::Read | Access::ReadWrite) {
            let _ = writeln!(shims, "@_silgen_name(\"{prefix}_get_{n}\")");
            let _ = writeln!(
                shims,
                "func _{prefix}_get_{n}(_ handle: OpaquePointer?) -> {}",
                shim_type(&p.ty)
            );
            let _ = writeln!(shims);
        }
    }
    for c in &schema.commands {
        let n = snake_case(&c.name.0);
        let _ = writeln!(shims, "@_silgen_name(\"{prefix}_cmd_{n}\")");
        let ret = c
            .ret
            .as_ref()
            .map(|r| format!(" -> {}", shim_type(r)))
            .unwrap_or_default();
        let sep = if c.params.is_empty() { "" } else { ", " };
        let _ = writeln!(
            shims,
            "func _{prefix}_cmd_{n}(_ handle: OpaquePointer?{sep}{}){ret}",
            params_shim(&c.params)
        );
        let _ = writeln!(shims);
    }
    for e in &schema.events {
        let n = snake_case(&e.name.0);
        let extra: String = swift_event_params(&e.params)
            .iter()
            .map(|p| format!(", _ {}: {}", p.name, p.shim))
            .collect();
        let _ = writeln!(shims, "@_silgen_name(\"{prefix}_on_{n}\")");
        let _ = writeln!(
            shims,
            "func _{prefix}_on_{n}(_ handle: OpaquePointer?, _ callback: (@convention(c) (UnsafeMutableRawPointer?{extra}) -> Void)?, _ userdata: UnsafeMutableRawPointer?)"
        );
        let _ = writeln!(shims);
    }

    // --- class file ---
    let mut class = String::new();
    let _ = writeln!(class, "// Generated by floom-codegen. DO NOT EDIT.");
    let _ = writeln!(class, "// {}", banner(schema));
    let _ = writeln!(class, "import Foundation");
    let _ = writeln!(class);
    let _ = writeln!(class, "public final class {name} {{");
    let _ = writeln!(
        class,
        "  public static let abiMajor: UInt32 = {}",
        schema.abi.major
    );
    let _ = writeln!(
        class,
        "  public static let abiMinor: UInt32 = {}",
        schema.abi.minor
    );
    let _ = writeln!(class);
    let _ = writeln!(class, "  private let handle: OpaquePointer");
    let _ = writeln!(class);
    let _ = writeln!(
        class,
        "  public init(handle: OpaquePointer) {{ self.handle = handle }}"
    );
    let _ = writeln!(
        class,
        "  public func destroy() {{ _{prefix}_destroy(handle) }}"
    );

    for p in &schema.properties {
        let n = snake_case(&p.name.0);
        let api = api_type(&p.ty);
        let get = shim_to_api(&format!("_{prefix}_get_{n}(handle)"), &p.ty);
        let _ = writeln!(class);
        let _ = writeln!(
            class,
            "  // property `{}` ({})",
            p.name.0,
            access_str(p.access)
        );
        match p.access {
            Access::Read => {
                let _ = writeln!(class, "  public var {n}: {api} {{ {get} }}");
            }
            Access::Write => {
                let _ = writeln!(class, "  public var {n}: {api} {{");
                let _ = writeln!(
                    class,
                    "    get {{ fatalError(\"write-only property `{n}`\") }}"
                );
                let _ = writeln!(class, "    set {{ _{prefix}_set_{n}(handle, newValue) }}");
                let _ = writeln!(class, "  }}");
            }
            Access::ReadWrite => {
                let _ = writeln!(class, "  public var {n}: {api} {{");
                let _ = writeln!(class, "    get {{ {get} }}");
                let _ = writeln!(class, "    set {{ _{prefix}_set_{n}(handle, newValue) }}");
                let _ = writeln!(class, "  }}");
            }
        }
    }

    for c in &schema.commands {
        let n = snake_case(&c.name.0);
        let _ = writeln!(class);
        let _ = writeln!(class, "  // command `{}`", c.name.0);
        let sep = if c.params.is_empty() { "" } else { ", " };
        let call = format!("_{prefix}_cmd_{n}(handle{sep}{})", args(&c.params));
        match &c.ret {
            Some(r) => {
                let _ = writeln!(
                    class,
                    "  public func {n}({}) -> {} {{ return {} }}",
                    params_api(&c.params),
                    api_type(r),
                    shim_to_api(&call, r)
                );
            }
            None => {
                let _ = writeln!(
                    class,
                    "  public func {n}({}) {{ {call} }}",
                    params_api(&c.params)
                );
            }
        }
    }

    for e in &schema.events {
        let n = snake_case(&e.name.0);
        let cap = capitalize(&n);
        let ev = swift_event_params(&e.params);
        // A function type may not carry argument labels, so only the types.
        let cb_args = ev
            .iter()
            .map(|p| p.closure.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let lambda_bind: String = ev.iter().map(|p| format!(", {}", p.name)).collect();
        let call_args = ev
            .iter()
            .map(|p| p.arg.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(class);
        let _ = writeln!(class, "  // event `{}`", e.name.0);
        let _ = writeln!(class, "  public var on{cap}: (({cb_args}) -> Void)? {{");
        let _ = writeln!(class, "    didSet {{ register{cap}() }}");
        let _ = writeln!(class, "  }}");
        let _ = writeln!(class);
        let _ = writeln!(class, "  private func register{cap}() {{");
        let _ = writeln!(
            class,
            "    _{prefix}_on_{n}(handle, {{ userdata{lambda_bind} in"
        );
        let _ = writeln!(class, "      guard let userdata else {{ return }}");
        let _ = writeln!(
            class,
            "      let me = Unmanaged<{name}>.fromOpaque(userdata).takeUnretainedValue()"
        );
        let _ = writeln!(class, "      me.on{cap}?({call_args})");
        let _ = writeln!(class, "    }}, Unmanaged.passUnretained(self).toOpaque())");
        let _ = writeln!(class, "  }}");
    }

    let _ = writeln!(class, "}}");

    vec![
        super::GeneratedFile::new(format!("{name}Shims.swift"), shims),
        super::GeneratedFile::new(format!("{name}.swift"), class),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    fn file(files: &[super::super::GeneratedFile], path: &str) -> String {
        files
            .iter()
            .find(|f| f.path == path)
            .unwrap_or_else(|| panic!("missing {path}"))
            .contents
            .clone()
    }

    #[test]
    fn class_exposes_properties_commands_events() {
        let s = fixtures::demo_video_player();
        let files = generate(&s);
        let class = file(&files, "VideoPlayer.swift");
        assert!(class.contains("public final class VideoPlayer {"));
        assert!(class.contains("public static let abiMajor: UInt32 = 1"));
        assert!(class.contains("public var src: String {"));
        assert!(class.contains("public func seek(_ ms: UInt64)"));
        assert!(class.contains(
            "public func position() -> UInt64 { return _floom_video_player_cmd_position(handle) }"
        ));
        assert!(class.contains("public var onEnded: (() -> Void)? {"));
        assert!(class.contains("Unmanaged<VideoPlayer>.fromOpaque(userdata)"));
    }

    #[test]
    fn shims_bind_the_c_symbols() {
        let s = fixtures::demo_video_player();
        let shims = file(&generate(&s), "VideoPlayerShims.swift");
        assert!(shims.contains("@_silgen_name(\"floom_video_player_set_src\")"));
        assert!(shims.contains("_ value: UnsafePointer<CChar>?"));
        assert!(shims.contains(
            "@convention(c) (UnsafeMutableRawPointer?, _ message: UnsafePointer<CChar>?) -> Void"
        ));
    }

    #[test]
    fn parameterised_event_lambda_takes_extra_binding() {
        let s = fixtures::demo_video_player();
        let class = file(&generate(&s), "VideoPlayer.swift");
        assert!(class.contains("{ userdata, message in"));
    }

    #[test]
    fn wide_schema_emits_mirror_structs() {
        let s = fixtures::wide_type_coverage();
        let shims = file(&generate(&s), "WideProbeShims.swift");
        assert!(shims.contains("public struct FloomBytes"));
        assert!(shims.contains(
            "public struct FloomSliceF32 { public var data: UnsafePointer<Float>?; public var len: Int }"
        ));
        // value-shaped Option drops the `?` at the shim boundary
        assert!(
            shims.contains(
                "func _floom_wide_probe_get_maybe_count(_ handle: OpaquePointer?) -> Int32"
            )
        );
    }

    #[test]
    fn write_only_property_fatal_errors_on_get() {
        let s = fixtures::wide_type_coverage();
        let class = file(&generate(&s), "WideProbe.swift");
        assert!(class.contains("fatalError(\"write-only property `device`\")"));
    }
}
