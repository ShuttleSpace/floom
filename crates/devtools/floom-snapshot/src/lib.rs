// SPDX-License-Identifier: MIT
//! Deterministic snapshots for Floom 2.0 visual-regression and a11y tests.
//!
//! Three snapshot kinds, each a stable fingerprint of a different layer:
//!
//! * [`SceneSnapshot`] — the retained scene graph (`floom-scene`), so a widget
//!   that paints differently shows up even when no pixel is compared.
//! * [`SemanticsSnapshot`] — the accessibility tree (`floom-accessibility`), so
//!   a role/label/state change is caught independent of rendering.
//! * [`PixelSnapshot`] — the software renderer's RGBA buffer (`floom-renderer-
//!   software`), the real visual-regression layer.
//!
//! Every snapshot serialises to plain text and carries a deterministic hash
//! (FNV-1a, hand-rolled — stable across platforms and Rust versions, unlike
//! `DefaultHasher`). [`assert_snapshot`] compares against a golden file under
//! `snapshots/`; set `UPDATE_SNAPSHOTS=1` to (re)write one.
//!
//! Determinism note: a scene built the same way allocates the same slot ids, so
//! `SceneNodeId`s in `Group` children are stable run to run. For semantics, the
//! tree is sorted by `(id.index(), id.generation())` because `ElementId` has no
//! `Ord` by design.

use floom_accessibility::{SemanticsNode, SemanticsTree};
use floom_renderer::{AttachedSurface, Presenter};
use floom_renderer_software::SoftwarePresenter;
use floom_scene::{ResourceRegistry, Scene, SceneNode};
use floom_types::{Color, Point, Rect, Size, SurfaceId, WindowId};

pub mod frames;
pub use frames::{FrameDiff, FrameDigest, GoldenFrames, parse_sequence};

// The golden-file machinery and the pixel snapshot type itself live in
// `floom_renderer`: a backend can reach a golden through a dependency it
// already has, instead of reaching up into this tooling-layer crate. These
// re-exports keep every existing caller unchanged, and the impls below add the
// reference half — which needs the software presenter, and so cannot live
// there.
pub use floom_renderer::{PixelSnapshot, assert_snapshot, fnv1a, snapshot_path, snapshots_dir};
pub use floom_types::{ElementId, SceneNodeId};

fn hash_str(s: &str) -> String {
    format!("{:016x}", fnv1a(s.as_bytes()))
}

/// A deterministic text + hash fingerprint of a [`Scene`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSnapshot {
    /// One line per live node, in slot order, formatted deterministically.
    pub text: String,
    /// FNV-1a of `text`.
    pub hash: String,
}

impl SceneSnapshot {
    /// Build a snapshot from a scene. The `Debug` representation of each node is
    /// used (floom types are float-stable); slot order is preserved by
    /// [`Scene::iter`].
    pub fn of(scene: &Scene) -> Self {
        let mut lines = Vec::new();
        for (_id, node) in scene.iter() {
            lines.push(format!("  {node:?}"));
        }
        let text = lines.join("\n");
        let hash = hash_str(&text);
        Self { text, hash }
    }

    /// Golden-file compare / update. Returns the hash so a test can also assert
    /// on it directly.
    pub fn assert(&self, manifest_dir: &str, name: &str) {
        assert_snapshot(manifest_dir, name, &self.text);
    }
}

/// A deterministic text + hash fingerprint of a [`SemanticsTree`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsSnapshot {
    /// One line per node, sorted by `(index, generation)`, formatted
    /// deterministically.
    pub text: String,
    /// FNV-1a of `text`.
    pub hash: String,
}

impl SemanticsSnapshot {
    /// Build a snapshot from a semantics tree. Node pairs are sorted by
    /// `(id.index(), id.generation())` for a stable order.
    pub fn of(tree: &SemanticsTree) -> Self {
        let mut pairs: Vec<_> = tree.iter().collect();
        pairs.sort_by_key(|(id, _)| (id.index(), id.generation()));
        let mut lines = Vec::new();
        for (id, node) in pairs {
            lines.push(format!(
                "  {}@{}/{}: {:?}",
                id.index(),
                id.generation(),
                id.index(),
                node
            ));
        }
        let text = lines.join("\n");
        let hash = hash_str(&text);
        Self { text, hash }
    }

    /// Golden-file compare / update.
    pub fn assert(&self, manifest_dir: &str, name: &str) {
        assert_snapshot(manifest_dir, name, &self.text);
    }
}

/// The **reference** pixel snapshot: `scene` rendered through the software
/// presenter, which is the picture every other backend is measured against.
///
/// A trait rather than an inherent constructor because it needs the software
/// presenter, and Rust does not allow an inherent `impl` for a type defined in
/// another crate. That constraint is the honest one: the *type* is
/// backend-neutral and lives in `floom_renderer` so any backend can snapshot
/// its own buffer, but *this* half is specifically the reference, and only a
/// crate that already depends on the software renderer can produce it.
///
/// The hashing is `PixelSnapshot::of_buffer`'s, so a golden written through
/// one constructor is comparable with one written through the other.
pub trait ReferencePixels {
    /// Render `scene` into an offscreen RGBA buffer via the software
    /// presenter. `scale` controls the physical buffer size; `1.0` keeps
    /// logical == physical.
    fn of(scene: &Scene, resources: Option<&ResourceRegistry>, size: Size, scale: f64) -> Self;
}

impl ReferencePixels for PixelSnapshot {
    fn of(scene: &Scene, resources: Option<&ResourceRegistry>, size: Size, scale: f64) -> Self {
        let mut presenter = SoftwarePresenter::new();
        let surface =
            AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size, scale, None);
        assert!(presenter.attach(surface), "software presenter must attach");
        let empty = ResourceRegistry::new();
        let res = resources.unwrap_or(&empty);
        presenter.present(scene, res);
        let width = presenter.width();
        let height = presenter.height();
        let rgba = presenter.pixels().to_vec();
        PixelSnapshot::of_buffer(&rgba, width, height)
    }
}

/// Read one RGBA pixel, or `None` if out of bounds.
///
/// A trait method for the same reason as [`ReferencePixels::of`]: the type is
/// defined in `floom_renderer`, so it cannot grow an inherent impl here.
pub trait PixelProbe {
    fn rgba_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]>;
}

impl PixelProbe for PixelSnapshot {
    fn rgba_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y * self.width + x) as usize) * 4;
        Some([
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ])
    }
}

/// Convenience: a filled axis-aligned rect node (radius 0 → crisp edges).
pub fn rect(bounds: Rect, fill: impl Into<floom_types::Brush>) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill: fill.into(),
        radius: 0.0,
        shadow: None,
    }
}

/// Convenience: opens a transform scope around the scene nodes that follow.
pub fn group_push(transform: floom_types::Transform) -> SceneNode {
    SceneNode::GroupPush { transform }
}

/// Convenience: closes the innermost transform scope.
pub fn group_pop() -> SceneNode {
    SceneNode::GroupPop
}

/// Build a one-rect scene at the given extent — the simplest pixel fixture.
pub fn single_rect_scene(size: Size, fill: Color) -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(rect(
        Rect::from_origin_size(Point::new(0.0, 0.0), size),
        fill,
    ));
    scene.finish_update();
    scene
}

/// A semantics node with the most common fields set, for snapshot fixtures.
pub fn semantics_node(role: floom_accessibility::SemanticsRole, label: &str) -> SemanticsNode {
    SemanticsNode::new(role).with_label(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_accessibility::{SemanticsRole, SemanticsTree};

    #[test]
    fn scene_snapshot_is_stable_and_sensitive() {
        let a = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(10, 20, 30));
        let b = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(10, 20, 30));
        let c = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(99, 99, 99));
        assert_eq!(SceneSnapshot::of(&a).text, SceneSnapshot::of(&b).text);
        assert_eq!(SceneSnapshot::of(&a).hash, SceneSnapshot::of(&b).hash);
        assert_ne!(SceneSnapshot::of(&a).hash, SceneSnapshot::of(&c).hash);
    }

    #[test]
    fn semantics_snapshot_is_sorted_and_stable() {
        let mut tree = SemanticsTree::new();
        // Insert out of index order; the snapshot must still come out sorted.
        tree.set_node(
            ElementId::new(3, 1),
            semantics_node(SemanticsRole::Button, "second"),
        );
        tree.set_node(
            ElementId::new(1, 1),
            semantics_node(SemanticsRole::StaticText, "first"),
        );
        let snap = SemanticsSnapshot::of(&tree);
        let first_line = snap.text.lines().next().unwrap();
        assert!(
            first_line.contains("1@1/1"),
            "must be sorted by index: {first_line}"
        );
        assert!(snap.text.contains("StaticText"));
        assert!(snap.text.contains("Button"));

        // A second identical tree produces an identical snapshot.
        let mut tree2 = SemanticsTree::new();
        tree2.set_node(
            ElementId::new(1, 1),
            semantics_node(SemanticsRole::StaticText, "first"),
        );
        tree2.set_node(
            ElementId::new(3, 1),
            semantics_node(SemanticsRole::Button, "second"),
        );
        assert_eq!(snap.hash, SemanticsSnapshot::of(&tree2).hash);
    }

    #[test]
    fn pixel_snapshot_renders_and_is_deterministic() {
        let scene = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(200, 100, 50));
        let p1 = PixelSnapshot::of(&scene, None, Size::new(8.0, 8.0), 1.0);
        let p2 = PixelSnapshot::of(&scene, None, Size::new(8.0, 8.0), 1.0);
        assert_eq!(p1.hash, p2.hash, "pixel hash must be reproducible");
        assert_eq!((p1.width, p1.height), (8, 8));
        // A pixel well inside the rect carries the fill, fully opaque.
        assert_eq!(p1.rgba_pixel(4, 4), Some([200, 100, 50, 255]));
        // A pixel outside the buffer is None.
        assert_eq!(p1.rgba_pixel(100, 100), None);

        // A different fill -> different hash.
        let other = single_rect_scene(Size::new(8.0, 8.0), Color::rgb(0, 0, 0));
        assert_ne!(
            p1.hash,
            PixelSnapshot::of(&other, None, Size::new(8.0, 8.0), 1.0).hash
        );
    }
}
