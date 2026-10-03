//! A box whose ink is a program, not a shape (A5).
//!
//! Everything below is `floom2::prelude` plus one WGSL fragment. Three acts,
//! each asserting against what actually happened:
//!
//! 1. **The program is the box's ink.** A `ShaderBox` mounted like any other
//!    widget puts one shader node over *its own* laid-out box, carrying the
//!    WGSL and the caller's scalars untouched. Nothing is filled behind it —
//!    the shader is the element, not a decoration on one.
//! 2. **A CPU rasteriser cannot run WGSL, and says so.** The same scene,
//!    presented to the software renderer, leaves the box exactly as the
//!    background painted it and counts one skip. That counter is the whole
//!    point of this act: an empty box is indistinguishable from a shader that
//!    happened to draw nothing, so the degradation has to be *reported* rather
//!    than merely be. (Pixels are a GPU-path matter — see the notes.)
//! 3. **A scalar change is a repaint, not a re-layout.** Re-pointing the
//!    shader's params on the mounted widget moves the numbers and leaves the
//!    box where it was: `u.time` already animates a shader for free, so the
//!    only thing a caller has to push per frame is its own inputs.
//!
//! Run with `cargo run -p floom2-shader`.

use floom_renderer::{FrameInfo, render_once};
use floom2::floom_layout::{AbsoluteInset, Position};
use floom2::floom_scene::SceneNode;
use floom2::prelude::*;

const SURFACE_W: u32 = 240;
const SURFACE_H: u32 = 120;

/// The box: inset from the surface so act 1's "its own box, not the window's"
/// claim has something else it could have been.
const BOX_LEFT: f32 = 24.0;
const BOX_TOP: f32 = 20.0;
const BOX_W: f32 = 160.0;
const BOX_H: f32 = 72.0;

/// What the surface is painted before the shader gets a chance at it, so act 2
/// can tell "the shader drew nothing" from "nothing was drawn".
const BACKDROP: Color = Color::rgb(12, 24, 36);

/// A plasma the caller wrote. The host owns the vertex stage and the uniform
/// block; this is only `fs_main`, and it reads `in.uv` and `u.time` — the
/// clock advances one nominal frame per present, so this animates without the
/// widget rebuilding anything.
const PLASMA: &str = "
@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let p = in.uv * 2.0 - vec2<f32>(1.0, 1.0);
    let t = u.time * 0.6;
    let r = length(p);
    let wave = sin(p.x * 6.0 + t) * cos(p.y * 6.0 - t);
    let band = 0.5 + 0.5 * wave;
    let fade = 1.0 - smoothstep(0.2, 1.2, r);
    let rgb = vec3<f32>(
        0.15 + 0.75 * band,
        0.25 + 0.35 * (1.0 - band),
        0.85 - 0.45 * band,
    );
    // Premultiplied, as the compositor expects.
    return vec4<f32>(rgb * fade, fade);
}
";

fn main() {
    act_1_program_reaches_the_scene();
    act_2_the_cpu_rasteriser_says_so();
    act_3_scalars_repaint_without_relayout();
    println!("\nfloom2-shader: 3 acts, all held");
}

/// A backdrop-coloured surface holding one 160×72 shader box. Returns the
/// box's own id as well, so act 3 can reach the *mounted* widget — a builder
/// is cloned into the tree, and mutating the caller's copy paints nothing.
fn mount_box() -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(SURFACE_W as f64, SURFACE_H as f64));
    let root = host.mount_root(
        LayoutStyle::default(),
        Container::new().background(BACKDROP),
    );
    let id = host.mount(root, box_style(), ShaderBox::wgsl(PLASMA, [0.0, 1.0]));
    host.frame();
    settle(&mut host);
    (host, id)
}

fn box_style() -> LayoutStyle {
    LayoutStyle {
        position: Position::Absolute,
        inset: AbsoluteInset {
            left: Some(BOX_LEFT),
            top: Some(BOX_TOP),
            ..AbsoluteInset::default()
        },
        width: Some(BOX_W),
        height: Some(BOX_H),
        ..LayoutStyle::default()
    }
}

fn settle(host: &mut WidgetHost) {
    for _ in 0..20 {
        if !host.needs_frame() {
            return;
        }
        host.frame();
    }
}

/// The one shader node the scene holds: its box and its program.
fn shader_of(host: &WidgetHost) -> (Rect, Shader) {
    let found: Vec<_> = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Shader { bounds, shader } => Some((*bounds, shader.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "the box declares exactly one program");
    found[0].clone()
}

fn act_1_program_reaches_the_scene() {
    let (host, _id) = mount_box();
    let (bounds, shader) = shader_of(&host);

    // The box a shader covers is the box layout gave the element. A shader
    // that quietly claimed the window would paint over everything beside it.
    assert_eq!(
        bounds.x0, BOX_LEFT as f64,
        "the box starts where it was put"
    );
    assert_eq!(bounds.y0, BOX_TOP as f64);
    assert_eq!(bounds.width(), BOX_W as f64, "and is exactly as big");
    assert_eq!(bounds.height(), BOX_H as f64);

    // What it carries is what the caller wrote: the source is not rewritten
    // and the scalars are not reordered on the way through.
    assert_eq!(shader.wgsl, PLASMA, "the WGSL survives the widget door");
    assert_eq!(shader.params, vec![0.0, 1.0], "and so do the scalars");

    println!(
        "act 1  a {}x{} box carries {} bytes of WGSL and {} scalars, over its own box",
        bounds.width(),
        bounds.height(),
        shader.wgsl.len(),
        shader.params.len()
    );
}

fn act_2_the_cpu_rasteriser_says_so() {
    let (host, _id) = mount_box();
    let mut sw = floom2::floom_renderer_software::SoftwareRenderer::new(SURFACE_W, SURFACE_H);
    render_once(
        &mut sw,
        &FrameInfo::software(SURFACE_W, SURFACE_H, 1),
        host.scene(),
    );

    // No WGSL on a CPU rasteriser — and no invented ink either. A backend that
    // painted a flat grey here would look like a shader that produced grey,
    // which is worse than the absence.
    let mid_x = (BOX_LEFT + BOX_W / 2.0) as u32;
    let mid_y = (BOX_TOP + BOX_H / 2.0) as u32;
    let px = sw.pixel_rgba(mid_x, mid_y).expect("probe inside surface");
    assert_eq!(
        [px[0], px[1], px[2]],
        [BACKDROP.r, BACKDROP.g, BACKDROP.b],
        "the rasteriser invented no ink for a program it cannot run"
    );
    assert_eq!(
        sw.shaders_skipped, 1,
        "and the degradation is counted, not silent"
    );
    println!(
        "act 2  the software renderer drew no ink and counted {} skip — the box is \
         empty because the program was dropped, not because it ran",
        sw.shaders_skipped
    );
}

fn act_3_scalars_repaint_without_relayout() {
    let (mut host, id) = mount_box();
    let before = shader_of(&host);
    assert_eq!(before.1.params, vec![0.0, 1.0]);

    host.widget_mut::<ShaderBox>(id)
        .expect("the mounted box")
        .set_params([0.5, 0.25]);
    settle(&mut host);

    let after = shader_of(&host);
    assert_eq!(after.1.params, vec![0.5, 0.25], "the scalars were re-read");
    assert_eq!(after.1.wgsl, PLASMA, "the source was left alone");
    assert_eq!(
        after.0, before.0,
        "and the box did not move: a param change is a repaint, not a re-layout"
    );
    println!(
        "act 3  params went [0.0, 1.0] -> [0.5, 0.25] over the same {}x{} box",
        after.0.width(),
        after.0.height()
    );
}
