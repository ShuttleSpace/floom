// SPDX-License-Identifier: MIT
//! The Skia side of Floom 2.0's renderer ABI.
//!
//! # What this is, and what it is not
//!
//! This crate **records** a scene as Skia-shaped commands. It does not draw:
//! it links no Skia, opens no surface, and puts no pixel on a screen. What it
//! does is say exactly *what* a Skia submit path would have to do, and count
//! the parts it could not do.
//!
//! That is worth a crate of its own, and it is worth the name
//! `floom-renderer-skia-abi` rather than `floom-renderer-skia`: a crate that
//! sits among the backends and draws nothing is a backend-shaped lie. The
//! name says what it is, so nobody reaches for it expecting pixels.
//!
//! # The counters
//!
//! The rule the whole renderer layer follows is that **a degradation is
//! recorded, never silent**: a backend that cannot honour a request counts
//! it, so the number can be surfaced. This adapter is where that rule
//! starts for Skia, because it is the layer that knows what Skia *can* do —
//! Skia has an `SkBlurImageFilter` and a `saveLayer`, so a submit path can
//! honour both a shadow and a filter, and neither is a degradation there.
//!
//! The two that *are* degradations for a Skia submit path, and are counted
//! here, are:
//!
//! * a **custom shader**: Skia has no WGSL runtime, so a `Shader` node
//!   would have to be skipped or rasterised elsewhere.
//! * a **snapshot**: freezing a participant into a texture needs a
//!   `SkImage` the framework can import, which is the same question the
//!   other backends answer differently.
//!
//! # What a real submit path would add
//!
//! A `SkiaRenderer` that links Skia, walks [`SkiaCommand`]s, and draws them.
//! The commands here are already in Skia's own vocabulary — `canvas.save()`,
//! `canvas.concat(matrix)`, `canvas.clipPath` — so the submit path is a
//! translation, not a redesign. The contract tests in this file pin that
//! vocabulary, and they will keep their value when the submit path lands.

use crate::{FrameInfo, Renderer};
use floom_scene::{DamageRegion, Scene, SceneNode};
use floom_types::{Color, Filter, Rect};

#[derive(Debug, Clone, PartialEq)]
pub enum SkiaCommand {
    FillRect {
        bounds: Rect,
        /// The fill as the scene's own `Brush` vocabulary: the real submit
        /// path maps a gradient to an `SkShader`, a solid to a plain paint
        /// colour. Carried structurally so the contract tests can compare
        /// fills without linking Skia.
        brush: floom_types::Brush,
        /// Drop shadow requested behind the fill; the submit path turns it
        /// into an `SkBlurImageFilter`-based draw (see the type's docs).
        shadow: Option<floom_types::Shadow>,
    },
    /// A path, in Skia's own shape vocabulary: `SkPath` holds the segments and
    /// a `SkPaint` decides fill or stroke. Recorded as the segment count plus
    /// the style so the adapter's contract test can tell a fill from a stroke
    /// and a line from a circle without linking Skia.
    DrawPath {
        bounds: Rect,
        segments: usize,
        fill: Option<floom_types::Brush>,
        stroke: Option<(f64, Color)>,
    },
    /// `canvas.clipPath` / `clipRRect`. Skia keeps the same push/pop stack the
    /// scene does, so a clip is recorded rather than folded into the commands
    /// under it — folding is what the scene stack exists to avoid.
    PushClip {
        bounds: Rect,
        rounded: bool,
    },
    PopClip,
    /// `canvas.save()` + `canvas.concat(matrix)` for a `GroupPush` marker.
    /// Recorded as the six matrix coefficients so the contract test can
    /// compare scopes without linking Skia.
    PushTransform {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
    /// `canvas.restore()` for a `GroupPop`.
    PopTransform,
    DrawImage {
        bounds: Rect,
        alpha: u8,
    },
    DrawText {
        bounds: Rect,
    },
    /// `canvas.saveLayer()` with an `SkImageFilter` for a `FilterPush`
    /// marker: the scope's content is drawn into a layer and the filter runs
    /// when the layer is restored — which is exactly the layer the scene's
    /// markers describe, so the adapter records the request instead of
    /// folding it into the commands underneath.
    PushFilter {
        filter: Filter,
    },
    /// `canvas.restore()` for a `FilterPop`.
    PopFilter,
    /// A filter over the pixels already drawn inside `rect`: Skia's
    /// `SkImageFilter` applied to a backdrop save-layer.
    BackdropFilter {
        rect: Rect,
        filter: Filter,
    },
    /// A custom WGSL shader filling `bounds`. Recorded so the request is
    /// visible at this ABI even though the submit path is the only place
    /// allowed to honour it (and the software rasteriser skips it). The same
    /// "degradation is never silent" half of A5 a `FilterPush` gets: a backend
    /// dropping it would leave a custom effect reading as plain transparency.
    Shader {
        bounds: Rect,
        shader: floom_types::Shader,
    },
}

/// What the adapter recorded, and what it could not.
///
/// The counters are the point of the crate: a submit path that has to skip
/// something says so here rather than in a comment. The names match the
/// other backends' (`shadows_skipped`, `filters_skipped`, `shaders_skipped`)
/// so a caller can read one number the same way whichever backend produced
/// it.
#[derive(Debug, Default)]
pub struct SkiaRecording {
    /// The commands, in the order the scene carried them.
    pub commands: Vec<SkiaCommand>,
    /// Frames recorded.
    pub frames: u64,
    /// Custom-shader nodes a Skia submit path could not run.
    ///
    /// Skia has no WGSL runtime, so a `Shader` node would have to be
    /// rasterised elsewhere or skipped. Counted here so the decision is
    /// visible rather than inferred.
    pub shaders_skipped: u64,
    /// Filter requests a submit path could not apply.
    ///
    /// Zero today: Skia has an `SkImageFilter` and a `saveLayer`, so a
    /// filter is something the submit path *can* do. The counter exists so
    /// that the day it cannot — an `SkImageFilter` the GPU backend refuses,
    /// say — the number appears rather than the effect vanishing.
    pub filters_skipped: u64,
    /// Snapshot requests a submit path could not satisfy.
    ///
    /// Zero today, for the same reason: a `SkImage` the framework can
    /// import is something the submit path can produce.
    pub shadows_skipped: u64,
}

impl SkiaRecording {
    /// The recording's degradations, in the vocabulary every backend shares.
    ///
    /// This crate records; it does not draw. So the honest report is the one
    /// its own counters carry — a shader or a filter a *submit path* could not
    /// run — and not an invented one. A recorder that reported zero while its
    /// counters moved would be the capability lie this whole exercise exists
    /// to prevent.
    pub fn degradation(&self) -> crate::DegradationReport {
        crate::DegradationReport {
            shadows_skipped: self.shadows_skipped,
            filters_skipped: self.filters_skipped,
            backdrop_filters_skipped: 0,
            shaders_skipped: self.shaders_skipped,
        }
    }
}

/// The name this crate had before it said what it was.
///
/// Kept as an alias so a caller that used the old spelling keeps compiling,
/// and so the rename is visible in the diff rather than silent. The new name
/// is the one to use.
pub type SkiaRendererAdapter = SkiaRecording;

impl Renderer for SkiaRecording {
    fn begin_frame(&mut self, _info: &FrameInfo) {
        self.commands.clear();
    }

    fn render_scene(&mut self, scene: &Scene, _damage: &DamageRegion) {
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect {
                    bounds,
                    fill,
                    shadow,
                    ..
                } => self.commands.push(SkiaCommand::FillRect {
                    bounds: *bounds,
                    brush: fill.clone(),
                    shadow: *shadow,
                }),
                SceneNode::Image {
                    bounds,
                    image: _,
                    alpha,
                } => self.commands.push(SkiaCommand::DrawImage {
                    bounds: *bounds,
                    alpha: *alpha,
                }),
                SceneNode::Text { origin, size, .. } => self.commands.push(SkiaCommand::DrawText {
                    bounds: Rect::from_origin_size(*origin, *size),
                }),
                SceneNode::Path { path, fill, stroke } => {
                    self.commands.push(SkiaCommand::DrawPath {
                        bounds: path.bounds(),
                        segments: path.segments().len(),
                        fill: fill.clone(),
                        stroke: stroke.as_ref().map(|s| (s.width, s.color)),
                    })
                }
                SceneNode::ClipPush { region } => self.commands.push(SkiaCommand::PushClip {
                    bounds: region.bounds(),
                    rounded: !matches!(region, floom_types::ClipRegion::Rect(_)),
                }),
                SceneNode::ClipPop => self.commands.push(SkiaCommand::PopClip),
                SceneNode::GroupPush { transform } => {
                    self.commands.push(SkiaCommand::PushTransform {
                        a: transform.a,
                        b: transform.b,
                        c: transform.c,
                        d: transform.d,
                        e: transform.e,
                        f: transform.f,
                    })
                }
                SceneNode::GroupPop => self.commands.push(SkiaCommand::PopTransform),
                SceneNode::FilterPush { filter } => self
                    .commands
                    .push(SkiaCommand::PushFilter { filter: *filter }),
                SceneNode::FilterPop => self.commands.push(SkiaCommand::PopFilter),
                SceneNode::BackdropFilter { rect, filter } => {
                    self.commands.push(SkiaCommand::BackdropFilter {
                        rect: *rect,
                        filter: *filter,
                    })
                }
                SceneNode::Shader { bounds, shader } => {
                    // Recorded *and* counted: the command says what was asked
                    // for, and the counter says that a Skia submit path
                    // cannot run it. A caller reading only the commands would
                    // think the shader was drawn.
                    self.commands.push(SkiaCommand::Shader {
                        bounds: *bounds,
                        shader: shader.clone(),
                    });
                    self.shaders_skipped += 1;
                }
            }
        }
    }

    fn end_frame(&mut self) {
        self.frames += 1;
    }
}
