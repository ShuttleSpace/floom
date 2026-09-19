//! Overlay tree with z-order hit priority, focus scopes, and enter/exit animation.

use floem_animation::{AnimId, AnimKind, AnimationScheduler, Easing, Tween};
use floem_focus::{FocusManager, FocusScopeId};
use floem_types::{ElementId, Point, Rect, Size};

/// Re-exported so the painter can apply what the scheduler sampled without
/// depending on `floem-animation` directly.
pub use floem_animation::CompositorProp;
pub use floem_types::{OverlayHandle, OverlayKind};

/// How long an overlay takes to enter when it is pushed.
///
/// Short on purpose: an overlay is meant to feel like it was already there and
/// simply became visible, not like it is being performed.
pub const ENTRY_SECS: f64 = 0.15;

/// The shape an overlay's entry takes.
///
/// Decelerating: the surface arrives at speed and settles, rather than being
/// eased *into* its position frame by frame. On an entrance that is the
/// difference between "it was already there and became visible" and "you are
/// watching it being performed", which is the same thing the short duration is
/// for.
pub const ENTRY_EASING: Easing = Easing::EASE_OUT;

#[derive(Debug, Clone)]
pub struct OverlayEntry {
    pub id: OverlayHandle,
    pub kind: OverlayKind,
    pub host: ElementId,
    pub bounds: Rect,
    pub z: i32,
    pub modal: bool,
    pub focus_scope: FocusScopeId,
    pub anim: Option<AnimId>,
    /// The entry animation's current value, or `None` once it has finished (or
    /// when the overlay has no animation).
    ///
    /// This is the *only* channel between the scheduler and the painter: the
    /// manager samples the tween on `tick_animations` and parks the result here
    /// so a painter that ticks and paints in the same frame reads one value
    /// rather than re-deriving one from the clock.
    pub composite: Option<CompositorProp>,
}

/// Manages stacked overlays above the main tree.
#[derive(Debug)]
pub struct OverlayManager {
    entries: Vec<OverlayEntry>,
    pub focus: FocusManager,
    pub anim: AnimationScheduler,
}

impl Default for OverlayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl OverlayManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            focus: FocusManager::new(),
            anim: AnimationScheduler::new(),
        }
    }

    /// Registers an overlay under a handle the caller already issued.
    ///
    /// The handle is passed in rather than minted here because the *host* hands
    /// it out: it is the host that must tell the opener and the content which
    /// overlay they got before this stack has been touched.
    pub fn push(
        &mut self,
        id: OverlayHandle,
        kind: OverlayKind,
        host: ElementId,
        bounds: Rect,
        modal: bool,
        now_secs: f64,
    ) -> OverlayHandle {
        let z = self.entries.iter().map(|e| e.z).max().unwrap_or(0) + 1;
        let scope = self.focus.create_scope(None);
        self.focus.set_active_scope(scope);
        self.focus.register(scope, host);
        self.focus.focus(host);

        // Surfaces that enter by sliding start off their own far edge;
        // everything else fades in. The tween's range *is* the unit the
        // compositor speaks — alpha for a fade, logical pixels for a slide — so
        // the two have to agree, or `TranslateY` would animate a single pixel.
        let (from, to, compositor) = match kind {
            OverlayKind::BottomSheet => (
                bounds.height(),
                0.0,
                CompositorProp::TranslateY(bounds.height()),
            ),
            // A drawer is start-aligned, so it comes in off the leading edge.
            OverlayKind::Drawer => (
                -bounds.width(),
                0.0,
                CompositorProp::TranslateX(-bounds.width()),
            ),
            _ => (0.0, 1.0, CompositorProp::Opacity(0.0)),
        };
        let anim = self.anim.start(
            AnimKind::Tween(Tween {
                from,
                to,
                duration_secs: ENTRY_SECS,
                easing: ENTRY_EASING,
                compositor: Some(compositor),
            }),
            now_secs,
        );

        self.entries.push(OverlayEntry {
            id,
            kind,
            host,
            bounds,
            z,
            modal,
            focus_scope: scope,
            anim: Some(anim),
            // Paint the first frame at the animation's start value: the entry
            // is real and hit-testable from the moment it is pushed, it just
            // is not fully drawn yet.
            composite: Some(compositor),
        });
        id
    }

    pub fn pop(&mut self) -> Option<OverlayEntry> {
        let entry = self.entries.pop()?;
        if let Some(top) = self.entries.last() {
            self.focus.set_active_scope(top.focus_scope);
            self.focus.focus(top.host);
        } else {
            self.focus.blur();
        }
        Some(entry)
    }

    /// Removes a *specific* overlay by id (not just the top of the stack), so a
    /// control can close its own popup without disturbing overlays opened on
    /// top of it. The focus scope of whatever remains on top is restored, or
    /// the manager blurs when the stack empties.
    pub fn remove(&mut self, id: OverlayHandle) -> Option<OverlayEntry> {
        let pos = self.entries.iter().position(|e| e.id == id)?;
        let entry = self.entries.remove(pos);
        if let Some(top) = self.entries.last() {
            self.focus.set_active_scope(top.focus_scope);
            self.focus.focus(top.host);
        } else {
            self.focus.blur();
        }
        Some(entry)
    }

    /// Refreshes an overlay's hit bounds after layout discovers the content is
    /// a different size than the opener guessed.
    pub fn set_bounds(&mut self, id: OverlayHandle, bounds: Rect) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.bounds = bounds;
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn top(&self) -> Option<&OverlayEntry> {
        self.entries.last()
    }

    /// Hit-test overlays front-to-back; modal blocks passes-through.
    pub fn hit_test(&self, point: Point) -> Option<OverlayHandle> {
        for e in self.entries.iter().rev() {
            if e.bounds.contains(point) {
                return Some(e.id);
            }
            if e.modal {
                // Modal consumes the hit even outside content (scrim).
                return Some(e.id);
            }
        }
        None
    }

    /// Samples every running entry animation at `now_secs`, parking each
    /// result on its entry for the painter to read.
    ///
    /// Returns whether anything is still running, which is what tells the
    /// frame loop to ask for another frame. Once an animation finishes its
    /// entry's `composite` goes back to `None` — the final opacity of 1.0 and
    /// the final offset of 0 are what "no compositing" already means, so
    /// leaving them set would keep re-writing paint payloads forever and the
    /// scene would never stop taking damage.
    pub fn tick_animations(&mut self, now_secs: f64) -> bool {
        for sample in self.anim.tick(now_secs) {
            let Some(entry) = self.entries.iter_mut().find(|e| e.anim == Some(sample.id)) else {
                continue;
            };
            if sample.finished {
                entry.anim = None;
                entry.composite = None;
            } else {
                entry.composite = sample.compositor;
            }
        }
        self.is_animating()
    }

    /// Whether any entry is still playing its animation.
    pub fn is_animating(&self) -> bool {
        self.entries.iter().any(|e| e.anim.is_some())
    }

    /// The current entry animation of `id`, or `None` when it has settled (or
    /// the overlay is not open).
    pub fn composite(&self, id: OverlayHandle) -> Option<CompositorProp> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.composite)
    }
}

/// Helper for tests constructing overlay bounds.
pub fn sheet_bounds(width: f64, height: f64) -> Rect {
    Rect::from_origin_size(
        Point::new(0.0, height * 0.4),
        Size::new(width, height * 0.6),
    )
}
