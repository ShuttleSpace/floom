//! Overlay tree with z-order hit priority, focus scopes, and enter/exit animation.

use floem_animation::{AnimKind, AnimationScheduler, CompositorProp, Tween};
use floem_focus::{FocusManager, FocusScopeId};
use floem_types::{ElementId, Point, Rect, Size};

pub use floem_types::{OverlayHandle, OverlayKind};

#[derive(Debug, Clone)]
pub struct OverlayEntry {
    pub id: OverlayHandle,
    pub kind: OverlayKind,
    pub host: ElementId,
    pub bounds: Rect,
    pub z: i32,
    pub modal: bool,
    pub focus_scope: FocusScopeId,
    pub anim: Option<floem_animation::AnimId>,
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

        let compositor = match kind {
            OverlayKind::BottomSheet => CompositorProp::TranslateY(0.0),
            _ => CompositorProp::Opacity(0.0),
        };
        let anim = self.anim.start(
            AnimKind::Tween(Tween {
                from: 0.0,
                to: 1.0,
                duration_secs: 0.15,
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

    pub fn tick_animations(&mut self, now_secs: f64) {
        let _ = self.anim.tick(now_secs);
    }
}

/// Helper for tests constructing overlay bounds.
pub fn sheet_bounds(width: f64, height: f64) -> Rect {
    Rect::from_origin_size(
        Point::new(0.0, height * 0.4),
        Size::new(width, height * 0.6),
    )
}
