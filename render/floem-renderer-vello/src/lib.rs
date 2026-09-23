//! Vello-oriented Scene adapter.
//!
//! Two layers live here:
//!
//! * [`VelloRendererAdapter`] — a dependency-free recorder that turns a
//!   [`Scene`] into a [`VelloCommand`] list. It freezes the ABI boundary and is
//!   what the default build compiles.
//! * [`VelloGpuPresenter`] (feature `gpu`) — the real thing: a wgpu
//!   swap-chain driven by vello, which is what actually puts pixels in a window.

pub mod budget;

#[cfg(feature = "gpu")]
mod gpu;

#[cfg(feature = "gpu")]
pub use budget::{CacheTrimTelemetry, GpuCacheBudget, MemoryPressure};
#[cfg(feature = "gpu")]
pub use gpu::{GpuFrameStats, VelloGpuPresenter};

use floem_renderer::{FrameInfo, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
use floem_types::{Filter, Rect};

/// Recorded draw command (Vello backend consumes this later).
#[derive(Debug, Clone, PartialEq)]
pub enum VelloCommand {
    FillRect {
        bounds: Rect,
        /// What the rect is filled with — a solid or a gradient brush. The
        /// brush is carried as the scene's own vocabulary; the GPU encoder is
        /// where it becomes a peniko gradient.
        brush: floem_types::Brush,
        /// Corner radius in logical px; `0.0` is a plain right-angle rect.
        radius: f32,
        /// The drop shadow requested behind the fill, if any. Vello has no
        /// blur primitive yet, so the encoder skips it and counts the skip —
        /// recorded here so the request is never silently dropped.
        shadow: Option<floem_types::Shadow>,
    },
    DrawImage {
        bounds: Rect,
    },
    DrawGlyphs {
        bounds: Rect,
    },
    /// A filled path. Bounds alone would let two different shapes collide,
    /// so the segment count rides along as a cheap identity check.
    FillPath {
        bounds: Rect,
        segments: usize,
    },
    /// A stroked path; `width` is in logical px.
    StrokePath {
        bounds: Rect,
        width: f32,
    },
    /// Opens a filter scope. Recorded so the request is visible at this ABI
    /// even though the GPU encoder cannot honour it — the same reasoning that
    /// keeps a rect's `shadow` on the command instead of dropping it.
    PushFilter {
        filter: Filter,
    },
    PopFilter,
    /// A filter over the pixels already drawn inside `rect`.
    BackdropFilter {
        rect: Rect,
        filter: Filter,
    },
    /// A custom WGSL shader filling `bounds`. Recorded so the request is
    /// visible at this ABI even though only the GPU encoder can honour it — the
    /// same reasoning that keeps a rect's `shadow` and a `FilterPush` on the
    /// command list instead of dropping them: a backend quietly dropping it
    /// would leave a custom effect reading as plain transparency with nothing
    /// to explain why.
    Shader {
        bounds: Rect,
        shader: floem_types::Shader,
    },
    /// Opens a clip. Every command recorded after it is confined to
    /// `bounds` until the matching [`VelloCommand::PopClip`].
    PushClip {
        bounds: Rect,
    },
    /// Closes the innermost clip.
    PopClip,
    /// Opens a transform scope: every command recorded after it is drawn
    /// through `transform` until the matching [`VelloCommand::PopTransform`].
    PushTransform {
        transform: floem_types::Transform,
    },
    /// Closes the innermost transform scope.
    PopTransform,
}

#[derive(Debug, Default)]
pub struct VelloRendererAdapter {
    pub commands: Vec<VelloCommand>,
    pub frames: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    /// Simple fingerprint cache of last full-frame command count.
    last_cmd_len: usize,
}

impl VelloRendererAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> &[VelloCommand] {
        &self.commands
    }
}

impl Renderer for VelloRendererAdapter {
    fn begin_frame(&mut self, _info: &FrameInfo) {
        self.commands.clear();
    }

    fn render_scene(&mut self, scene: &Scene, damage: &DamageRegion) {
        if damage.is_empty() && self.last_cmd_len > 0 {
            self.cache_hits += 1;
            return;
        }
        self.cache_misses += 1;
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                    shadow,
                } => {
                    self.commands.push(VelloCommand::FillRect {
                        bounds: *bounds,
                        brush: fill.clone(),
                        radius: radius.max(0.0),
                        shadow: *shadow,
                    });
                }
                SceneNode::Image { bounds, .. } => {
                    self.commands
                        .push(VelloCommand::DrawImage { bounds: *bounds });
                }
                SceneNode::Text { origin, size, .. } => {
                    self.commands.push(VelloCommand::DrawGlyphs {
                        bounds: Rect::from_origin_size(*origin, *size),
                    });
                }
                SceneNode::GroupPush { transform } => {
                    self.commands.push(VelloCommand::PushTransform {
                        transform: *transform,
                    });
                }
                SceneNode::GroupPop => self.commands.push(VelloCommand::PopTransform),
                SceneNode::Path { path, fill, stroke } => {
                    let bounds = path.bounds();
                    if let Some(fill) = fill {
                        self.commands.push(VelloCommand::FillPath {
                            bounds: Rect::from_origin_size(bounds.origin(), bounds.size()),
                            segments: path.segments().len(),
                        });
                        let _ = fill;
                    }
                    if let Some(stroke) = stroke {
                        self.commands.push(VelloCommand::StrokePath {
                            bounds: Rect::from_origin_size(bounds.origin(), bounds.size()),
                            width: stroke.width as f32,
                        });
                    }
                }
                SceneNode::ClipPush { region } => {
                    self.commands.push(VelloCommand::PushClip {
                        bounds: region.bounds(),
                    });
                }
                SceneNode::ClipPop => self.commands.push(VelloCommand::PopClip),
                SceneNode::FilterPush { filter } => self
                    .commands
                    .push(VelloCommand::PushFilter { filter: *filter }),
                SceneNode::FilterPop => self.commands.push(VelloCommand::PopFilter),
                SceneNode::BackdropFilter { rect, filter } => {
                    self.commands.push(VelloCommand::BackdropFilter {
                        rect: *rect,
                        filter: *filter,
                    })
                }
                SceneNode::Shader { bounds, shader } => self.commands.push(VelloCommand::Shader {
                    bounds: *bounds,
                    shader: shader.clone(),
                }),
            }
        }
        self.last_cmd_len = self.commands.len();
    }

    fn end_frame(&mut self) {
        self.frames += 1;
    }

    fn trim_caches(&mut self) {
        self.last_cmd_len = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_renderer::{FrameInfo, render_once};
    use floem_scene::{Point, SceneNode, Size};
    use floem_types::Color;

    #[test]
    fn records_rect_image_text_fast_paths() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: 0.0,
            shadow: None,
        });
        let _ = scene.insert(SceneNode::image(
            Rect::from_origin_size(Point::ZERO, Size::new(8.0, 8.0)),
            floem_types::ResourceId::new(0, 1),
        ));
        let _ = scene.insert(SceneNode::Text {
            origin: Point::ZERO,
            size: Size::new(20.0, 12.0),
            glyphs: floem_types::ResourceId::new(1, 1),
            color: Color::WHITE,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        assert_eq!(vello.commands.len(), 3);
        assert!(matches!(vello.commands[0], VelloCommand::FillRect { .. }));
        assert!(matches!(vello.commands[1], VelloCommand::DrawImage { .. }));
        assert!(matches!(vello.commands[2], VelloCommand::DrawGlyphs { .. }));
        assert_eq!(vello.cache_misses, 1);
    }

    /// A filter request reaches the headless command list intact.
    ///
    /// Vello has no blur primitive, so the GPU encoder skips it and counts the
    /// skip — but the request has to be *visible* at this ABI, exactly as a
    /// rect's shadow is: a backend quietly dropping it would leave a frosted
    /// panel reading as plain transparency with nothing to explain why.
    #[test]
    fn records_filter_requests_instead_of_dropping_them() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::FilterPush {
            filter: floem_types::Filter::blur(3.0),
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(4.0, 4.0), Size::new(8.0, 8.0)),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: 0.0,
            shadow: None,
        });
        let _ = scene.insert(SceneNode::FilterPop);
        let _ = scene.insert(SceneNode::BackdropFilter {
            rect: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(12.0, 12.0)),
            filter: floem_types::Filter::blur_xy(5.0, 1.0),
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        assert_eq!(
            vello.commands,
            vec![
                VelloCommand::PushFilter {
                    filter: floem_types::Filter::blur(3.0)
                },
                VelloCommand::FillRect {
                    bounds: Rect::from_origin_size(Point::new(4.0, 4.0), Size::new(8.0, 8.0)),
                    brush: floem_types::Brush::from(Color::BLACK),
                    radius: 0.0,
                    shadow: None,
                },
                VelloCommand::PopFilter,
                VelloCommand::BackdropFilter {
                    rect: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(12.0, 12.0)),
                    filter: floem_types::Filter::blur_xy(5.0, 1.0),
                },
            ],
            "the scope brackets its draw and the backdrop keeps its own rect"
        );
    }

    /// A transform scope reaches the headless command list as a
    /// push/pop pair bracketing the draws it wraps, carrying the transform.
    #[test]
    fn records_a_transform_scope_bracketing_its_draws() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::GroupPush {
            transform: floem_types::Transform::scale_around(
                2.0,
                2.0,
                floem_types::Point::new(16.0, 16.0),
            ),
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(4.0, 4.0), Size::new(8.0, 8.0)),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: 0.0,
            shadow: None,
        });
        let _ = scene.insert(SceneNode::GroupPop);
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        assert!(
            matches!(
                vello.commands.as_slice(),
                [
                    VelloCommand::PushTransform { transform },
                    VelloCommand::FillRect { .. },
                    VelloCommand::PopTransform,
                ] if *transform
                    == floem_types::Transform::scale_around(
                        2.0,
                        2.0,
                        floem_types::Point::new(16.0, 16.0)
                    )
            ),
            "the scope brackets its draws and carries the transform: {:?}",
            vello.commands
        );
    }

    #[test]
    fn empty_damage_can_hit_cache() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(1.0, 1.0)),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: 0.0,
            shadow: None,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(8, 8, 1), &scene);
        scene.clear_damage();
        render_once(&mut vello, &FrameInfo::software(8, 8, 2), &scene);
        assert!(vello.cache_hits >= 1);
    }

    /// The adapter forwards the corner radius verbatim so the GPU presenter
    /// can shape corners from the command list alone.
    #[test]
    fn fill_rect_commands_carry_their_radius() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: 6.0,

            shadow: None,
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 20.0), Size::new(10.0, 10.0)),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: -3.0, // clamped to 0 at the adapter boundary
            shadow: None,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        let [a, b] = vello.commands.as_slice() else {
            panic!("expected exactly two commands");
        };
        let (VelloCommand::FillRect { radius: ra, .. }, VelloCommand::FillRect { radius: rb, .. }) =
            (a, b)
        else {
            panic!("expected fill-rect commands, got {a:?} / {b:?}");
        };
        assert_eq!(*ra, 6.0);
        assert_eq!(*rb, 0.0, "negative radius is clamped");
    }

    /// A gradient crosses the adapter boundary as the scene's own brush — not
    /// collapsed to one sampled colour here, where the pre-gradient code would
    /// have had room to quietly degrade it. The GPU encoder is the only place
    /// allowed to turn a brush into pixels.
    #[test]
    fn a_gradient_reaches_the_command_list_as_a_brush() {
        let ramp = floem_types::Brush::linear(
            &[(0.0, Color::BLACK), (1.0, Color::WHITE)],
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
        );
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: ramp.clone(),
            radius: 0.0,
            shadow: None,
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        let VelloCommand::FillRect { brush, .. } = &vello.commands[0] else {
            panic!("expected a fill-rect command, got {:?}", vello.commands[0]);
        };
        assert_eq!(*brush, ramp, "the ramp is intact at the ABI boundary");
        assert!(brush.solid().is_none(), "and it is not one flat colour");
    }

    /// A shadow request is *carried*, not dropped, even though vello has no
    /// blur primitive yet. This is the "degradation is never silent" half of
    /// A4: the encoder counts the skip and draws the fill, but it can only do
    /// that if the request survives to reach it.
    #[test]
    fn a_shadow_request_is_carried_rather_than_dropped() {
        let shadow = floem_types::Shadow {
            color: Color::rgba(0, 0, 0, 128),
            blur: 6.0,
            offset_x: 0.0,
            offset_y: 4.0,
        };
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
            fill: floem_types::Brush::from(Color::WHITE),
            radius: 4.0,
            shadow: Some(shadow),
        });
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        let VelloCommand::FillRect {
            brush,
            radius,
            shadow: carried,
            ..
        } = &vello.commands[0]
        else {
            panic!("expected a fill-rect command, got {:?}", vello.commands[0]);
        };
        assert_eq!(*carried, Some(shadow), "the request survives the adapter");
        assert_eq!(*radius, 4.0);
        assert_eq!(brush.solid(), Some(Color::WHITE), "and so does the fill");
    }

    /// A custom-shader request reaches the headless command list intact.
    ///
    /// Only the GPU encoder can actually run WGSL; the adapter cannot, so it
    /// carries the request (the same "degradation is never silent" half of A5
    /// that a `FilterPush` gets). A backend dropping it would leave the effect
    /// reading as plain transparency with nothing to explain why.
    #[test]
    fn records_a_shader_request_instead_of_dropping_it() {
        let shader = floem_types::Shader::new(
            "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
             return vec4<f32>(in.uv, 0.0, 1.0); }",
            [0.5, 0.25],
        );
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::shader(
            Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(12.0, 12.0)),
            shader.clone(),
        ));
        scene.finish_update();

        let mut vello = VelloRendererAdapter::new();
        render_once(&mut vello, &FrameInfo::software(32, 32, 1), &scene);
        assert_eq!(vello.commands.len(), 1);
        let VelloCommand::Shader {
            bounds,
            shader: carried,
        } = &vello.commands[0]
        else {
            panic!("expected a shader command, got {:?}", vello.commands[0]);
        };
        assert_eq!(
            *bounds,
            Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(12.0, 12.0)),
            "the bounds survive"
        );
        assert_eq!(carried, &shader, "and so does the program");
        assert_eq!(carried.params, vec![0.5, 0.25]);
    }
}
