// SPDX-License-Identifier: MIT
//! Reference component schemas.
//!
//! These are canonical, hand-written examples used by the schema tests, the
//! backend tests and the compile fixtures. They are public because they double
//! as the documented example of what a native component schema looks like.

use crate::codegen::schema::{
    AbiVersion, Access, CommandSchema, ComponentSchema, Composition, EventSchema, Ident, InputMode,
    Literal, Param, PropertySchema, TypeRef, ViewKind,
};

fn id(s: &str) -> Ident {
    Ident::new(s).expect("fixture identifiers are valid")
}

/// A video player: read/write properties, a no-arg command, a parameterised
/// command, and both a no-arg and a parameterised event.
///
/// This is the schema the compile fixtures render, so it deliberately uses only
/// types whose C↔Swift/C++ mappings are unambiguous and ABI-stable.
pub fn demo_video_player() -> ComponentSchema {
    ComponentSchema {
        name: id("VideoPlayer"),
        abi: AbiVersion { major: 1, minor: 0 },
        kind: ViewKind::Video,
        composition: Composition::Texture,
        input: InputMode::Passthrough,
        properties: vec![
            PropertySchema {
                name: id("src"),
                ty: TypeRef::String,
                access: Access::ReadWrite,
                default: Some(Literal::String("".into())),
            },
            PropertySchema {
                name: id("muted"),
                ty: TypeRef::Bool,
                access: Access::ReadWrite,
                default: Some(Literal::Bool(false)),
            },
            PropertySchema {
                name: id("volume"),
                ty: TypeRef::F64,
                access: Access::ReadWrite,
                default: Some(Literal::Float(1.0)),
            },
        ],
        commands: vec![
            CommandSchema {
                name: id("play"),
                params: vec![],
                ret: None,
            },
            CommandSchema {
                name: id("seek"),
                params: vec![Param {
                    name: id("ms"),
                    ty: TypeRef::U64,
                }],
                ret: None,
            },
            CommandSchema {
                name: id("position"),
                params: vec![],
                ret: Some(TypeRef::U64),
            },
        ],
        events: vec![
            EventSchema {
                name: id("ended"),
                params: vec![],
            },
            EventSchema {
                name: id("error"),
                params: vec![Param {
                    name: id("message"),
                    ty: TypeRef::String,
                }],
            },
        ],
    }
}

/// A schema that exercises every [`TypeRef`] shape, including the ones the
/// compile fixtures do not render (bytes, slices, options, handles, resources
/// and user-defined types). Used to pin the exact type mappings per backend.
pub fn wide_type_coverage() -> ComponentSchema {
    ComponentSchema {
        name: id("WideProbe"),
        abi: AbiVersion { major: 2, minor: 3 },
        kind: ViewKind::Custom("probe".into()),
        composition: Composition::Offscreen,
        input: InputMode::Capture,
        properties: vec![
            PropertySchema {
                name: id("payload"),
                ty: TypeRef::Bytes,
                access: Access::ReadWrite,
                default: None,
            },
            PropertySchema {
                name: id("texture"),
                ty: TypeRef::Resource(id("TextureId")),
                access: Access::ReadWrite,
                default: None,
            },
            PropertySchema {
                name: id("native"),
                ty: TypeRef::Handle(id("OsView")),
                access: Access::ReadWrite,
                default: None,
            },
            PropertySchema {
                name: id("maybe_count"),
                ty: TypeRef::Option(Box::new(TypeRef::I32)),
                access: Access::Read,
                default: None,
            },
            PropertySchema {
                name: id("samples"),
                ty: TypeRef::Vec(Box::new(TypeRef::F32)),
                access: Access::Read,
                default: None,
            },
            PropertySchema {
                name: id("device"),
                ty: TypeRef::Custom(id("GpuDevice")),
                access: Access::Write,
                default: None,
            },
        ],
        commands: vec![CommandSchema {
            name: id("submit"),
            params: vec![Param {
                name: id("samples"),
                ty: TypeRef::Vec(Box::new(TypeRef::F32)),
            }],
            ret: Some(TypeRef::Bytes),
        }],
        events: vec![EventSchema {
            name: id("frame"),
            params: vec![Param {
                name: id("pixels"),
                ty: TypeRef::Bytes,
            }],
        }],
    }
}
