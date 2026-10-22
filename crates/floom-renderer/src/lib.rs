//! Backend-neutral renderer ABI.
//!
//! Concrete GPU backends live in separate crates and must not pull widgets.

mod external;
mod frame_queue;
mod present;

pub use external::{ExternalFrame, ExternalTextureCache, ExternalTextureStats, ImportDecision};
pub use floom_scene::{DamageRegion, ResourceRegistry, Scene, SceneNode};
pub use floom_types::{Color, Point, Rect, ResourceId, SceneNodeId, Size};
pub use frame_queue::{FrameHandoffStats, SceneFrameQueue, SceneSnapshot};
pub use present::{AttachedSurface, Presenter, SurfaceHandles};

use floom_scene::Scene as SceneGraph;
use floom_types::{SurfaceId, WindowId};

/// Where a frame is presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderTarget {
    /// Window-backed surface.
    Surface {
        window: WindowId,
        surface: SurfaceId,
    },
    /// Offscreen texture (logical id only).
    Texture { id: ResourceId },
    /// External compositor target.
    External { id: ResourceId },
    /// CPU buffer target.
    Software { width: u32, height: u32 },
}

/// Per-frame timing / size metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameInfo {
    pub target: RenderTarget,
    pub logical_size: Size,
    pub scale_factor: f64,
    pub frame_number: u64,
}

impl FrameInfo {
    pub fn software(width: u32, height: u32, frame_number: u64) -> Self {
        Self {
            target: RenderTarget::Software { width, height },
            logical_size: Size::new(width as f64, height as f64),
            scale_factor: 1.0,
            frame_number,
        }
    }
}

/// What a renderer backend can and cannot do.
///
/// A host asks this **before** it draws, so it can decide whether to ask for
/// an effect at all. The alternative is drawing it and finding out afterwards
/// that the backend dropped it — and a dropped shadow is indistinguishable
/// from a shadow that was never asked for.
///
/// The four flags are the requests that differ between backends today. A
/// request that *every* backend honours is not a flag: it is a fact of the
/// scene vocabulary, and a flag for it would be noise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RendererCapabilities {
    /// The backend draws a drop shadow behind a shape.
    ///
    /// `spread` is honoured where it is drawn: the silhouette is dilated
    /// *before* the blur, as CSS does, rather than the blur being widened.
    pub shadows: bool,
    /// The backend applies a filter to a scope's contents.
    pub filters: bool,
    /// The backend applies a filter to the pixels already drawn.
    pub backdrop_filters: bool,
    /// The backend runs a custom shader program.
    pub shaders: bool,
}

impl RendererCapabilities {
    /// Everything a backend can do.
    pub const ALL: Self = Self {
        shadows: true,
        filters: true,
        backdrop_filters: true,
        shaders: true,
    };

    /// Nothing: a backend that draws shapes and text only.
    pub const NONE: Self = Self {
        shadows: false,
        filters: false,
        backdrop_filters: false,
        shaders: false,
    };
}

/// Backend-neutral renderer contract.
pub trait Renderer {
    /// Begin a frame against `target`.
    fn begin_frame(&mut self, info: &FrameInfo);

    /// Consume a retained scene (and optional damage) into the current frame.
    fn render_scene(&mut self, scene: &SceneGraph, damage: &DamageRegion);

    /// Like [`Renderer::render_scene`], with access to the scene's resource
    /// registry (glyph runs, image pixels). Backends that can render those
    /// nodes override this; the default ignores the registry, so text and
    /// image nodes are simply skipped rather than fatal.
    fn render_scene_with_resources(
        &mut self,
        scene: &SceneGraph,
        damage: &DamageRegion,
        resources: &ResourceRegistry,
    ) {
        let _ = resources;
        self.render_scene(scene, damage);
    }

    /// Finish and present/submit the frame.
    fn end_frame(&mut self);

    /// Optional: clear caches after resource eviction.
    fn trim_caches(&mut self) {}

    /// What this backend can do.
    ///
    /// The default is the pessimistic answer: a backend that has not said
    /// otherwise is assumed to be able to do nothing beyond shapes and text.
    /// A backend that renders a shadow has to say so, and a backend that
    /// cannot render one has to be believed.
    ///
    /// This is the same rule the platform capabilities follow, and the same
    /// reason: a capability that is claimed and not honoured is worse than one
    /// that is missing.
    fn capabilities(&self) -> RendererCapabilities {
        RendererCapabilities::NONE
    }
}

/// Render one scene through any [`Renderer`] backend.
pub fn render_once(renderer: &mut dyn Renderer, info: &FrameInfo, scene: &SceneGraph) {
    renderer.begin_frame(info);
    renderer.render_scene(scene, scene.damage());
    renderer.end_frame();
}
