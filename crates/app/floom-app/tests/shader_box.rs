// SPDX-License-Identifier: MIT
//! A5 through the door a control can actually reach.
//!
//! The renderer-side tests prove a *scene* carrying `SceneNode::Shader` is
//! executed (vello) or counted (software, skia). What they cannot prove is
//! that an app can *produce* such a scene — a shader nobody can name from the
//! widget tree is a backend capability, not a feature. This file closes that
//! half: a `ShaderBox` is mounted like any other widget, and what it declares
//! is what the retained scene carries, over the box layout gave it.
//!
//! Two properties are asserted because they are the two ways this slice can
//! fail quietly:
//!
//! * **the request survives** — the wgsl and the caller scalars reach the
//!   scene untouched, over the element's own box rather than the viewport's;
//! * **the degradation is recorded** — the software backend cannot run WGSL,
//!   and says so with a counter instead of leaving an empty box behind.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_renderer::{AttachedSurface, Presenter};
use floom_renderer_software::SoftwarePresenter;
use floom_scene::SceneNode;
use floom_types::{Color, Length, Shader, Size, SurfaceId, WindowId};
use floom_widgets::{Container, ShaderBox};

const WGSL: &str = "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
     return vec4<f32>(in.uv.x, in.uv.y, 0.5, 1.0); }";

const WGSL_SWAP: &str = "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
     return vec4<f32>(0.0, 0.0, 0.0, 1.0); }";

/// A 300×200 window holding one 120×60 shader box. Returns the box's own id,
/// so a test can reach the mounted widget (the caller's builder is a copy).
fn host_with_shader_box() -> (WidgetHost, floom_app::NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(300.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(300.0)),
            height: Some(Length::Px(200.0)),
            ..LayoutStyle::default()
        },
        Container::new().background(Color::rgb(10, 10, 10)),
    );
    let shader = host.mount(
        root,
        LayoutStyle::default(),
        ShaderBox::wgsl(WGSL, [0.25, 0.5]).size(120.0, 60.0),
    );
    settle(&mut host);
    (host, shader)
}

fn settle(host: &mut WidgetHost) {
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert!(!host.needs_frame(), "the host did not settle");
}

/// The one `Shader` node the scene holds.
fn shader_of(host: &WidgetHost) -> (floom_types::Rect, Shader) {
    let found: Vec<_> = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Shader { bounds, shader } => Some((*bounds, shader.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(found.len(), 1, "expected exactly one shader node");
    found[0].clone()
}

#[test]
fn a_shader_box_carries_its_program_over_its_own_box() {
    let (host, _root) = host_with_shader_box();
    let (bounds, shader) = shader_of(&host);

    // The box a shader covers is the box layout gave the element, not the
    // window's — a shader that silently claimed the viewport would paint over
    // everything around it.
    assert_eq!(bounds.width(), 120.0, "width is the laid-out box");
    assert_eq!(bounds.height(), 60.0, "height is the laid-out box");
    assert_eq!(bounds.x0, 0.0);
    assert_eq!(bounds.y0, 0.0);

    // And what it carries is exactly what the caller wrote: neither the source
    // nor the scalars are rewritten on the way through.
    assert_eq!(shader.wgsl, WGSL, "the wgsl survives the widget door");
    assert_eq!(shader.params, vec![0.25, 0.5], "the params survive too");
}

#[test]
fn a_shader_the_software_backend_cannot_run_is_counted_not_silently_dropped() {
    let (mut host, _root) = host_with_shader_box();

    let mut presenter = SoftwarePresenter::new();
    assert!(presenter.attach(AttachedSurface::new(
        WindowId::new(0, 1),
        SurfaceId::new(0, 1),
        Size::new(300.0, 200.0),
        1.0,
        None,
    )));
    presenter.present(host.scene(), host.resources());

    // A CPU rasteriser cannot run WGSL. What it must not do is leave an empty
    // box and say nothing: the skip is counted, so an app can tell "my shader
    // is running" from "my shader was dropped".
    assert_eq!(
        presenter.renderer().shaders_skipped,
        1,
        "the degradation is recorded, not silent"
    );

    // The counter is per present, not cumulative across frames, so a second
    // identical frame reports the same one rather than two.
    host.frame();
    presenter.present(host.scene(), host.resources());
    assert_eq!(presenter.renderer().shaders_skipped, 1);
}

#[test]
fn replacing_the_program_on_the_mounted_box_repaints_the_new_source() {
    let (mut host, shader_id) = host_with_shader_box();
    assert_eq!(shader_of(&host).1.wgsl, WGSL);

    // The in-place door: a builder is cloned into the tree, so mutating the
    // caller's copy paints nothing. `widget_mut` reaches the mounted one.
    host.widget_mut::<ShaderBox>(shader_id)
        .expect("the mounted box")
        .set_shader(Shader::new(WGSL_SWAP, [0.0, 0.0]));
    settle(&mut host);

    let shader = shader_of(&host).1;
    assert_eq!(shader.wgsl, WGSL_SWAP, "the new source reached the scene");
    assert_eq!(shader.params, vec![0.0, 0.0], "and so did its params");
}

#[test]
fn changing_only_the_scalars_repaints_them() {
    let (mut host, shader_id) = host_with_shader_box();
    assert_eq!(shader_of(&host).1.params, vec![0.25, 0.5]);

    host.widget_mut::<ShaderBox>(shader_id)
        .expect("the mounted box")
        .set_params([0.75, 1.0]);
    settle(&mut host);

    let (bounds, shader) = shader_of(&host);
    assert_eq!(shader.params, vec![0.75, 1.0], "the scalars were re-read");
    // A param change is a repaint, not a re-layout: the box stays put.
    assert_eq!(bounds.width(), 120.0);
    assert_eq!(bounds.height(), 60.0);
    assert_eq!(shader.wgsl, WGSL, "the source itself was left alone");
}
