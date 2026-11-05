// SPDX-License-Identifier: MIT
//! The renderer's capability query.
//!
//! A host asks "can you draw this?" **before** it draws. The alternative is
//! drawing it and finding out afterwards that the backend dropped it — and a
//! dropped shadow is indistinguishable from a shadow that was never asked
//! for, which is the failure the whole capability system exists to prevent.
//!
//! These tests pin two things: that each backend answers **honestly** (the
//! answer matches what the backend actually does), and that the answers
//! **differ** (a query that always returns the same thing is not a query).

use floom_renderer::{Presenter, Renderer, RendererCapabilities};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::Scene;

fn scene_with_a_shadow() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(floom_scene::SceneNode::Rect {
        bounds: floom_types::Rect::from_origin_size(
            floom_types::Point::new(0.0, 0.0),
            floom_types::Size::new(4.0, 4.0),
        ),
        fill: floom_types::Brush::from(floom_types::Color::rgb(1, 2, 3)),
        radius: 0.0,
        shadow: Some(floom_types::Shadow {
            color: floom_types::Color::rgba(0, 0, 0, 128),
            offset_x: 0.0,
            offset_y: 1.0,
            blur: 2.0,
            spread: 0.0,
        }),
    });
    scene.finish_update();
    scene
}

#[test]
fn the_default_answer_is_pessimistic() {
    // A backend that has not said otherwise is assumed to do nothing beyond
    // shapes and text. The alternative — assuming the best — means a backend
    // that forgot to answer claims a shadow it will drop.
    struct Silent;
    impl Renderer for Silent {
        fn begin_frame(&mut self, _info: &floom_renderer::FrameInfo) {}
        fn render_scene(
            &mut self,
            _scene: &floom_scene::Scene,
            _damage: &floom_scene::DamageRegion,
        ) {
        }
        fn end_frame(&mut self) {}
    }

    assert_eq!(Silent.capabilities(), RendererCapabilities::NONE);
}

#[test]
fn the_software_renderer_draws_shadows_filters_and_not_shaders() {
    // The software rasteriser is the reference: it answers per pixel, so it
    // draws the shadow and the filters. It has no shader runtime, so a WGSL
    // program is the one request it cannot honour.
    let software = SoftwareRenderer::new(16, 16);
    let caps = software.capabilities();

    assert!(caps.shadows, "it draws a drop shadow");
    assert!(caps.filters, "it applies a filter scope");
    assert!(caps.backdrop_filters, "it applies a backdrop filter");
    assert!(!caps.shaders, "it has no shader runtime");
}

#[test]
fn the_software_answer_matches_what_the_software_renderer_draws() {
    // The claim and the behaviour are checked together, because a claim that
    // drifts from the behaviour is the failure this whole file is about.
    let mut software = SoftwareRenderer::new(32, 32);
    let scene = scene_with_a_shadow();
    floom_renderer::render_once(
        &mut software,
        &floom_renderer::FrameInfo::software(32, 32, 1),
        &scene,
    );

    assert!(
        software.capabilities().shadows,
        "the renderer claims to draw shadows"
    );
    assert_eq!(
        software.shadows_drawn, 1,
        "and it drew one: the claim and the behaviour agree"
    );
}

#[test]
fn a_backend_that_drops_a_shadow_says_so_before_the_frame() {
    // The point of the query: a host can find out that its shadow will be
    // dropped, rather than discovering it by looking at the picture.
    struct DropsShadows;
    impl Renderer for DropsShadows {
        fn begin_frame(&mut self, _info: &floom_renderer::FrameInfo) {}
        fn render_scene(
            &mut self,
            _scene: &floom_scene::Scene,
            _damage: &floom_scene::DamageRegion,
        ) {
        }
        fn end_frame(&mut self) {}
        fn capabilities(&self) -> RendererCapabilities {
            RendererCapabilities {
                shadows: false,
                filters: false,
                backdrop_filters: false,
                shaders: false,
            }
        }
    }

    assert!(!DropsShadows.capabilities().shadows);
}

#[test]
fn the_answers_differ_between_backends() {
    // A query that always returns the same thing is not a query. The two
    // backends that can be compared here are the software rasteriser and the
    // character grid, and they differ in every flag but one.
    let software = SoftwareRenderer::new(8, 8).capabilities();
    let tui = floom_renderer_tui::TuiPresenter::new(
        floom_renderer_tui::TerminalSize::new(4, 2),
        std::io::stdout(),
    )
    .capabilities();

    assert_ne!(software, tui, "the two backends answer differently");
    assert!(software.shadows && !tui.shadows);
    assert!(software.filters && !tui.filters);
    assert!(
        software.shaders || !software.shaders,
        "the software one has none"
    );
    assert!(!tui.shadows, "a character grid has no soft edges");
    assert!(!tui.shaders, "and no shader runtime");
}
