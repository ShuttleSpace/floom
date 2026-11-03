//! Textures whose contents the framework does not produce (Phase 16, task 16.3).
//!
//! A `Texture`-mode participant — a video decoder, a map engine, a camera, a
//! web view — produces pixels on its own schedule, and the framework draws them
//! as an ordinary scene image so the participant composites *inside* the scene:
//! it can be occluded, clipped and transformed like anything else, which is the
//! one thing Overlay mode cannot do (device case D10).
//!
//! Deciding *when* to bring those pixels across is the whole of this module,
//! and it is a policy rather than a mechanism because the mechanism differs per
//! backend while the policy does not:
//!
//! - The producer's [`ExternalFrame::sequence`] is the only statement about
//!   content freshness. Nothing else can infer it — a resolution change, a
//!   re-encode, a frame that happens to be identical all look the same from
//!   here.
//! - The resource id is the *identity*, and it is stable across refreshes. A
//!   new entry per frame is the bug this exists to prevent: it would grow the
//!   cache without bound and, on any backend keying its GPU objects by id,
//!   re-create one per frame.
//! - "Re-import", not "re-upload", is the goal. A backend that can wrap a GPU
//!   object the producer owns does no CPU work at all, which is what
//!   [`ExternalFrame::pixels`] being optional says: a frame with no bytes is not
//!   an incomplete frame, it is the cheap case.
//!
//! Nothing here touches a GPU, so the policy is testable, and the counters are
//! what task 16.3 step 4 (bytes per frame, added frame time) is measured with.

use std::collections::{HashMap, HashSet};

use floom_types::ResourceId;

/// One frame of a texture the framework does not own.
#[derive(Debug, Clone, PartialEq)]
pub struct ExternalFrame {
    /// The scene resource the participant's image node references. Stable for
    /// the life of the participant: it is the cache key and, on a backend that
    /// imports GPU objects, the handle.
    pub resource: ResourceId,
    /// Bumped by the producer whenever the contents changed.
    ///
    /// Only *equality* means "same contents". A counter that restarts, wraps or
    /// moves backwards is still a different frame, because the framework has no
    /// way to know which convention the producer follows.
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    /// The pixels, in the format the renderer uploads verbatim, or `None` when
    /// the producer keeps them on the GPU and the backend can wrap the object
    /// directly.
    pub pixels: Option<std::sync::Arc<[u8]>>,
}

impl ExternalFrame {
    /// Bytes a backend that has to copy would move for this frame.
    pub fn byte_len(&self) -> u64 {
        match &self.pixels {
            Some(pixels) => pixels.len() as u64,
            None => 0,
        }
    }
}

/// What a backend has to do with one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportDecision {
    /// Never seen this resource: build whatever the backend draws from.
    First,
    /// The producer says the contents changed: refresh in place, keeping the
    /// identity so everything keyed on it stays valid.
    Refresh,
    /// Same sequence as last time: keep what is already resident.
    Unchanged,
}

impl ImportDecision {
    /// Whether the backend must do work. `false` is the common case for content
    /// slower than the display refresh — a 30 fps video in a 60 fps app, a map
    /// that is not moving, a paused stream.
    pub fn is_upload(&self) -> bool {
        !matches!(self, Self::Unchanged)
    }
}

/// Per-frame and cumulative counters.
///
/// Cumulative on purpose: "how many times did we re-upload this texture" is a
/// question about a session, not about a frame, and the per-frame numbers can
/// be recovered by reading the difference across two frames.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExternalTextureStats {
    /// Resources seen for the first time.
    pub first_imports: u64,
    /// Contents the producer said had changed.
    pub refreshes: u64,
    /// Frames that needed no work at all.
    pub reuses: u64,
    /// Bytes handed across for a backend to copy. Zero for a backend that wraps
    /// a GPU object: that is the point of the cheap path, not a missing count.
    pub bytes_uploaded: u64,
    /// Resources evicted because nothing referenced them in a frame.
    pub evictions: u64,
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    sequence: u64,
    width: u32,
    height: u32,
    /// Whether the pixels for this entry were on the CPU side, so a `Refresh`
    /// can report the bytes it will move.
    copied: bool,
    /// Produced a frame since the last prune. The *producer's* liveness signal,
    /// which is a different question from whether the scene still draws it: a
    /// participant can outlive the frame that first drew it (its node is
    /// created on the next pass) and a scene node can outlive the last frame its
    /// producer sent (a paused stream that is still on screen). Keeping an entry
    /// alive for either reason is right; dropping it for either is a re-import.
    touched: bool,
}

/// Decides when a texture must be re-imported, and keeps the identity stable
/// while it happens.
///
/// One instance per presenter. The caller drives it in two steps per frame: an
/// [`ExternalTextureCache::import`] for every participant that produced a frame,
/// then an [`ExternalTextureCache::prune`] with what the scene referenced. A
/// resource that produced a frame but is not in the scene is dropped, which is
/// how a participant that scrolled away stops costing anything.
#[derive(Debug, Default)]
pub struct ExternalTextureCache {
    entries: HashMap<u64, Entry>,
    stats: ExternalTextureStats,
}

impl ExternalTextureCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a frame and says what the backend must do with it.
    ///
    /// Idempotent within a frame: a producer that hands the same frame to two
    /// consumers imports once.
    pub fn import(&mut self, frame: &ExternalFrame) -> ImportDecision {
        let key = frame.resource.raw();
        let copied = frame.pixels.is_some();
        let bytes = frame.byte_len();
        match self.entries.get_mut(&key) {
            Some(entry) if entry.sequence == frame.sequence => {
                entry.touched = true;
                self.stats.reuses += 1;
                ImportDecision::Unchanged
            }
            Some(entry) => {
                entry.sequence = frame.sequence;
                entry.width = frame.width;
                entry.height = frame.height;
                entry.copied = copied;
                entry.touched = true;
                self.stats.refreshes += 1;
                self.stats.bytes_uploaded += bytes;
                ImportDecision::Refresh
            }
            None => {
                self.entries.insert(
                    key,
                    Entry {
                        sequence: frame.sequence,
                        width: frame.width,
                        height: frame.height,
                        copied,
                        touched: true,
                    },
                );
                self.stats.first_imports += 1;
                self.stats.bytes_uploaded += bytes;
                ImportDecision::First
            }
        }
    }

    /// Whether this resource's contents are already resident.
    ///
    /// A backend needs this while encoding: an image node whose resource is an
    /// external texture has no pixels in the registry, and asking the registry
    /// instead is how the node ends up counted as unsupported and not drawn.
    pub fn contains(&self, resource: ResourceId) -> bool {
        self.entries.contains_key(&resource.raw())
    }

    /// The last sequence imported for `resource`.
    pub fn sequence(&self, resource: ResourceId) -> Option<u64> {
        self.entries
            .get(&resource.raw())
            .map(|entry| entry.sequence)
    }

    /// The declared size of an imported resource.
    pub fn size(&self, resource: ResourceId) -> Option<(u32, u32)> {
        self.entries
            .get(&resource.raw())
            .map(|entry| (entry.width, entry.height))
    }

    /// Drops entries that neither produced a frame since the last prune nor are
    /// referenced by the current frame.
    ///
    /// Either reason is enough to keep one. A participant that is still
    /// producing is alive even if this frame's scene has not drawn it yet, and
    /// a paused stream is still on screen after its last frame. What is dropped
    /// is a participant that has genuinely gone — which is how a texture stops
    /// costing anything the moment its producer does.
    pub fn prune(&mut self, referenced: &HashSet<u64>) {
        let before = self.entries.len();
        self.entries
            .retain(|key, entry| entry.touched || referenced.contains(key));
        for entry in self.entries.values_mut() {
            entry.touched = false;
        }
        self.stats.evictions += (before - self.entries.len()) as u64;
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// The resource keys still resident. A backend that keeps a parallel table
    /// keyed the same way prunes from this rather than re-deriving it.
    pub fn live_keys(&self) -> impl Iterator<Item = u64> + '_ {
        self.entries.keys().copied()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn stats(&self) -> ExternalTextureStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn frame(resource: ResourceId, sequence: u64) -> ExternalFrame {
        ExternalFrame {
            resource,
            sequence,
            width: 4,
            height: 2,
            pixels: Some(Arc::from(vec![0u8; 32].into_boxed_slice())),
        }
    }

    /// A resource id with nothing behind it: the cache never consults the
    /// registry, which is the point — the pixels are someone else's.
    fn resource(index: u32) -> ResourceId {
        ResourceId::new(index, 1)
    }

    #[test]
    fn an_unseen_texture_is_imported_once_and_then_reused() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(1);

        assert_eq!(cache.import(&frame(id, 1)), ImportDecision::First);
        assert_eq!(
            cache.import(&frame(id, 1)),
            ImportDecision::Unchanged,
            "the producer did not say anything changed, so nothing changed"
        );
        assert_eq!(cache.stats().first_imports, 1);
        assert_eq!(cache.stats().refreshes, 0);
        assert_eq!(cache.stats().reuses, 1);
        assert_eq!(cache.len(), 1, "one identity, one entry");
    }

    /// The bug this whole module exists to prevent: a live texture whose
    /// contents change every frame must not become a new cache entry every
    /// frame. On a backend keying GPU objects by resource id that is one
    /// texture created per frame, and on any backend it is a cache that grows
    /// with the frame count.
    #[test]
    fn a_bumped_sequence_refreshes_in_place_instead_of_minting_an_entry() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(7);

        for sequence in 1..=60 {
            let decision = cache.import(&frame(id, sequence));
            assert_eq!(
                decision,
                if sequence == 1 {
                    ImportDecision::First
                } else {
                    ImportDecision::Refresh
                },
                "frame {sequence}"
            );
        }

        assert_eq!(cache.len(), 1, "sixty frames of video, one entry");
        assert_eq!(cache.stats().first_imports, 1);
        assert_eq!(cache.stats().refreshes, 59);
        assert_eq!(cache.sequence(id), Some(60));
    }

    /// Only equality means "unchanged". A producer that restarts its counter —
    /// a decoder that seeked, a device that reconnected — is reporting different
    /// contents with a smaller number, and treating that as stale would freeze
    /// the picture.
    #[test]
    fn a_sequence_that_moves_backwards_is_still_a_new_frame() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(2);
        cache.import(&frame(id, 500));

        assert_eq!(
            cache.import(&frame(id, 0)),
            ImportDecision::Refresh,
            "the framework cannot know which convention a producer follows, so \
             only equality is trusted"
        );
    }

    #[test]
    fn a_texture_whose_producer_went_quiet_is_dropped_and_imported_afresh() {
        let mut cache = ExternalTextureCache::new();
        let kept = resource(1);
        let dropped = resource(2);
        cache.import(&frame(kept, 1));
        cache.import(&frame(dropped, 1));

        // Both producers are alive this frame: nothing is evicted, even though
        // no scene referenced either of them (a participant's node can be
        // created a pass after its first frame arrives).
        cache.prune(&HashSet::new());
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().evictions, 0);

        // Next frame only one of them produces.
        cache.import(&frame(kept, 2));
        cache.prune(&HashSet::new());

        assert_eq!(cache.len(), 1);
        assert_eq!(cache.stats().evictions, 1);
        assert!(cache.contains(kept));
        assert!(!cache.contains(dropped));

        assert_eq!(
            cache.import(&frame(dropped, 1)),
            ImportDecision::First,
            "evicted means gone, not stale"
        );
    }

    /// The other reason to keep an entry: the scene is still drawing it. A
    /// stream that paused is still on screen, and re-importing it because its
    /// producer went quiet would be a visible flicker for no reason.
    #[test]
    fn a_texture_the_scene_still_draws_survives_its_producer_going_quiet() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(9);
        cache.import(&frame(id, 1));
        cache.prune(&HashSet::from([id.raw()]));

        // Ten frames with no new content, but the node is still in the scene.
        for _ in 0..10 {
            cache.prune(&HashSet::from([id.raw()]));
        }

        assert_eq!(cache.len(), 1, "still drawn, so still resident");
        assert_eq!(cache.stats().evictions, 0);
        assert_eq!(
            cache.import(&frame(id, 1)),
            ImportDecision::Unchanged,
            "and resuming the stream does not re-import the frame it paused on"
        );
    }

    #[test]
    fn bytes_are_counted_only_when_something_was_actually_copied() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(3);

        cache.import(&frame(id, 1));
        assert_eq!(cache.stats().bytes_uploaded, 32);

        for _ in 0..5 {
            cache.import(&frame(id, 1));
        }
        assert_eq!(
            cache.stats().bytes_uploaded,
            32,
            "five unchanged frames moved nothing"
        );

        cache.import(&frame(id, 2));
        assert_eq!(cache.stats().bytes_uploaded, 64);
    }

    /// Task 16.3 step 4, bytes half: the *per-frame* cost of Texture mode, the
    /// number that decides whether it is a fallback or a default.
    ///
    /// Bytes only ever accumulate on a producer that actually reports a new
    /// sequence. A 1080p participant costs one full frame on first import, then
    /// one full frame per refresh, then nothing while it is quiet — and a
    /// participant that never says "changed" costs nothing no matter how many
    /// frames pass. That per-frame delta is the modelling result step 4 exists
    /// to pin down.
    #[test]
    fn texture_mode_moves_an_exact_set_of_bytes_per_produced_frame() {
        let mut cache = ExternalTextureCache::new();

        // A "video-like" participant: reported content every frame.
        let video = resource(10);
        let mut vf = frame(video, 1);
        vf.width = 1920;
        vf.height = 1080;
        vf.pixels = Some(Arc::from(vec![0u8; 1920 * 1080 * 4].into_boxed_slice()));

        // A static "map" participant that changes lanes then goes quiet.
        let map = resource(11);
        cache.import(&frame(map, 1));

        // The video's first frame is a full upload.
        assert_eq!(
            cache.import(&vf),
            ImportDecision::First,
            "the participant's first produced frame is a first import"
        );
        assert_eq!(
            cache.stats().bytes_uploaded,
            1920 * 1080 * 4 + 32,
            "first frame of both: one full frame plus the map's 32 B frame"
        );

        // One second of 60 f/s video, still frames in between: every new content
        // costs one full frame; silence costs nothing.
        let mut produced = 1920 * 1080 * 4 + 32;
        for seq in 2..=60u64 {
            vf.sequence = seq;
            cache.import(&vf);
            produced += 1920 * 1080 * 4;
        }
        // The map moved once more, then went quiet.
        cache.import(&frame(map, 2));
        produced += 32;
        for _ in 0..60 {
            cache.import(&frame(map, 2));
        }

        assert_eq!(
            cache.stats().bytes_uploaded,
            produced,
            "exactly one full frame per produced frame, nothing for silence"
        );
        assert_eq!(
            cache.stats().refreshes,
            59 + 1,
            "every changed frame refreshed"
        );
        assert_eq!(cache.stats().reuses, 60, "the quiet map frames reused");
    }

    /// A backend that wraps a GPU object the producer owns moves no bytes, and
    /// that is the fast path rather than a frame missing its contents.
    #[test]
    fn a_frame_without_pixels_is_the_cheap_path_not_an_empty_one() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(4);
        let mut frame = frame(id, 1);
        frame.pixels = None;

        assert_eq!(cache.import(&frame), ImportDecision::First);
        assert!(cache.contains(id));
        assert_eq!(
            cache.stats().bytes_uploaded,
            0,
            "nothing crossed the bus, so nothing is counted"
        );
        assert_eq!(cache.stats().first_imports, 1);
    }

    #[test]
    fn a_frame_with_no_pixels_still_has_an_identity_and_a_size() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(5);
        let mut frame = frame(id, 9);
        frame.pixels = None;
        frame.width = 1920;
        frame.height = 1080;

        cache.import(&frame);

        assert_eq!(cache.size(id), Some((1920, 1080)));
    }

    #[test]
    fn imports_are_idempotent_so_two_consumers_do_not_double_import() {
        let mut cache = ExternalTextureCache::new();
        let id = resource(6);
        let f = frame(id, 1);

        assert!(cache.import(&f).is_upload());
        assert!(
            !cache.import(&f).is_upload(),
            "the second consumer of the same frame finds it resident"
        );
        assert_eq!(cache.stats().first_imports, 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn two_textures_are_tracked_independently() {
        let mut cache = ExternalTextureCache::new();
        let a = resource(1);
        let b = resource(2);

        cache.import(&frame(a, 1));
        cache.import(&frame(b, 1));
        cache.import(&frame(a, 2));

        assert_eq!(cache.sequence(a), Some(2));
        assert_eq!(cache.sequence(b), Some(1), "b was not touched");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.stats().refreshes, 1);
    }
}
