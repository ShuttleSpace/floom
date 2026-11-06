use crate::shared::SharedNumber;
use std::any::Any;

use floom_core::{
    AnimationSpec, Child, CompositorProp, CreateContext, ElementId, PaintContext, UpdateContext,
    Widget, WidgetAny,
};

/// The shared number, under the name the parallax knows it by.
pub type SharedScroll = SharedNumber;

/// Read the shared offset, treating a poisoned mutex as "no scroll".
pub use crate::shared::read_number as read_scroll;
/// A fresh shared scroll offset.
pub use crate::shared::shared_number as shared_scroll;
/// Write the shared offset. Reports whether it changed, so a caller can skip a
/// frame.
pub use crate::shared::write_number as write_scroll;

/// The parallax layer: the content, moved by `-scroll * factor`.
///
/// It is a layer and nothing else: it declares no size of its own -- v1's
/// `Container` had none either -- so the caller's slot decides how big the
/// layer is.
pub struct ParallaxLayer {
    /// The content being moved.
    child: Option<Child>,
    /// The host-provided scroll offset.
    scroll: SharedScroll,
    /// How much of the offset the content is moved by.
    factor: f32,
    /// The offset this widget last read.
    last_offset: f32,
    /// Bumped when the offset moves.
    ///
    /// The host reconciles a widget's declaration against the one it is
    /// already running, and an unchanged declaration is left alone. The offset
    /// is part of the declaration, so a moved offset is a new declaration --
    /// and the key is what makes the host see it as one.
    rev: u64,
}

impl ParallaxLayer {
    pub fn new(child: Option<Child>, scroll: SharedScroll, factor: f32) -> Self {
        Self {
            child,
            scroll,
            factor,
            last_offset: 0.0,
            rev: 0,
        }
    }

    /// The offset this layer last read.
    pub fn offset(&self) -> f32 {
        self.last_offset
    }
}

impl Widget for ParallaxLayer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Read the offset, and ask for a repaint when it moved.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let offset = read_scroll(&self.scroll);
        if offset != self.last_offset {
            self.last_offset = offset;
            self.rev = self.rev.wrapping_add(1);
            cx.mark_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        match &self.child {
            Some(child) => vec![child.clone()],
            None => Vec::new(),
        }
    }

    /// v1's `translate_y(-scroll * factor)`.
    ///
    /// The engine's translate is a compositor channel, so the layer declares
    /// one: a zero-length tween whose endpoints are the offset it wants,
    /// which the host parks on the node and folds onto the whole subtree.
    fn animation(&self) -> Option<AnimationSpec> {
        let ty = -f64::from(self.last_offset) * f64::from(self.factor);
        Some(
            AnimationSpec::new(ty, ty, 0.0)
                .with_prop(CompositorProp::TranslateY(0.0))
                .with_key(self.rev),
        )
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}
}

impl WidgetAny for ParallaxLayer {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        // A layer that moves: it publishes nothing of its own, and the content
        // inside it publishes whatever it is.
        None
    }
}
