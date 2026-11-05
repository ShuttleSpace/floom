// SPDX-License-Identifier: MIT
//! Golden-file comparison for a backend's own output.
//!
//! This is the layer a backend's pixel regression lives at. It knows nothing
//! about scenes, semantics or pictures — only that a backend produced some
//! text and that text should not change without somebody saying so. That
//! narrowness is deliberate, and it is why this module sits here rather than
//! in `floom-snapshot`:
//!
//! `floom-snapshot` is a *tooling*-layer crate, and a backend that wants a
//! golden may only reach upward into tooling through a test — which is not a
//! dependency a manifest can express. `floom-renderer` is the ABI every
//! backend already depends on, so putting the comparison here makes it
//! reachable from every backend's `dev-dependencies` without adding an edge
//! the architecture gate would have to forbid. `floom-snapshot` re-exports
//! these two functions so its own callers keep working unchanged.
//!
//! # Why the golden lives in the asserting crate
//!
//! A golden file belongs to the crate whose test asserts it. `env!` inside
//! *this* crate would expand to *this* crate's manifest, so a path built
//! here would drop every backend's goldens into one directory — where two
//! backends could collide on a name. The caller therefore passes its own
//! manifest directory in, which [`snapshots_dir!`] supplies at the call site.

use std::path::PathBuf;

/// The manifest directory of the crate that *calls* this macro.
///
/// A macro expands at the call site, so it resolves to the caller's own
/// directory — the one place that actually knows where its goldens belong.
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
///
/// The comparison is a plain string equality rather than a hash: a hash
/// makes the diff unreadable, and the whole point of a golden is that a
/// failure says *what changed* to whoever has to decide whether to bless it.
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

/// FNV-1a over `bytes`, as 16 hex digits.
///
/// Hand-rolled rather than `DefaultHasher` because a golden's hash has to be
/// stable across platforms and Rust versions: a std hasher is free to change
/// its algorithm between releases, and every such change would invalidate
/// every golden in the repository.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// A deterministic RGBA + hash fingerprint of a rendered picture.
///
/// The type lives here, in the renderer ABI, so that **any** backend can have
/// a pixel golden through a dependency it already has. The alternative — a
/// snapshot type in a tooling-layer crate — leaves every backend that draws
/// its own pixels without one, which is precisely the gap that let the
/// macOS-gate CJK defect reach a screenshot instead of a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelSnapshot {
    pub width: u32,
    pub height: u32,
    /// FNV-1a of the raw RGBA8 buffer (length = width*height*4), with the
    /// dimensions folded in first.
    pub hash: String,
    /// Raw RGBA8 pixels, kept so a test can assert on individual pixels too.
    pub rgba: Vec<u8>,
}

impl PixelSnapshot {
    /// A snapshot of a buffer a backend produced.
    ///
    /// The backend hands over its buffer, and the same hashing and the same
    /// golden file apply as for any other snapshot.
    ///
    /// The width and height are the **physical** ones, so a buffer that is
    /// the wrong size is a different snapshot rather than a re-hash of a
    /// different picture. The dimensions go into the hash, not just the
    /// buffer: the same bytes in a different shape are a different image, and
    /// a hash that could not tell them apart would let a backend change the
    /// shape of its output without the golden noticing.
    pub fn of_buffer(rgba: &[u8], width: u32, height: u32) -> Self {
        let mut hashed = Vec::with_capacity(rgba.len() + 8);
        hashed.extend_from_slice(&width.to_le_bytes());
        hashed.extend_from_slice(&height.to_le_bytes());
        hashed.extend_from_slice(rgba);
        let hash = format!("{:016x}", fnv1a(&hashed));
        Self {
            width,
            height,
            hash,
            rgba: rgba.to_vec(),
        }
    }

    /// The alpha of one pixel, or `None` outside the buffer.
    ///
    /// A convenience for the probe style the software backend's tests use:
    /// "this pixel is inked" only means something next to "the same picture
    /// without it is clear", and a probe reads better than an index
    /// calculation.
    pub fn alpha(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = ((y as usize) * self.width as usize + x as usize) * 4;
        self.rgba.get(index + 3).copied()
    }

    /// Golden-file compare / update. The golden stores `WxH <hash>` so a resize
    /// shows up as a mismatch, not a silent re-hash of a different buffer.
    pub fn assert(&self, manifest_dir: &str, name: &str) {
        let content = format!("{}x{} {}", self.width, self.height, self.hash);
        assert_snapshot(manifest_dir, name, &content);
    }
}
