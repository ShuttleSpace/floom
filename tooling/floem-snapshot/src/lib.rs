//! Deterministic snapshots for Floem 2.0 visual-regression and a11y tests.
//!
//! Three snapshot kinds, each a stable fingerprint of a different layer:
//!
//! * [`SceneSnapshot`] — the retained scene graph (`floem-scene`), so a widget
//!   that paints differently shows up even when no pixel is compared.
//! * [`SemanticsSnapshot`] — the accessibility tree (`floem-accessibility`), so
//!   a role/label/state change is caught independent of rendering.
//! * [`PixelSnapshot`] — the software renderer's RGBA buffer (`floem-renderer-
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

use std::path::PathBuf;

use floem_accessibility::{SemanticsNode, SemanticsTree};
use floem_renderer::{AttachedSurface, Presenter};
use floem_renderer_software::SoftwarePresenter;
use floem_scene::{ResourceRegistry, Scene};
use floem_types::{Color, Point, Rect, Size, SurfaceId, WindowId};

/// Stable, dependency-free 64-bit hash (FNV-1a, 64-bit offset basis).
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

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
    /// used (floem types are float-stable); slot order is preserved by
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

/// A deterministic RGBA + hash fingerprint of a rendered [`Scene`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelSnapshot {
    pub width: u32,
    pub height: u32,
    /// FNV-1a of the raw RGBA8 buffer (length = width*height*4).
    pub hash: String,
    /// Raw RGBA8 pixels, kept so a test can assert on individual pixels too.
    pub rgba: Vec<u8>,
}

impl PixelSnapshot {
    /// Render `scene` (with optional `resources`) into an offscreen RGBA buffer
    /// via the software presenter and hash the result. `scale` controls the
    /// physical buffer size; `1.0` keeps logical == physical.
    pub fn of(scene: &Scene, resources: Option<&ResourceRegistry>, size: Size, scale: f64) -> Self {
        let mut presenter = SoftwarePresenter::new();
        let surface =
            AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size, scale, None);
        assert!(presenter.attach(surface), "software presenter must attach");
        let empty = ResourceRegistry::new();
        let res = resources.unwrap_or(&empty);
        presenter.present(scene, res);
        let rgba = presenter.pixels().to_vec();
        let hash = format!("{:016x}", fnv1a(&rgba));
        Self {
            width: presenter.width(),
            height: presenter.height(),
            hash,
            rgba,
        }
    }

    /// Golden-file compare / update. The golden stores `WxH <hash>` so a resize
    /// shows up as a mismatch, not a silent re-hash of a different buffer.
    pub fn assert(&self, manifest_dir: &str, name: &str) {
        let content = format!("{}x{} {}", self.width, self.height, self.hash);
        assert_snapshot(manifest_dir, name, &content);
    }
}

/// The manifest directory of the crate that *calls* this macro.
///
/// A golden file belongs to the crate whose test asserts it, and the caller is
/// the only one that knows where that is: `env!` inside this crate expands to
/// *this* crate's manifest, so a `snapshot_path` that used it would drop every
/// downstream crate's goldens into `tooling/floem-snapshot/snapshots/` — which
/// is exactly what happened before this macro existed, and why two crates could
/// collide on a snapshot name. A macro expands at the call site, so it resolves
/// to the right directory.
#[macro_export]
macro_rules! snapshots_dir {
    () => {
        env!("CARGO_MANIFEST_DIR")
    };
}

/// Compare `content` against `<manifest_dir>/snapshots/<name>.snap`.
///
/// With `UPDATE_SNAPSHOTS=1` the file is (re)written and the test passes.
/// Otherwise a missing or differing file fails the test with a short diff.
pub fn assert_snapshot(manifest_dir: &str, name: &str, content: &str) {
    let path = snapshot_path(manifest_dir, name);
    if std::env::var("UPDATE_SNAPSHOTS")
        .map(|v| v == "1")
        .unwrap_or(false)
    {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create snapshots dir");
        }
        std::fs::write(&path, content).expect("write snapshot");
        return;
    }
    match std::fs::read_to_string(&path) {
        Ok(prev) if prev == content => {}
        Ok(prev) => panic!(
            "snapshot `{name}` changed ({}):\n--- previous ---\n{prev}\n--- current ---\n{content}",
            path.display()
        ),
        Err(_) => panic!(
            "snapshot `{name}` missing at {}; run with UPDATE_SNAPSHOTS=1 to create it",
            path.display()
        ),
    }
}

/// Path of the golden file for `name`, inside `manifest_dir`.
///
/// `manifest_dir` is the asserting crate's own manifest directory — normally
/// [`snapshots_dir!`], which resolves it at the call site.
pub fn snapshot_path(manifest_dir: &str, name: &str) -> PathBuf {
    PathBuf::from(manifest_dir)
        .join("snapshots")
        .join(format!("{name}.snap"))
}

// Re-export the few builders a snapshot test wants, so callers depend on this
// crate alone rather than reaching into the scene/semantics crates.
pub use floem_scene::SceneNode;
pub use floem_types::{ElementId, SceneNodeId};

/// Convenience: a filled axis-aligned rect node (radius 0 → crisp edges).
pub fn rect(bounds: Rect, fill: Color) -> SceneNode {
    SceneNode::Rect {
        bounds,
        fill,
        radius: 0.0,
    }
}

/// Convenience: a group node with the given children.
pub fn group(children: &[SceneNodeId], transform: floem_types::Transform) -> SceneNode {
    SceneNode::Group {
        children: children.iter().copied().collect(),
        transform,
    }
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
pub fn semantics_node(role: floem_accessibility::SemanticsRole, label: &str) -> SemanticsNode {
    SemanticsNode::new(role).with_label(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_accessibility::{SemanticsRole, SemanticsTree};

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

impl PixelSnapshot {
    /// Read one RGBA pixel, or `None` if out of bounds.
    pub fn rgba_pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
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
