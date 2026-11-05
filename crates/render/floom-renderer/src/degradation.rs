// SPDX-License-Identifier: MIT
//! One vocabulary for "what did this backend drop".
//!
//! Every backend counts its own degradations, and for a long time each spelled
//! them its own way: `shadows_drawn` here, `shadow_skips` there, a single
//! `skipped` on the terminal that folded six different requests into one
//! number. A host that wanted to answer "how much of my scene did the
//! renderer drop" had to know which backend it was talking to — which is
//! exactly the question a host should not have to ask.
//!
//! [`DegradationReport`] is that answer, and it is a *view* rather than a
//! replacement. Each backend keeps its own stats struct, with its own fields,
//! its own tests and its own callers; what this adds is a conversion into one
//! shape, so a host can compare a vello frame against a software one without
//! a `match` on the backend.
//!
//! # What it deliberately does not do
//!
//! It does not merge the counters into one number. "Six things were dropped"
//! and "six shadows were dropped" are different facts, and a caller deciding
//! whether to ask for an effect at all needs the second one. The per-kind
//! fields stay separate, and the only aggregation is [`DegradationReport::any`]
//! — which answers "was anything dropped", the one question that genuinely
//! does not care which kind.

use super::RendererCapabilities;

/// What a frame asked for that the backend could not do.
///
/// Four kinds, because there are four kinds of request a scene can make that
/// a backend might not honour: a drop shadow, a filter over a scope, a filter
/// over already-drawn pixels, and a custom shader. Everything else a backend
/// drops — an image under a transform, a non-solid fill on a character grid —
/// is a property of that backend's medium rather than a missing feature, and
/// stays in that backend's own stats.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DegradationReport {
    /// Drop-shadow requests dropped. A shadow on a shape the backend cannot
    /// blur (a path on vello, anything on a character grid) lands here.
    pub shadows_skipped: u64,
    /// Filter-scope requests dropped: the content was drawn sharp.
    pub filters_skipped: u64,
    /// Backdrop-filter requests dropped: nothing was drawn.
    pub backdrop_filters_skipped: u64,
    /// Custom-shader requests dropped: a program that failed to compile, or a
    /// node whose bounds made a zero-sized target.
    pub shaders_skipped: u64,
}

impl DegradationReport {
    /// Nothing dropped.
    pub const NONE: Self = Self {
        shadows_skipped: 0,
        filters_skipped: 0,
        backdrop_filters_skipped: 0,
        shaders_skipped: 0,
    };

    /// Was anything dropped at all?
    ///
    /// The one question that does not care which kind, and the reason this
    /// type exists as a view rather than a replacement: a host can ask it
    /// without knowing which backend it is talking to.
    pub fn any(&self) -> bool {
        self.shadows_skipped > 0
            || self.filters_skipped > 0
            || self.backdrop_filters_skipped > 0
            || self.shaders_skipped > 0
    }

    /// How many requests were dropped, across all four kinds.
    ///
    /// A size, not a diagnosis: useful for "is this frame badly degraded" and
    /// useless for "which effect should I stop asking for".
    pub fn total(&self) -> u64 {
        self.shadows_skipped
            + self.filters_skipped
            + self.backdrop_filters_skipped
            + self.shaders_skipped
    }

    /// The parts of this report that `capabilities` already said would happen.
    ///
    /// A backend that reports `filters: false` and then counts a skipped
    /// filter has not degraded anything the caller was not warned about. What
    /// is left — the *surprises* — is what a host should actually look at:
    /// a request the backend claimed it could honour, and then did not.
    ///
    /// This is the check that keeps [`RendererCapabilities`] honest. A
    /// capability bit with no implementation behind it is worse than a
    /// missing one, and the way to notice one is to compare what was claimed
    /// against what was dropped.
    pub fn unexpected(&self, capabilities: &RendererCapabilities) -> Self {
        Self {
            shadows_skipped: if capabilities.shadows {
                self.shadows_skipped
            } else {
                0
            },
            filters_skipped: if capabilities.filters {
                self.filters_skipped
            } else {
                0
            },
            backdrop_filters_skipped: if capabilities.backdrop_filters {
                self.backdrop_filters_skipped
            } else {
                0
            },
            shaders_skipped: if capabilities.shaders {
                self.shaders_skipped
            } else {
                0
            },
        }
    }
}
