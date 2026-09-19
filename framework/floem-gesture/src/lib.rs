//! Gesture arena: competing recognizers with exclusive ownership.
//!
//! The arena ([`GestureArena`]) arbitrates competing pointer recognizers
//! (tap / drag / scroll / scale / native scroll) for a pointer sequence.
//! [`momentum`] provides the post-gesture fling physics that pairs with the
//! scroll recognizer: feed it pointer velocities on lift, then drain
//! decelerating deltas on each frame.

use floem_types::{ElementId, Point};

pub mod momentum;
pub mod touch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecognizerId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GestureKind {
    Tap,
    Drag,
    Scroll,
    Scale,
    /// Native view / scrollable that must win over app gestures when claimed.
    NativeScroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecognizerState {
    Possible,
    Competing,
    Accepted,
    Rejected,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Recognizer {
    pub id: RecognizerId,
    pub kind: GestureKind,
    pub element: ElementId,
    pub state: RecognizerState,
    /// Higher priority wins when multiple accept simultaneously.
    pub priority: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointerSample {
    Down { position: Point },
    Move { position: Point },
    Up { position: Point },
}

/// Arena resolving conflicting recognizers for a pointer sequence.
#[derive(Debug, Default)]
pub struct GestureArena {
    next_id: u32,
    members: Vec<Recognizer>,
    owner: Option<RecognizerId>,
    down_at: Option<Point>,
    /// When true, NativeScroll claims cancel app Tap/Drag.
    native_scroll_active: bool,
}

impl GestureArena {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, kind: GestureKind, element: ElementId, priority: i32) -> RecognizerId {
        let id = RecognizerId(self.next_id);
        self.next_id += 1;
        self.members.push(Recognizer {
            id,
            kind,
            element,
            state: RecognizerState::Possible,
            priority,
        });
        id
    }

    pub fn owner(&self) -> Option<RecognizerId> {
        self.owner
    }

    pub fn recognizer(&self, id: RecognizerId) -> Option<&Recognizer> {
        self.members.iter().find(|r| r.id == id)
    }

    pub fn reset(&mut self) {
        self.members.clear();
        self.owner = None;
        self.down_at = None;
        self.native_scroll_active = false;
        self.next_id = 0;
    }

    pub fn set_native_scroll_active(&mut self, active: bool) {
        self.native_scroll_active = active;
        if active {
            self.cancel_kinds(&[GestureKind::Tap, GestureKind::Drag, GestureKind::Scale]);
            if let Some(r) = self
                .members
                .iter_mut()
                .find(|r| r.kind == GestureKind::NativeScroll)
            {
                r.state = RecognizerState::Accepted;
                self.owner = Some(r.id);
            }
        }
    }

    fn cancel_kinds(&mut self, kinds: &[GestureKind]) {
        for r in &mut self.members {
            if kinds.contains(&r.kind)
                && r.state != RecognizerState::Rejected
                && r.state != RecognizerState::Cancelled
            {
                r.state = RecognizerState::Cancelled;
            }
        }
    }

    /// Feed a pointer sample; may accept/reject members.
    pub fn on_pointer(&mut self, sample: PointerSample) {
        match sample {
            PointerSample::Down { position } => {
                self.down_at = Some(position);
                for r in &mut self.members {
                    r.state = RecognizerState::Competing;
                }
            }
            PointerSample::Move { position } => {
                let Some(start) = self.down_at else {
                    return;
                };
                let dx = position.x - start.x;
                let dy = position.y - start.y;
                let dist2 = dx * dx + dy * dy;

                if self.native_scroll_active {
                    return;
                }

                // Movement cancels Tap; enables Drag / Scroll / Scale heuristics.
                if dist2 > 25.0 {
                    self.reject_kind(GestureKind::Tap);
                    self.try_accept_best(&[
                        GestureKind::Drag,
                        GestureKind::Scroll,
                        GestureKind::Scale,
                        GestureKind::NativeScroll,
                    ]);
                }
            }
            PointerSample::Up { position } => {
                let Some(start) = self.down_at else {
                    return;
                };
                let dx = position.x - start.x;
                let dy = position.y - start.y;
                let dist2 = dx * dx + dy * dy;
                if self.owner.is_none() && dist2 <= 25.0 && !self.native_scroll_active {
                    self.try_accept_best(&[GestureKind::Tap]);
                }
                // Reject remaining competitors.
                for r in &mut self.members {
                    if r.state == RecognizerState::Competing {
                        r.state = RecognizerState::Rejected;
                    }
                }
                self.down_at = None;
            }
        }
    }

    fn reject_kind(&mut self, kind: GestureKind) {
        for r in &mut self.members {
            if r.kind == kind && r.state == RecognizerState::Competing {
                r.state = RecognizerState::Rejected;
            }
        }
    }

    fn try_accept_best(&mut self, kinds: &[GestureKind]) {
        if self.owner.is_some() {
            return;
        }
        let mut best: Option<(RecognizerId, i32)> = None;
        for r in &self.members {
            if r.state != RecognizerState::Competing || !kinds.contains(&r.kind) {
                continue;
            }
            match best {
                None => best = Some((r.id, r.priority)),
                Some((_, p)) if r.priority > p => best = Some((r.id, r.priority)),
                _ => {}
            }
        }
        if let Some((id, _)) = best {
            self.accept(id);
        }
    }

    pub fn accept(&mut self, id: RecognizerId) {
        for r in &mut self.members {
            if r.id == id {
                r.state = RecognizerState::Accepted;
                self.owner = Some(id);
            } else if r.state == RecognizerState::Competing || r.state == RecognizerState::Possible
            {
                r.state = RecognizerState::Cancelled;
            }
        }
    }
}
