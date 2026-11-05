// SPDX-License-Identifier: MIT
//! `Surface` — a decorated box that holds its children.
//!
//! This is the spelling of `Container::new(child).style(..)`, the most common
//! shape in floem-ui (165 `Container::new` uses, nearly all of them wrapping
//! something) and the one 2.0 could not express: its `Container` is a *leaf*, so
//! a box can be filled, rounded, outlined, shadowed and filtered — and can hold
//! no content to put those things around.
//!
//! ## Why a separate type rather than a child on `Container`
//!
//! 2.0's model is that composition is declared by the parent
//! (`Child::new(key, LayoutRequest, ..)`) and that leaves are leaves. `Container`
//! is the *decoration description*; `Stack` is the *structure description*; and
//! this is the one thing that is both. Keeping `Container` a leaf means its
//! invalidation model stays the one every other widget uses, while `Surface`
//! adds exactly the thing a leaf cannot have.
//!
//! ## Padding lives here, not on `Container`
//!
//! A leaf has nothing to inset, which is why `Container` once carried a `padding`
//! field that was stored and never read, and why `md-card` rendered with its text
//! against the edge while its test passed. `Surface` is a node with children, so
//! its `padding` is real: it goes out as [`LayoutRequest::padding`] and insets
//! the children inside the box.
//!
//! A margin would *almost* work — with one child the two are geometrically
//! identical — and "almost" is why they are not interchangeable: a margin is per
//! child and lands between children as well, while a padding is one inset
//! around the group. `floom-layout`'s
//! `a_margin_is_per_child_and_a_padding_is_around_the_group` is the measurement.
//!
//! ## One revision, owned here
//!
//! `Surface` keeps its own [`Revision`] rather than forwarding the decoration's,
//! and bumps it from every builder and setter that touches the decoration. That
//! way a frame reports dirty at most once per node however many of its fields
//! moved, which is the same single-report discipline every other widget here
//! keeps. The consequence is that [`Self::decoration`] hands out a *shared*
//! reference: the decoration is not reachable mutably from outside, so it cannot
//! be changed without being invalidated.
//!
//! ## Theme-agnostic
//!
//! Like every 2.0 widget it takes resolved values; deciding which palette is the
//! kit's job. See `docs/porting-floem-ui-to-2.0.md`.

use floom_core::{
    Brush, Child, ChildKey, CreateContext, Edges, ElementId, Filter, LayoutRequest, PaintContext,
    PaintStyle, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::{Color, Insets, Shadow};

use crate::primitives::{Container, Revision, SemanticsHint};

/// A decorated box with children — the composition 2.0 was missing.
///
/// The decoration is a [`Container`] held whole rather than a copy of its fields,
/// so there is exactly one implementation of "fill, then outline, then shadow,
/// then the backdrop filter" in the crate and a `Surface` cannot drift from a
/// bare `Container` about how either is drawn.
#[derive(Clone)]
pub struct Surface {
    decoration: Container,
    padding: Insets,
    /// The caller's own extent for this box, merged with [`Surface::padding`]
    /// in [`Widget::layout_request`]. See [`Self::request`].
    request: LayoutRequest,
    revision: Revision,
    /// Set when the padding moved, so the next `update` can also ask for a
    /// layout pass. A repaint alone would redraw the *previous* frame's geometry,
    /// because `WidgetHost::frame` runs layout only when layout is dirty.
    padding_moved: bool,
    children: Vec<Child>,
}

impl std::fmt::Debug for Surface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Surface")
            .field("decoration", &self.decoration)
            .field("padding", &self.padding)
            .field("children", &self.children.len())
            .finish()
    }
}

impl Surface {
    /// A decorated box with nothing in it yet — the same thing a bare
    /// `Container` is, and the starting point for constructing a decorated box
    /// like `Container::new(child).style(..)`:
    ///
    /// ```ignore
    /// Surface::new()
    ///     .push(|| Box::new(Text::new("hello")))
    ///     .background(color)
    ///     .radius(12.0)
    ///     .padding(Insets::uniform(16.0))
    /// ```
    pub fn new() -> Self {
        Self {
            decoration: Container::new(),
            padding: Insets::ZERO,
            request: LayoutRequest::default(),
            revision: Revision::default(),
            padding_moved: false,
            children: Vec::new(),
        }
    }

    /// Declares this box's own extent — a bar's height, a full-width row.
    ///
    /// Without this a decorated box could only ever shrink-wrap, because its
    /// `layout_request` carried padding and nothing else. A leaf cannot declare a
    /// size at all, and a surface is not a leaf, so the asymmetry was accidental
    /// rather than principled: `md-toolbar` needs "64px tall and the full width of
    /// its parent", and the only way to say that was to put the request on the
    /// *parent's* slot for the child, which spreads one box's styling across two
    /// widgets.
    ///
    /// Padding wins over this request's `padding` field, since the padding is the
    /// surface's own and a caller restating it is restating the same thing.
    ///
    /// ```ignore
    /// Surface::new()
    ///     .request(LayoutRequest::new().height(64.0).width_pct(100.0))
    ///     .push(|| Box::new(Text::new("..")))
    /// ```
    pub fn request(mut self, request: LayoutRequest) -> Self {
        self.request = request;
        self.revision.bump();
        self
    }

    /// Appends a prebuilt child slot.
    pub fn child(mut self, child: Child) -> Self {
        self.children.push(child);
        self
    }

    /// Appends a child auto-keyed by position, with default layout.
    pub fn push(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        let key = ChildKey::index(self.children.len());
        self.children
            .push(Child::new(key, LayoutRequest::new(), build));
        self
    }

    /// Appends a child auto-keyed by position, with an explicit layout.
    pub fn push_layout(
        mut self,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        let key = ChildKey::index(self.children.len());
        self.children.push(Child::new(key, layout, build));
        self
    }

    /// Appends a child keyed by a stable id, so its state follows the item rather
    /// than the slot.
    pub fn push_keyed(
        mut self,
        id: u64,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.children
            .push(Child::new(ChildKey::keyed(id), LayoutRequest::new(), build));
        self
    }

    /// What the box is filled with. See [`Container::background`].
    pub fn background(mut self, fill: impl Into<Brush>) -> Self {
        self.decoration = self.decoration.background(fill);
        self.revision.bump();
        self
    }

    /// Corner radius of the fill and of the outline.
    pub fn radius(mut self, radius: f32) -> Self {
        self.decoration = self.decoration.radius(radius);
        self.revision.bump();
        self
    }

    /// An outline just inside the box's edge.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.decoration = self.decoration.border(width, color);
        self.revision.bump();
        self
    }

    /// An outline on only some edges — a footer with a rule along its top, a
    /// toolbar with a divider under it. Drawn as straight bands; see
    /// [`crate::primitives::Container::border_edges`].
    pub fn border_edges(mut self, width: f32, color: Color, edges: Edges) -> Self {
        self.decoration = self.decoration.border_edges(width, color, edges);
        self.revision.bump();
        self
    }

    /// The shadow this box casts, drawn behind its own fill.
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.decoration = self.decoration.shadow(shadow);
        self.revision.bump();
        self
    }

    /// Draws this box's content — its fill *and* its children — through a blur.
    pub fn blur(mut self, radius: f64) -> Self {
        self.decoration = self.decoration.blur(radius);
        self.revision.bump();
        self
    }

    /// Blurs what is behind this box, inside its own box.
    pub fn backdrop_blur(mut self, radius: f64) -> Self {
        self.decoration = self.decoration.backdrop_blur(radius);
        self.revision.bump();
        self
    }

    /// Space between this box's edge and its children.
    ///
    /// Real padding, not a margin on each child: the box's own size grows by it,
    /// and children stay flush with each other inside it.
    pub fn padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self.padding_moved = true;
        self.revision.bump();
        self
    }

    /// [`Self::padding`] on the horizontal axis only.
    pub fn padding_horiz(mut self, value: f64) -> Self {
        self.padding.left = value;
        self.padding.right = value;
        self.padding_moved = true;
        self.revision.bump();
        self
    }

    /// [`Self::padding`] on the vertical axis only.
    pub fn padding_vert(mut self, value: f64) -> Self {
        self.padding.top = value;
        self.padding.bottom = value;
        self.padding_moved = true;
        self.revision.bump();
        self
    }

    // --- in-place forms ----------------------------------------------------
    //
    // A builder takes `self` by value, so a caller holding `&mut Surface` — a
    // test driving a mounted widget, or a host mutating one — cannot call it.
    // Each of these is idempotent, for the reason `Ripple`'s are: a theme signal
    // fires on unrelated changes too, and a setter that always bumped would
    // repaint the tree on every one of them.

    /// The in-place form of [`Self::background`].
    pub fn set_background(&mut self, fill: impl Into<Brush>) {
        let fill = fill.into();
        if self.decoration.background_brush() != Some(&fill) {
            self.decoration.set_background(fill);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::radius`].
    pub fn set_radius(&mut self, radius: f32) {
        if self.decoration.corner_radius() != radius {
            self.decoration.set_radius(radius);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::border`].
    pub fn set_border(&mut self, width: f32, color: Color) {
        self.set_border_edges(width, color, Edges::ALL);
    }

    /// The in-place form of [`Self::border_edges`].
    ///
    /// The comparison is on the whole spec rather than the `(width, colour)` pair,
    /// because a selected row in `md-item_group` changing *which* edges are
    /// outlined is a change, and a pair comparison would call it unchanged.
    pub fn set_border_edges(&mut self, width: f32, color: Color, edges: Edges) {
        let next = self.decoration.border_spec();
        let wanted = crate::primitives::Border::new(width, color, edges);
        if next != wanted {
            self.decoration.set_border_edges(width, color, edges);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::shadow`].
    pub fn set_shadow(&mut self, shadow: Option<Shadow>) {
        if self.decoration.drop_shadow() != shadow {
            self.decoration.set_shadow(shadow);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::blur`].
    pub fn set_blur(&mut self, radius: f64) {
        if self.decoration.content_filter() != Some(Filter::blur(radius)) {
            self.decoration.set_blur(radius);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::backdrop_blur`].
    pub fn set_backdrop_blur(&mut self, radius: f64) {
        if self.decoration.backdrop_filter() != Some(Filter::blur(radius)) {
            self.decoration.set_backdrop_blur(radius);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::padding`].
    pub fn set_padding(&mut self, padding: Insets) {
        if self.padding != padding {
            self.padding = padding;
            self.padding_moved = true;
            self.revision.bump();
        }
    }

    // --- readers -----------------------------------------------------------

    /// The decoration, shared. Read-only on purpose — see the module note on why
    /// there is no mutable form.
    pub fn decoration(&self) -> &Container {
        &self.decoration
    }

    /// The inset this box keeps between its edge and its children.
    pub fn padding_insets(&self) -> Insets {
        self.padding
    }

    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Surface {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Surface {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// One revision, so a frame reports dirty at most once per node however many
    /// of this widget's fields moved — plus the layout door, but only when the
    /// padding is what moved. Asking for a re-layout because a *colour* changed
    /// would make a theme toggle re-run layout for the whole tree, which is the
    /// cost `mark_layout_dirty` exists to keep off the common path.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let padding_moved = std::mem::take(&mut self.padding_moved);
        self.revision.sync(cx, id);
        if padding_moved {
            cx.mark_layout_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        self.children.clone()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // Merged rather than replaced: the padding is this box's own, and a caller
        // declaring an extent is saying something else. Neither can erase the other,
        // which is the rule the host's own lowering already follows.
        let mut request = self.request;
        if self.padding != Insets::ZERO {
            request.padding = Some(self.padding);
        }
        (request != LayoutRequest::default()).then_some(request)
    }

    /// Declared rather than pushed in `paint`: the filter has to bracket the whole
    /// subtree, which a paint call cannot do.
    fn filter(&self) -> Option<Filter> {
        self.decoration.filter()
    }

    /// The box's decoration, plus its padding — which is this widget's own
    /// property rather than the `Container`'s, so it has to be added here.
    fn paint_style(&self) -> Option<PaintStyle> {
        let uniform = [
            self.padding.top,
            self.padding.right,
            self.padding.bottom,
            self.padding.left,
        ];
        let mut style = self.decoration.paint_style().unwrap_or_default();
        style.padding = uniform
            .iter()
            .all(|v| *v == uniform[0])
            .then_some(uniform[0] as f32);
        (!style.is_empty()).then_some(style)
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.decoration.paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.decoration.semantics(cx, id);
    }
}
