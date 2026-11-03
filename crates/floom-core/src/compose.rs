//! Composite widget composition.
//!
//! `floom-core` deliberately does not depend on a layout engine, so this module
//! expresses *intent* in vocabulary any engine understands (axis, alignment,
//! grow) rather than in engine types. [`crate::Widget::children`] hands these
//! to whichever layer owns the tree; that layer lowers each
//! [`LayoutRequest`] into whatever its concrete engine wants.
//!
//! Nothing here allocates identities: children are returned fresh on each call
//! and diffed by [`ChildKey`], so a composite never gives away ownership of its
//! own state.

use std::sync::Arc;

use floom_types::{Insets, Length};

use crate::WidgetAny;

/// Main axis along which a box lays out its own children.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Axis {
    #[default]
    Horizontal,
    Vertical,
}

impl Axis {
    /// The axis a size describes: the other one from `self`.
    pub const fn cross(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

/// Alignment along one axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Align {
    /// Occupy the full cross size (the flex `stretch` default).
    #[default]
    Stretch,
    Start,
    Center,
    End,
    /// Distribute the free space *between* the children, none at either end.
    ///
    /// Only meaningful as a `main_align`, which is why it is here rather than in
    /// the padding vocabulary: it distributes, it does not place one box. floem-ui
    /// asks for it 38 times, nearly always as `justify_between()`.
    SpaceBetween,
}

/// Whether a box participates in its parent's flow.
///
/// Engine-neutral: the lowering layer maps this onto its concrete value
/// (CSS `position`, etc.). `Absolute` escapes the flow so overlays (dialogs,
/// drawers) can cover ancestor content instead of sitting as one more sibling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

/// Offsets of an absolutely positioned box from its containing block.
///
/// `None` on a side means "auto": the layer resolves it from the box's own
/// size. Mirrors the engine's absolute-inset vocabulary without depending on it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Inset {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
}

impl Inset {
    pub const ZERO: Self = Self {
        left: Some(0.0),
        top: Some(0.0),
        right: Some(0.0),
        bottom: Some(0.0),
    };

    /// Fills the containing block on every side.
    pub const fn fill() -> Self {
        Self::ZERO
    }
}

/// Engine-neutral layout intent for one node.
///
/// Every field is optional so that "unspecified" stays distinguishable from
/// "deliberately zero"; the layer lowering this decides the defaults.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutRequest {
    /// `None` is "unspecified", which is not the same as zero: the layer lowering
    /// this decides whether an unspecified dimension is auto or filled.
    pub width: Option<Length>,
    pub height: Option<Length>,
    /// A floor for the main axis, in the same units as [`Self::width`].
    ///
    /// Separate from `height` on purpose, and this is the whole reason: a row that
    /// must be *at least* a touch target tall but may grow with its content cannot
    /// say `height`, because `height` is that content's height now. The direct
    /// translation is `min_height(..)`, and the first component to need it was the
    /// list item, whose row is exactly that shape.
    ///
    /// `None` means "no floor", which is what taffy's `auto` means — so the default
    /// is the absent constraint rather than zero, which would pin every box.
    pub min_width: Option<Length>,
    /// A floor for the cross axis. See [`Self::min_width`].
    pub min_height: Option<Length>,
    /// A ceiling for the main axis, in the same units as [`Self::width`].
    ///
    /// The other half of [`Self::min_width`], and absent for the same reason the
    /// floor was: a dialog's card is `width_pct(100.0)` *and* capped, and with only
    /// `width` one of the two had to be dropped. floem-ui asks for this 22 times
    /// across 8 crates, so it is a real part of the vocabulary rather than one
    /// component's quirk.
    pub max_width: Option<Length>,
    /// A ceiling for the cross axis. See [`Self::max_width`].
    pub max_height: Option<Length>,
    /// Fraction of leftover space this box claims along the parent's main axis.
    pub flex_grow: Option<f32>,
    /// The box's height as a fraction of its width: `1.0` is a square.
    ///
    /// The other dimension is derived rather than stated, which is the whole
    /// point: a media box that must be 16:9 cannot say "width 200, height 112.5"
    /// without knowing its width first. `aspect_ratio(..)` expresses the same
    /// idea, and the first component to need it was the responsive media box.
    pub aspect_ratio: Option<f32>,
    /// Direction this box lays out *its own* children in.
    pub axis: Option<Axis>,
    pub gap: Option<f32>,
    pub main_align: Option<Align>,
    pub cross_align: Option<Align>,
    /// Space reserved *outside* this box, inside its parent.
    ///
    /// This is what a child's own slot asks for when it wants room around itself:
    /// the gap between a card's edge and its content is a margin on the content's
    /// slot, and the parent's fill still covers it.
    pub margin: Option<Insets>,
    /// Space reserved *inside* this box, between its edge and its children.
    ///
    /// Only meaningful on a node that declares children — the engine's word for
    /// it is the box model's, and a leaf has nothing to inset. This is the direct
    /// translation of the box-model padding: the padding a child's own slot asks
    /// for when it wants room around itself, inside the box that holds it.
    ///
    /// The two are not interchangeable: a margin is outside the box and so
    /// changes where the box sits among its siblings, while padding is inside it
    /// and only moves the content. A parent that stretches its children wants
    /// padding; a child in a flow wants margin.
    pub padding: Option<Insets>,
    /// Flow participation; `Absolute` removes the box from its parent's flow.
    pub position: Option<Position>,
    /// Offsets when `position` is `Absolute`; `None` sides resolve to auto.
    pub inset: Option<Inset>,
}

impl LayoutRequest {
    pub const fn new() -> Self {
        Self {
            width: None,
            height: None,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
            flex_grow: None,
            aspect_ratio: None,
            axis: None,
            gap: None,
            main_align: None,
            cross_align: None,
            margin: None,
            padding: None,
            position: None,
            inset: None,
        }
    }

    pub const fn width(mut self, width: f32) -> Self {
        self.width = Some(Length::Px(width));
        self
    }

    /// A floor for the main axis. See [`LayoutRequest::min_width`].
    pub const fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(Length::Px(width));
        self
    }

    /// A floor for the cross axis. See [`LayoutRequest::min_width`].
    pub const fn min_height(mut self, height: f32) -> Self {
        self.min_height = Some(Length::Px(height));
        self
    }

    /// A ceiling for the main axis. See [`LayoutRequest::max_width`].
    pub const fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(Length::Px(width));
        self
    }

    /// A ceiling for the cross axis. See [`LayoutRequest::max_width`].
    pub const fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(Length::Px(height));
        self
    }

    pub const fn height(mut self, height: f32) -> Self {
        self.height = Some(Length::Px(height));
        self
    }

    /// Width as a percentage of the containing block. See [`Length`].
    pub const fn width_pct(mut self, percent: f32) -> Self {
        self.width = Some(Length::Pct(percent));
        self
    }

    /// Height as a percentage of the containing block. See [`Length`].
    pub const fn height_pct(mut self, percent: f32) -> Self {
        self.height = Some(Length::Pct(percent));
        self
    }

    /// Grow to fill leftover space on the parent's main axis.
    pub const fn grow(mut self, grow: f32) -> Self {
        self.flex_grow = Some(grow);
        self
    }

    /// The box's height as a fraction of its width: `1.0` is a square.
    ///
    /// The other dimension is derived rather than stated, which is the whole
    /// point: a media box that must be 16:9 cannot say "width 200, height 112.5"
    /// without knowing its width first.
    pub const fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio);
        self
    }

    pub const fn fill(self) -> Self {
        self.grow(1.0)
    }

    /// Lay this box's own children out along `axis`, `gap` apart.
    pub const fn stack(mut self, axis: Axis, gap: f32) -> Self {
        self.axis = Some(axis);
        self.gap = Some(gap);
        self
    }

    pub const fn main_align(mut self, align: Align) -> Self {
        self.main_align = Some(align);
        self
    }

    pub const fn cross_align(mut self, align: Align) -> Self {
        self.cross_align = Some(align);
        self
    }

    /// Space reserved outside this box, inside its parent. See
    /// [`LayoutRequest::margin`].
    pub const fn margin(mut self, margin: Insets) -> Self {
        self.margin = Some(margin);
        self
    }

    /// Space reserved inside this box, between its edge and its children. See
    /// [`LayoutRequest::padding`].
    pub const fn padding(mut self, padding: Insets) -> Self {
        self.padding = Some(padding);
        self
    }

    /// The same inset on the horizontal axis only.
    pub fn margin_horiz(mut self, value: f64) -> Self {
        let current = self.margin.unwrap_or(Insets::ZERO);
        self.margin = Some(Insets {
            left: value,
            right: value,
            ..current
        });
        self
    }

    /// The same inset on the vertical axis only.
    pub fn margin_vert(mut self, value: f64) -> Self {
        let current = self.margin.unwrap_or(Insets::ZERO);
        self.margin = Some(Insets {
            top: value,
            bottom: value,
            ..current
        });
        self
    }

    /// Padding on the horizontal axis only.
    pub fn padding_horiz(mut self, value: f64) -> Self {
        let current = self.padding.unwrap_or(Insets::ZERO);
        self.padding = Some(Insets {
            left: value,
            right: value,
            ..current
        });
        self
    }

    /// Padding on the vertical axis only.
    pub fn padding_vert(mut self, value: f64) -> Self {
        let current = self.padding.unwrap_or(Insets::ZERO);
        self.padding = Some(Insets {
            top: value,
            bottom: value,
            ..current
        });
        self
    }

    /// Remove this box from its parent's flow and anchor it to every side of
    /// the containing block (a covering overlay).
    pub const fn absolute_fill(mut self) -> Self {
        self.position = Some(Position::Absolute);
        self.inset = Some(Inset::fill());
        self
    }

    /// Fill everything the parent gives and centre what it contains.
    pub fn panel(axis: Axis, gap: f32) -> Self {
        Self::new().fill().stack(axis, gap)
    }
}

/// Stable identity for a child slot across [`crate::Widget::children`] calls.
///
/// The reconciler reuses a mounted child when the key at a position still
/// matches, so keys decide whether state survives an update. Positional keys
/// (`ChildKey::index`) are the default: good enough unless items are inserted
/// in the middle of a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChildKey(u64);

impl ChildKey {
    /// Keyed by position. Reordering reuses slots, so state follows position.
    pub const fn index(index: usize) -> Self {
        Self(index as u64)
    }

    /// Keyed by a caller-supplied id, so state follows the item rather than the
    /// slot. `raw` is mixed with a tag to keep numeric ids from colliding with
    /// positional keys.
    pub const fn keyed(raw: u64) -> Self {
        Self(raw | (1u64 << 63))
    }
}

impl From<usize> for ChildKey {
    fn from(index: usize) -> Self {
        Self::index(index)
    }
}

impl From<&str> for ChildKey {
    fn from(value: &str) -> Self {
        // FNV-1a: stable across runs, unlike `DefaultHasher`.
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in value.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        Self::keyed(hash)
    }
}

/// Deferred construction of a child widget.
///
/// Children are rebuilt rather than kept, so a composite keeps only builders
/// and never hands out ownership of the widgets behind them. The builder yields
/// a `Box<dyn WidgetAny>` — the same type-erased object the host stores — so a
/// `Widget::children` slot plugs directly into the retained tree.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync>;

/// One child slot declared by a composite widget.
#[derive(Clone)]
pub struct Child {
    pub key: ChildKey,
    pub layout: LayoutRequest,
    pub build: ChildBuilder,
    /// A shared-element identity (A6): the same `shared_key` at two spots in
    /// the tree names the source and the destination of a shared-element
    /// transition, so a host that lower these children can pair them up and
    /// fly the source into the destination's box.
    pub shared_key: Option<u64>,
}

impl Child {
    pub fn new(
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        Self {
            key: key.into(),
            layout,
            build: Arc::new(build),
            shared_key: None,
        }
    }

    /// Marks this child as participating in a shared-element transition under
    /// `key`. Two children with the same key are the source and destination.
    pub fn shared(mut self, key: u64) -> Self {
        self.shared_key = Some(key);
        self
    }

    /// Builds the child. Called by whoever owns the tree; only when the key at
    /// this position is new, so surviving children keep their state.
    pub fn build(&self) -> Box<dyn WidgetAny> {
        (self.build)()
    }
}

impl std::fmt::Debug for Child {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Child")
            .field("key", &self.key)
            .field("layout", &self.layout)
            .field("shared_key", &self.shared_key)
            .finish_non_exhaustive()
    }
}
