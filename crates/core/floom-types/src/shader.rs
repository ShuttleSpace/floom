// SPDX-License-Identifier: MIT
//! Custom shader programs: a single GPU language (WGSL) the caller supplies.
//!
//! WGSL is the one shader language floom speaks. vello/wgpu compiles it
//! natively (no transpile — WGSL is wgpu's own shading language), a software
//! rasteriser cannot run it and so skips it and counts the skip, and skia
//! likewise degrades — the degradation is recorded, never silent. We
//! deliberately carry exactly one language rather than Compose's AGSL+SkSL
//! pair, which forces every effect to be written twice.

/// A custom shader program the caller supplies in WGSL.
///
/// The host owns the vertex stage (a fullscreen triangle) and a uniform block;
/// the caller writes only the fragment entry. The full module the host builds is
///
/// ```wgsl
/// struct VsOut { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
/// struct Uniforms {
///     resolution: vec2<f32>,
///     time: f32,
///     _pad: f32,
///     params: array<vec4<f32>, 2>,
/// };
/// @group(0) @binding(0) var<uniform> u: Uniforms;
/// @vertex fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut { /* fullscreen tri */ }
/// // --- caller source begins here ---
/// @fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { /* ... */ }
/// ```
///
/// so the caller's source is only the body of `fn fs_main`, and may name
/// `VsOut`, `Uniforms`, `u` and `vs_main` (they are already in scope). The
/// fragment receives:
///
/// * `in.uv` — normalised coordinates, `(0,0)` top-left to `(1,1)` bottom-right;
/// * `u.resolution` — the output size in device px;
/// * `u.time` — seconds since the shader was first drawn, driven by the host
///   clock (so the same on every machine, unlike a wall-clock);
/// * `u.params[0]` and `u.params[1]` — each a `vec4<f32>` of caller scalars.
///
/// The fragment must write **premultiplied** alpha (`rgb` already multiplied by
/// `a`), matching the compositor's `AlphaPremultiplied` expectation.
#[derive(Debug, Clone, PartialEq)]
pub struct Shader {
    /// Fragment entry source in WGSL. The host prepends the `VsOut` struct, the
    /// `Uniforms` block, its `@group(0) @binding(0)` binding and the vertex
    /// stage; this is only the body of `fn fs_main`.
    pub wgsl: String,
    /// Caller scalars, surfaced as `u.params[0]` and `u.params[1]` (two
    /// `vec4<f32>`, eight floats total). Longer input is truncated, shorter is
    /// zero-filled, by the host when it writes the uniform buffer.
    pub params: Vec<f32>,
}

impl Shader {
    /// A shader from its fragment source and caller parameters.
    pub fn new(wgsl: impl Into<String>, params: impl Into<Vec<f32>>) -> Self {
        Shader {
            wgsl: wgsl.into(),
            params: params.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shader_carries_its_source_and_params() {
        let s = Shader::new(
            "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
             return vec4<f32>(in.uv, 0.0, 1.0); }",
            [0.5, 1.0],
        );
        assert_eq!(s.params, vec![0.5, 1.0]);
        assert!(s.wgsl.contains("fs_main"));
    }
}
