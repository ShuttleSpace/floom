//! Tier-0 primitive widgets.
//!
//! Primitives are *descriptions*: they carry the data needed to lay out, paint
//! and describe a node, and they lower that data into the renderer-neutral
//! contracts in [`floom_core`]. Nothing in this module touches a concrete
//! renderer, a windowing system, or a GPU backend.
//!
//! Invalidation follows the architecture's dirty rules: mutating a primitive
//! bumps an internal revision, and the next [`Widget::update`] marks the
//! element dirty exactly once until the change has been painted.

use floom_accessibility::{SemanticsRole, SemanticsState};

/// The accessibility role vocabulary, under the neutral name the rest of the crate
/// uses, so a widget in another module does not have to know which crate the enum
/// lives in.
pub use floom_accessibility::SemanticsRole as Role;
pub use floom_accessibility::SemanticsState as StateBits;
use floom_core::{
    Axis, Brush, Color, CreateContext, Edges, ElementId, Filter, LayoutRequest, PaintContext,
    PaintStyle, Point, Rect, ResourceId, SemanticsContext, Shadow, Size, UpdateContext, Widget,
};
use floom_types::{FontFamily, FontWeight, Path, Stroke};

/// How an [`Image`] maps its intrinsic size onto the bounds it is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ImageFit {
    /// Stretch to exactly fill the bounds (may distort).
    Fill,
    /// Scale down/up preserving aspect ratio so the whole image is visible.
    #[default]
    Contain,
    /// Scale preserving aspect ratio so the bounds are fully covered (crops).
    Cover,
    /// Draw at intrinsic size, centered.
    None,
}

/// What a primitive reports to assistive technology.
///
/// `SemanticsContext` currently only carries a label, so `role` is published
/// here for the runtime's semantics bridge to consume when it lowers widgets
/// into `floom_accessibility::SemanticsTree`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsHint {
    pub role: SemanticsRole,
    pub label: Option<String>,
    pub state: SemanticsState,
}

impl SemanticsHint {
    /// Decorative / transparent: skipped by assistive technology.
    pub const fn decorative() -> Self {
        Self {
            role: SemanticsRole::None,
            label: None,
            state: SemanticsState::empty(),
        }
    }

    pub fn labeled(role: SemanticsRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: Some(label.into()),
            state: SemanticsState::empty(),
        }
    }

    /// Attaches accessibility state (`CHECKED`, `DISABLED`, `FOCUSED`, …).
    pub fn with_state(mut self, state: SemanticsState) -> Self {
        self.state = self.state.union(state);
        self
    }
}

impl Default for SemanticsHint {
    fn default() -> Self {
        Self::decorative()
    }
}

/// Revision counter driving one-shot `PAINT` invalidation.
///
/// Starts "unpainted" so the first [`Widget::update`] after creation marks the
/// element dirty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision {
    current: u64,
    painted: u64,
}

impl Default for Revision {
    fn default() -> Self {
        Self {
            current: 1,
            painted: 0,
        }
    }
}

impl Revision {
    /// Call from every mutating builder.
    /// Marks the widget as changed since it last painted.
    pub fn bump(&mut self) {
        self.current = self.current.wrapping_add(1);
    }

    /// Marks `id` dirty at most once per batch of mutations.
    /// Reports whether the element needs painting, given the host.
    pub fn sync(&mut self, cx: &mut dyn UpdateContext, id: ElementId) -> bool {
        if self.painted == self.current {
            return false;
        }
        self.painted = self.current;
        cx.mark_dirty(id);
        true
    }
}

/// Publishes `hint` to assistive technology.
///
/// Public because implementing [`Widget::semantics`](floom_core::Widget::semantics)
/// means writing these four steps, and a crate outside this one — the
/// composites in `floom-ui-composites` publish the same way — has no way to
/// reach them otherwise. Nine lines of "set the role, set the label, set the
/// bounds" is not a thing each caller should re-derive, and the early return
/// for an empty hint is the part most likely to be got wrong: publishing an
/// empty role clears whatever an ancestor published.
pub fn emit_semantics(hint: SemanticsHint, cx: &mut dyn SemanticsContext) {
    if hint.role == SemanticsRole::None && hint.label.is_none() {
        return;
    }
    if let Some(label) = hint.label.as_deref() {
        cx.set_label(label);
    }
    let bounds = cx.bounds();
    cx.set_bounds(bounds);
}

/// Strokes a rounded outline of `width` px just inside `bounds`.
///
/// Shared by [`Container`]'s border and by a composite that draws its own
/// outline — decoration boxes are leaves, so a widget that both decorates
/// *and* holds children has to trace the outline itself rather than wrap itself
/// in a `Container`.
///
/// The path is traced on the box edge and pulled in by half the width, because a
/// stroke band straddles its path and a border drawn half outside the bounds
/// would be clipped away by any ancestor that clips.
/// A box outline: how thick, in what colour, and on which edges.
///
/// The edge set is what makes "a footer with a rule along its top" expressible.
/// It defaults to all four so that everything written before it — `border(..)`,
/// every test, every kit crate — means exactly what it meant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    width: f32,
    color: Color,
    edges: Edges,
}

impl Border {
    /// `width <= 0.0` is "no border", so a token that resolves to zero leaves no
    /// hairline behind and callers need not check the sign themselves.
    pub fn new(width: f32, color: Color, edges: Edges) -> Option<Self> {
        (width > 0.0 && !edges.is_empty()).then_some(Self {
            width,
            color,
            edges,
        })
    }

    pub fn width(self) -> f32 {
        self.width
    }

    pub fn color(self) -> Color {
        self.color
    }

    pub fn edges(self) -> Edges {
        self.edges
    }
}

/// Strokes a rounded rectangle inset by half the line width, so the stroke
/// lands inside `bounds` rather than straddling it.
///
/// Public because widgets above this crate draw their own outlines -- the
/// accordion's panel and the dialog's card are two of them -- and the inset is
/// the whole point: a stroke drawn on the edge reads half a pixel too wide and
/// clips against whatever clips the box.
pub fn stroke_rounded_outline(
    cx: &mut dyn PaintContext,
    bounds: Rect,
    radius: f64,
    width: f64,
    color: Color,
) {
    let half = width / 2.0;
    let rect = Rect {
        x0: bounds.x0 + half,
        y0: bounds.y0 + half,
        x1: bounds.x1 - half,
        y1: bounds.y1 - half,
    };
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    cx.stroke_path(
        &Path::rounded_rect(rect, radius),
        &Stroke::new(width, color),
    );
}

// ---------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------

/// A run of static text.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    content: String,
    size: f32,
    /// How heavy the run is drawn. Carried into both the measure and the paint
    /// call, because a widget that measured at `Normal` and painted at `Medium`
    /// would lay out a box too narrow for the glyphs it draws.
    weight: FontWeight,
    /// The typeface the run is set in. Defaults to the platform's own face; a
    /// named family is a request that falls back to the same stack, so setting
    /// one cannot make the text disappear on a device that lacks it.
    family: FontFamily,
    color: Color,
    /// Overrides the accessibility label; defaults to `content`.
    label: Option<String>,
    revision: Revision,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 14.0,
            weight: FontWeight::Normal,
            family: FontFamily::SansSerif,
            color: Color::BLACK,
            label: None,
            revision: Revision::default(),
        }
    }

    /// Font size in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self.revision.bump();
        self
    }

    /// Font weight. floem-ui asks for `Medium` and `SemiBold` at 68 of its 73
    /// weight call sites, so those two are the ones that matter in practice.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self.revision.bump();
        self
    }

    /// The typeface the text is set in.
    ///
    /// The family reaches both the measure and the paint call. Measuring in the
    /// default face and painting in a monospace one would size the box for the
    /// wrong advances and clip the text — the same class of bug as measuring at
    /// `Normal` and painting at `Medium`.
    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self.revision.bump();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    /// Replaces the accessibility label (the visible text stays unchanged).
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    /// In-place content change; bumps the revision so the element repaints.
    pub fn set_content(&mut self, content: impl Into<String>) -> &mut Self {
        self.content = content.into();
        self.revision.bump();
        self
    }

    pub fn font_size(&self) -> f32 {
        self.size
    }

    pub fn font_weight(&self) -> FontWeight {
        self.weight
    }

    pub fn font_family(&self) -> &FontFamily {
        &self.family
    }

    /// In-place family change, for a theme switch on an already-mounted text.
    pub fn set_family(&mut self, family: FontFamily) -> &mut Self {
        self.family = family;
        self.revision.bump();
        self
    }

    /// In-place weight change, for a text that is already mounted — a theme
    /// switch, a pressed state, a row that becomes a heading.
    ///
    /// The bump repaints. Re-measuring is the host's job: the text measure
    /// callback reads the widget, so it sees the new weight, and the box follows.
    pub fn set_weight(&mut self, weight: FontWeight) -> &mut Self {
        self.weight = weight;
        self.revision.bump();
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.label.clone().or_else(|| Some(self.content.clone())) {
            Some(label) if !label.is_empty() => {
                SemanticsHint::labeled(SemanticsRole::StaticText, label)
            }
            _ => SemanticsHint::decorative(),
        }
    }
}

impl Widget for Text {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the laid-out extent of the content at this font size.
    ///
    /// The host applies this as the *default* sizing — an explicit width or
    /// height in the mount style still wins, so `fixed(160, 20)` labels keep
    /// their slot while unwrapped children shrink-wrap their text.
    fn layout_request(&self) -> Option<LayoutRequest> {
        if self.content.is_empty() || self.size <= 0.0 {
            return None;
        }
        let measured = floom_text::measure_weighted_text_in(
            &self.family,
            &self.content,
            self.size,
            self.weight,
            f64::INFINITY,
        );
        if measured.width <= 0.0 || measured.height <= 0.0 {
            return None;
        }
        Some(
            LayoutRequest::new()
                .width(measured.width as f32)
                .height(measured.height as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.content.is_empty() || self.size <= 0.0 {
            return;
        }
        // Nothing to draw when layout gave us no space.
        if cx.bounds().width() <= 0.0 || cx.bounds().height() <= 0.0 {
            return;
        }
        let origin = cx.bounds().origin();
        cx.fill_text_in(
            origin,
            &self.content,
            self.size,
            self.weight,
            self.color,
            &self.family,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }

    /// A run of text is its colour and its size — the whole of what it paints.
    fn paint_style(&self) -> Option<PaintStyle> {
        Some(PaintStyle {
            foreground: Some(self.color),
            font_size: Some(self.size),
            ..PaintStyle::default()
        })
    }
}

// ---------------------------------------------------------------------------
// Icon
// ---------------------------------------------------------------------------

/// A single glyph from an icon font.
///
/// Icons are decorative by default: they emit no semantics unless a label is
/// supplied (which promotes them to an accessible static text node).
#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    glyph: char,
    size: f32,
    color: Color,
    label: Option<String>,
    revision: Revision,
}

impl Icon {
    pub fn new(glyph: char) -> Self {
        Self {
            glyph,
            size: 16.0,
            color: Color::BLACK,
            label: None,
            revision: Revision::default(),
        }
    }

    /// Font size in logical pixels (icons are square).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self.revision.bump();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    /// Gives the icon an accessible name; without it the icon is decorative.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    pub fn glyph(&self) -> char {
        self.glyph
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.label.clone() {
            Some(label) if !label.is_empty() => {
                SemanticsHint::labeled(SemanticsRole::StaticText, label)
            }
            _ => SemanticsHint::decorative(),
        }
    }
}

impl Widget for Icon {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the glyph's advance/extent at the icon's font size.
    /// Icons are square by convention (font icons are designed on a square
    /// em box), so the box takes the larger of the two measured extents.
    fn layout_request(&self) -> Option<LayoutRequest> {
        if self.size <= 0.0 {
            return None;
        }
        let mut buf = [0u8; 4];
        let glyph = self.glyph.encode_utf8(&mut buf);
        let measured = floom_text::measure_text(glyph, self.size, f64::INFINITY);
        if measured.width <= 0.0 || measured.height <= 0.0 {
            return None;
        }
        let side = measured.width.max(measured.height) as f32;
        Some(LayoutRequest::new().width(side).height(side))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.size <= 0.0 {
            return;
        }
        if cx.bounds().width() <= 0.0 || cx.bounds().height() <= 0.0 {
            return;
        }
        let mut buf = [0u8; 4];
        let glyph = self.glyph.encode_utf8(&mut buf);
        let origin = cx.bounds().origin();
        cx.fill_text(origin, glyph, self.size, FontWeight::Normal, self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Image
// ---------------------------------------------------------------------------

/// A registered image resource.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    resource: ResourceId,
    /// Intrinsic size in logical pixels; used by [`ImageFit`].
    intrinsic: Size,
    fit: ImageFit,
    alt: Option<String>,
    revision: Revision,
}

impl Image {
    pub fn new(resource: ResourceId) -> Self {
        Self {
            resource,
            intrinsic: Size::ZERO,
            fit: ImageFit::default(),
            alt: None,
            revision: Revision::default(),
        }
    }

    pub fn intrinsic_size(mut self, size: Size) -> Self {
        self.intrinsic = size;
        self.revision.bump();
        self
    }

    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self.revision.bump();
        self
    }

    /// Alternative text; an image without alt text is decorative.
    pub fn alt(mut self, alt: impl Into<String>) -> Self {
        self.alt = Some(alt.into());
        self.revision.bump();
        self
    }

    pub fn resource(&self) -> ResourceId {
        self.resource
    }

    /// Maps the intrinsic size onto `bounds` according to the fit policy.
    pub fn resolve_bounds(&self, bounds: Rect) -> Rect {
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return Rect::ZERO;
        }
        if self.fit == ImageFit::Fill {
            return bounds;
        }
        let scale = match self.fit {
            ImageFit::None => 1.0,
            ImageFit::Contain => {
                if self.intrinsic.width <= 0.0 || self.intrinsic.height <= 0.0 {
                    return bounds;
                }
                (bounds.width() / self.intrinsic.width).min(bounds.height() / self.intrinsic.height)
            }
            ImageFit::Cover => {
                if self.intrinsic.width <= 0.0 || self.intrinsic.height <= 0.0 {
                    return bounds;
                }
                (bounds.width() / self.intrinsic.width).max(bounds.height() / self.intrinsic.height)
            }
            ImageFit::Fill => unreachable!(),
        };
        let width = self.intrinsic.width * scale;
        let height = self.intrinsic.height * scale;
        let origin = Point::new(
            bounds.x0 + (bounds.width() - width) / 2.0,
            bounds.y0 + (bounds.height() - height) / 2.0,
        );
        Rect::from_origin_size(origin, Size::new(width, height))
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        match self.alt.clone() {
            Some(alt) if !alt.is_empty() => SemanticsHint::labeled(SemanticsRole::Image, alt),
            _ => SemanticsHint {
                role: SemanticsRole::Image,
                label: None,
                state: SemanticsState::empty(),
            },
        }
    }
}

impl Widget for Image {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: whatever the producer declared for the resource
    /// ([`Image::intrinsic_size`]). Without it there is nothing to report —
    /// the renderer only ever sees pixels through the resource registry, so
    /// the box stays under the caller's control (`None` here means "no
    /// opinion", not "zero").
    fn layout_request(&self) -> Option<LayoutRequest> {
        if self.intrinsic.width <= 0.0 || self.intrinsic.height <= 0.0 {
            return None;
        }
        Some(
            LayoutRequest::new()
                .width(self.intrinsic.width as f32)
                .height(self.intrinsic.height as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        let resolved = self.resolve_bounds(bounds);
        if resolved.width() <= 0.0 || resolved.height() <= 0.0 {
            return;
        }
        cx.draw_image(resolved, self.resource);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Container
// ---------------------------------------------------------------------------

/// A decoration box: background, corner radius, inner padding, and elevation.
///
/// Semantically transparent — it never publishes a node of its own.
///
/// The background is a [`Brush`] rather than a colour, so a card can be filled
/// with a gradient; a plain colour still fits through `impl Into<Brush>`, which
/// is why every pre-gradient call site reads exactly as it did.
///
/// There is deliberately **no padding here.** The decoration box holds no children
/// (composition moved to the parent), so a
/// box with nothing inside it has nothing to pad, and a field that is stored and
/// never read is worse than an absent one — it makes a caller believe an inset
/// is being applied. The inset is a *margin on the content's own slot* instead:
/// `LayoutRequest::margin`.
#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    background: Option<Brush>,
    radius: f32,
    /// The outline drawn just inside this box's edge, if any. A separate field
    /// from `background` because it is a stroke and not a fill — the paint
    /// contract grew `stroke_path` with the vector work, and a design system's
    /// hairline divider is the reason it is reachable from a box at all.
    border: Option<Border>,
    /// The drop shadow this card casts, drawn behind its own fill.
    shadow: Option<Shadow>,
    /// The filter this card's **content** — its own fill and every descendant
    /// — is drawn through. Declared rather than pushed in `paint` because the
    /// filter has to see the whole subtree composited, which a paint call
    /// cannot do (see `Widget::filter`).
    filter: Option<Filter>,
    /// A filter over what is *behind* this card, inside its own box. A
    /// separate thing from `filter` rather than a flag on it: one starts from
    /// an empty layer, the other from the pixels already on screen, and a
    /// single field would have to pick.
    backdrop: Option<Filter>,
    revision: Revision,
}

impl Container {
    pub fn new() -> Self {
        Self {
            background: None,
            radius: 0.0,
            border: None,
            shadow: None,
            filter: None,
            backdrop: None,
            revision: Revision::default(),
        }
    }

    /// What the box is filled with: a colour in the common case, a gradient
    /// where the design asks for one.
    pub fn background(mut self, fill: impl Into<Brush>) -> Self {
        self.background = Some(fill.into());
        self.revision.bump();
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.revision.bump();
        self
    }

    /// Draws a `width`-px outline just inside this box's edge.
    ///
    /// The border lands on the decoration box: it is a *stroke* —
    /// the path is traced and the band drawn along it — because filling an outer
    /// rounded rect in the border colour and an inset one in the background
    /// colour cannot work for the common case: a bordered box with a
    /// transparent fill has nothing to paint the inside back with, and would
    /// show the border colour wherever the fill was absent.
    ///
    /// `width <= 0.0` is "no border", so a token that resolves to zero does not
    /// leave a hairline behind.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border = Border::new(width, color, Edges::ALL);
        self.revision.bump();
        self
    }

    /// The same outline, drawn on only some edges.
    ///
    /// A single edge is a **straight band**, not a partial rounded outline: a
    /// footer's rule and a toolbar's divider are straight lines, and curving them
    /// would raise a question this API has no opinion about — what the corners do
    /// where two drawn edges meet. `radius` still applies to the *fill*; only the
    /// outline ignores it. `Edges::ALL` keeps the rounded stroke.
    pub fn border_edges(mut self, width: f32, color: Color, edges: Edges) -> Self {
        self.border = Border::new(width, color, edges);
        self.revision.bump();
        self
    }

    /// The border this box draws, as `(width, colour)`, if any.
    ///
    /// Deliberately lossy: it cannot report *which* edges. Kept because it is
    /// what the pre-edges callers ask, and because a caller comparing two borders
    /// that differ only in edges would get a false "unchanged" from it — which is
    /// why [`Self::border_spec`] exists.
    pub fn border_style(&self) -> Option<(f32, Color)> {
        self.border.map(|b| (b.width, b.color))
    }

    /// The full border, edges included.
    pub fn border_spec(&self) -> Option<Border> {
        self.border
    }

    /// Casts this card's own shadow: its silhouette, blurred by `shadow.blur`
    /// and pushed by its offset, drawn *behind* the background rather than as
    /// a second hard-edged rect underneath it — which is what "elevation"
    /// used to mean here, and what every pseudo-shadow represents.
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self.revision.bump();
        self
    }

    /// Draws this card's content — its own fill *and* everything inside it —
    /// through a blur: Compose's `Modifier.blur`, Flutter's `ImageFiltered`.
    ///
    /// `0.0` (or less) is "no filter": the card is drawn as it is, and no
    /// backend pays for a layer.
    pub fn blur(mut self, radius: f64) -> Self {
        self.filter = Some(Filter::blur(radius));
        self.revision.bump();
        self
    }

    /// [`Self::blur`] with a radius per axis.
    pub fn blur_xy(mut self, radius_x: f64, radius_y: f64) -> Self {
        self.filter = Some(Filter::blur_xy(radius_x, radius_y));
        self.revision.bump();
        self
    }

    /// Blurs what is *behind* this card, inside its own box — the frosted
    /// panel. Drawn before this card's own background, so the tint and the
    /// text on top of it stay sharp while the picture underneath goes soft.
    pub fn backdrop_blur(mut self, radius: f64) -> Self {
        self.backdrop = Some(Filter::blur(radius));
        self.revision.bump();
        self
    }

    /// The in-place form of [`Self::background`].
    ///
    /// Idempotent: setting the same fill dirties nothing, so a theme signal
    /// firing on an unrelated change does not repaint every frame. See
    /// [`Self::set_blur`] for why an in-place form exists at all.
    pub fn set_background(&mut self, fill: impl Into<Brush>) {
        let fill = fill.into();
        if self.background.as_ref() != Some(&fill) {
            self.background = Some(fill);
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::radius`]. Idempotent.
    pub fn set_radius(&mut self, radius: f32) {
        if self.radius != radius {
            self.radius = radius;
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::border`]. Idempotent, and a non-positive
    /// `width` clears the border exactly as the builder does.
    pub fn set_border(&mut self, width: f32, color: Color) {
        self.set_border_edges(width, color, Edges::ALL);
    }

    /// The in-place form of [`Self::border_edges`].
    pub fn set_border_edges(&mut self, width: f32, color: Color, edges: Edges) {
        let next = Border::new(width, color, edges);
        // The whole spec, not the lossy pair: a change of edges alone is a change.
        if self.border != next {
            self.border = next;
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::shadow`]. Idempotent.
    pub fn set_shadow(&mut self, shadow: Option<Shadow>) {
        if self.shadow != shadow {
            self.shadow = shadow;
            self.revision.bump();
        }
    }

    /// The in-place form of [`Self::blur`].
    ///
    /// A builder takes `self` by value, so a caller holding `&mut Container` —
    /// a test driving a mounted widget, or a host mutating one — cannot call
    /// it; this is the same shape `Progress::set_disabled` had to take, for the
    /// same reason.
    pub fn set_blur(&mut self, radius: f64) {
        self.filter = Some(Filter::blur(radius));
        self.revision.bump();
    }

    /// The in-place form of [`Self::backdrop_blur`]. See [`Self::set_blur`]
    /// for why a builder cannot serve here.
    pub fn set_backdrop_blur(&mut self, radius: f64) {
        self.backdrop = Some(Filter::blur(radius));
        self.revision.bump();
    }

    /// The background as whatever it is — the accessor a gradient assertion
    /// reads, since a colour cannot express a ramp.
    pub fn background_brush(&self) -> Option<&Brush> {
        self.background.as_ref()
    }

    /// The background, when it is a flat colour. Not every background survives
    /// this narrowing, and that is the point: a caller that can only take a
    /// colour asks here rather than sampling the brush behind the caller's
    /// back and pretending the answer was exact.
    pub fn background_color(&self) -> Option<Color> {
        self.background.as_ref().and_then(Brush::solid)
    }

    pub fn corner_radius(&self) -> f32 {
        self.radius
    }

    /// The shadow this card casts, if any. Named apart from the builder
    /// because Rust has no overloads and a setter plus getter cannot share a
    /// name; `drop_shadow` keeps the door the builder opened recognisable.
    pub fn drop_shadow(&self) -> Option<Shadow> {
        self.shadow
    }

    /// The filter this card's content is drawn through, if any.
    pub fn content_filter(&self) -> Option<Filter> {
        self.filter
    }

    /// The filter this card applies to what is behind it, if any.
    pub fn backdrop_filter(&self) -> Option<Filter> {
        self.backdrop
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Container {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// The content filter this card declared — answered here rather than in
    /// `paint` so the host can bracket the whole subtree with it.
    fn filter(&self) -> Option<Filter> {
        self.filter
    }

    /// What this card paints, for the inspector's Style readout.
    ///
    /// The narrowing is the same one [`Container::background_color`] already
    /// makes for its own callers: a gradient fill reports no background rather
    /// than its first stop, because one stop is not the fill.
    fn paint_style(&self) -> Option<PaintStyle> {
        let border = self.border_spec();
        let style = PaintStyle {
            background: self.background_color(),
            radius: (self.radius > 0.0).then_some(self.radius),
            border_width: border.map(|b| b.width()),
            border_color: border.map(|b| b.color()),
            // The blur is the one length the style vocabulary has a word for.
            // Offset and spread are dropped rather than folded in; see
            // `PaintStyle`'s module docs.
            elevation: self.shadow.map(|s| s.blur as f32),
            ..PaintStyle::default()
        };
        (!style.is_empty()).then_some(style)
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        // A backdrop filter comes first of all: it consumes what is already
        // behind this box, so it has to run before anything this card draws
        // lands on top of it.
        if let Some(backdrop) = self.backdrop {
            cx.backdrop_filter(bounds, &backdrop);
        }
        // Shadow first: it belongs *behind* the shape that cast it, and the
        // order of ops inside one paint call is the order the scene keeps.
        if let Some(shadow) = self.shadow {
            cx.draw_shadow(bounds, self.radius, &shadow);
        }
        // Fill and outline are independent: a box may be outlined with nothing
        // behind it (a divider) or filled with no outline (a surface). So the
        // fill no longer decides whether anything else is drawn.
        if let Some(fill) = self.background.as_ref() {
            // A solid background takes the fast path the renderers take
            // themselves — one colour needs no per-pixel sampling, and emitting
            // the ops it always did keeps every pre-gradient snapshot honest
            // about what it is looking at.
            match fill.solid() {
                Some(color) if self.radius > 0.0 => {
                    cx.fill_rounded_rect(bounds, self.radius, color)
                }
                Some(color) => cx.fill_rect(bounds, color),
                None if self.radius > 0.0 => cx.fill_rounded_rect_brush(bounds, self.radius, fill),
                None => cx.fill_rect_brush(bounds, fill),
            }
        }
        if let Some(border) = self.border {
            if border.edges().is_all() {
                stroke_rounded_outline(
                    cx,
                    bounds,
                    f64::from(self.radius),
                    f64::from(border.width()),
                    border.color(),
                );
            } else {
                // Filled bands rather than strokes: a stroke straddles its path, so
                // half of it would land outside the box, and a 1px rule would end
                // up half a pixel light on both sides. A band is exactly `width`
                // tall and starts at the edge.
                let w = f64::from(border.width());
                let color = border.color();
                let edges = border.edges();
                if edges.contains(Edges::TOP) {
                    cx.fill_rect(
                        Rect::from_origin_size(bounds.origin(), Size::new(bounds.width(), w)),
                        color,
                    );
                }
                if edges.contains(Edges::BOTTOM) {
                    cx.fill_rect(
                        Rect::from_origin_size(
                            Point::new(bounds.x0, bounds.y1 - w),
                            Size::new(bounds.width(), w),
                        ),
                        color,
                    );
                }
                if edges.contains(Edges::LEFT) {
                    cx.fill_rect(
                        Rect::from_origin_size(bounds.origin(), Size::new(w, bounds.height())),
                        color,
                    );
                }
                if edges.contains(Edges::RIGHT) {
                    cx.fill_rect(
                        Rect::from_origin_size(
                            Point::new(bounds.x1 - w, bounds.y0),
                            Size::new(w, bounds.height()),
                        ),
                        color,
                    );
                }
            }
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Spacer
// ---------------------------------------------------------------------------

/// Empty space along one axis. Paints nothing and emits no semantics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spacer {
    extent: f64,
    axis: Axis,
    /// The flex-grow factor, when this spacer absorbs free space.
    ///
    /// `None` for a fixed-extent spacer. A flexible spacer has one -- 1.0 for
    /// [`Spacer::flexible`], and whatever the caller asked for otherwise.
    grow: Option<f32>,
    revision: Revision,
}

impl Spacer {
    /// Fixed extent in logical pixels along [`Spacer::axis`].
    pub fn new(extent: f64) -> Self {
        Self {
            extent,
            axis: Axis::default(),
            grow: None,
            revision: Revision::default(),
        }
    }

    /// A spacer that absorbs all remaining free space (flex-grow style).
    ///
    /// Represented as `f64::INFINITY` so the layout layer can recognise it
    /// without a separate variant.
    pub fn flexible() -> Self {
        Self::new(f64::INFINITY)
    }

    /// A spacer that absorbs free space by a *factor*.
    ///
    /// [`Spacer::flexible`] is this with a factor of 1.0. A factor is what a
    /// flex row reads to divide the free space between its children, so a kit
    /// whose spacer takes a grow factor -- the design systems' spacers do --
    /// can name the factor rather than rounding it to "all of it".
    pub fn growing(factor: f32) -> Self {
        Self {
            extent: 0.0,
            axis: Axis::default(),
            grow: Some(factor),
            revision: Revision::default(),
        }
    }

    /// The flex-grow factor, when this spacer absorbs free space.
    pub fn grow_factor(&self) -> Option<f32> {
        self.grow
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self.revision.bump();
        self
    }

    pub fn extent(&self) -> f64 {
        self.extent
    }

    pub fn is_flexible(&self) -> bool {
        self.grow.is_some() || self.extent.is_infinite()
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Widget for Spacer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the spacer's extent along its axis. A flexible spacer
    /// ([`Spacer::flexible`]) stores `INFINITY`, which the layout layer
    /// understands as "absorb the free space" — express it as `grow`, the
    /// same thing a flex child would say.
    fn layout_request(&self) -> Option<LayoutRequest> {
        if let Some(grow) = self.grow {
            return Some(LayoutRequest::new().grow(grow));
        }
        if self.is_flexible() {
            return Some(LayoutRequest::new().grow(1.0));
        }
        let extent = self.extent.max(0.0) as f32;
        Some(match self.axis {
            Axis::Horizontal => LayoutRequest::new().width(extent),
            Axis::Vertical => LayoutRequest::new().height(extent),
        })
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, _cx: &mut dyn SemanticsContext, _id: ElementId) {}
}

// ---------------------------------------------------------------------------
// Divider
// ---------------------------------------------------------------------------

/// Where a rule sits on its *cross* axis.
///
/// ## Why this is a parameter and not a decision
///
/// The `md_divider`, `cupertino_divider`, and `fluent_divider` kits all wrote
/// the same thing: a horizontal rule with
/// `height(thickness).width_full()` and no vertical placement at all. In a
/// container taller than the rule, "no placement" means the rule lands on the
/// container's **start edge** — the top.
///
/// This widget was written before those crates were ported, and it centres the
/// rule on the cross axis. That is a different component wearing the same name,
/// and neither position is obviously the correct one: a rule under a heading
/// wants to touch the heading, and a rule between two columns of equal content
/// wants the middle.
///
/// Centring stays the default because it is what every existing caller already
/// gets and two golden tests pin it. `Start` exists so the ported kits can
/// reproduce those kits exactly rather than being silently moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAlignment {
    /// Centred on the cross axis. The default, and this widget's original
    /// behaviour.
    Center,
    /// Flush with the cross axis' start edge — top for a horizontal rule, left
    /// for a vertical one. **What all three kits produced.**
    Start,
}

/// A thin rule separating content along one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Divider {
    thickness: f64,
    color: Color,
    axis: Axis,
    /// Inset applied to both ends of the rule.
    indent: f64,
    alignment: RuleAlignment,
    revision: Revision,
}

impl Divider {
    pub fn new() -> Self {
        Self {
            thickness: 1.0,
            color: Color::rgb(0, 0, 0),
            axis: Axis::default(),
            indent: 0.0,
            alignment: RuleAlignment::Center,
            revision: Revision::default(),
        }
    }

    pub fn thickness(mut self, thickness: f64) -> Self {
        self.thickness = thickness;
        self.revision.bump();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self.revision.bump();
        self
    }

    pub fn indent(mut self, indent: f64) -> Self {
        self.indent = indent;
        self.revision.bump();
        self
    }

    /// Where the rule sits on its cross axis. Defaults to
    /// [`RuleAlignment::Center`]; see that type for why `Start` exists.
    pub fn alignment(mut self, alignment: RuleAlignment) -> Self {
        self.alignment = alignment;
        self.revision.bump();
        self
    }

    /// The rule's cross-axis placement.
    pub fn rule_alignment(&self) -> RuleAlignment {
        self.alignment
    }

    /// The rect the rule occupies inside `bounds`.
    pub fn resolve_bounds(&self, bounds: Rect) -> Rect {
        let thickness = self.thickness.max(0.0);
        match self.axis {
            Axis::Horizontal => {
                let y = match self.alignment {
                    RuleAlignment::Center => bounds.y0 + (bounds.height() - thickness) / 2.0,
                    // No vertical placement is written, which puts the rule on the
                    // container's top edge. Reproduced rather than corrected: a
                    // kit whose rule silently shifted would introduce a visual
                    // regression nobody would attribute to this change.
                    RuleAlignment::Start => bounds.y0,
                };
                Rect {
                    x0: bounds.x0 + self.indent,
                    y0: y,
                    x1: bounds.x1 - self.indent,
                    y1: y + thickness,
                }
            }
            Axis::Vertical => {
                let x = match self.alignment {
                    RuleAlignment::Center => bounds.x0 + (bounds.width() - thickness) / 2.0,
                    RuleAlignment::Start => bounds.x0,
                };
                Rect {
                    x0: x,
                    y0: bounds.y0 + self.indent,
                    x1: x + thickness,
                    y1: bounds.y1 - self.indent,
                }
            }
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Divider {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the rule's thickness on its own axis. The *other* axis
    /// is deliberately unspecified — a rule spans its container, so stating
    /// no extent there leaves the cross-axis stretch free to fill it.
    fn layout_request(&self) -> Option<LayoutRequest> {
        let thickness = self.thickness.max(0.0) as f32;
        Some(match self.axis {
            Axis::Horizontal => LayoutRequest::new().height(thickness),
            Axis::Vertical => LayoutRequest::new().width(thickness),
        })
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let rule = self.resolve_bounds(bounds);
        if rule.width() <= 0.0 || rule.height() <= 0.0 {
            return;
        }
        cx.fill_rect(rule, self.color);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
