//! An accordion, and the select group that shares its expand/collapse machinery.
//!
//! Both are "one of several, and the chosen one shows more". They share a style type, an indicator builder and an expansion state machine, so they share a file -- and with them the inline tests that pin that state machine.
//!
//! It lived in `floom-widgets/src/compose.rs` beside the layout vocabulary
//! (`Stack`, `List`, `Grid`, `Conditional`, `Crossfade`). The two are not the
//! same kind of thing: a `Stack` arranges children along an axis, which is
//! arithmetic; this decides *what a control looks like*, which is a product
//! decision. So it moved up a layer with the tree, the counter and the select,
//! and the dependency runs one way -- this crate composes the engine's
//! vocabulary rather than the other way round.

use std::any::Any;
use std::sync::atomic::{Ordering};
use std::sync::{Arc, Mutex};

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Axis, Child, ChildBuilder, ChildKey, Color, CreateContext, ElementId, Insets, LayoutRequest,
    PaintContext, PresenceTransition, SemanticsContext, UpdateContext, Widget, WidgetAny,
};

use floom_widgets::primitives::{SemanticsHint, Text};
use floom_widgets::{Conditional, Pressable, Stack};

// ---------------------------------------------------------------------------
// Accordion
// ---------------------------------------------------------------------------

/// Builds one panel's expand/collapse indicator, told which way the panel is.
///
/// A closure over the flag rather than two separate widgets (`collapsed` /
/// `expanded`) because the caller cannot know the state when it declares the
/// panel — that is the accordion's answer, and it changes after mount.
pub type AccordionIndicator = Arc<dyn Fn(bool) -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Resolved, theme-agnostic style for an [`Accordion`].
///
/// Like every 2.0 widget this takes explicit values: deciding *which* palette
/// and *which* metrics is the design system's job, so a kit resolves them once
/// (`floom-core-theme` -> `MdExpansionPanels::resolve`) and hands the result here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AccordionStyle {
    /// Height of each header row, in logical px.
    pub header_height: f32,
    /// Inset between the header's outline and its content, horizontally.
    pub header_padding: f32,
    /// Inset between the revealed content's edge and its own content.
    pub content_padding: f32,
    /// Extra horizontal indent the revealed content is pushed by, so it reads as
    /// belonging to the panel rather than to the page.
    pub content_indent: f32,
    /// Gap between stacked panels.
    pub gap: f32,
    /// Header title colour.
    pub title_color: Color,
    /// Header title size, in logical px.
    pub title_size: f32,
    /// Outline width drawn just inside each header's edge. `0.0` draws none.
    pub border_width: f32,
    /// Outline colour.
    pub border_color: Color,
    /// Corner radius of the header's outline.
    pub radius: f32,
}

impl Default for AccordionStyle {
    fn default() -> Self {
        Self {
            header_height: 48.0,
            header_padding: 16.0,
            content_padding: 16.0,
            content_indent: 0.0,
            gap: 8.0,
            title_color: Color::BLACK,
            title_size: 14.0,
            border_width: 1.0,
            border_color: Color::rgba(0, 0, 0, 32),
            radius: 12.0,
        }
    }
}

/// One panel of an [`Accordion`].
struct AccordionPanel {
    title: String,
    indicator: AccordionIndicator,
    content: ChildBuilder,
}

/// A stack of panels, each a clickable header that reveals its content.
///
/// This is the **stateful** shape the floom-ui kits all share: a value that
/// changes after mount, a subtree that mounts and unmounts with it, and — the
/// part that is easy to get wrong — a header that must *keep its identity*
/// across the toggle, so keyboard focus is not thrown away by the very act of
/// activating it. All three kits ship this component and their copies differ
/// only in tokens and glyph names, so it is promoted here once.
///
/// ## Where the state lives, and why it is shared
///
/// The click lands on a [`Pressable`] deep inside a header subtree, while the
/// answer — which panels are open — is needed by that panel's body, by its
/// indicator, and in single-panel mode by every *other* panel. None of those
/// owns the accordion's node. So the open set is a shared
/// `Arc<Mutex<Vec<bool>>>`, exactly as [`Tabs`] shares its selection, and every
/// widget that cares reads it in its own `update` and marks itself. The host
/// visits every node each frame, so no notification has to travel.
///
/// ## What this cannot animate
///
/// The reveal is a mount/unmount, not a height transition. v1 animated
/// `height: 0 -> computed` (`md-expand_transition`); 2.0's implicit layer
/// cannot, because `run_layout` only runs when layout is dirty and no
/// compositor channel carries a size. [`Accordion::content_transition`] is
/// therefore compositor-only — a fade, a grow or a slide over the revealed
/// subtree — while the box appears at its final height. See
/// `docs/2.0/MOTION-GAP.md` (B4, the "size" entry under "not done").
pub struct Accordion {
    panels: Vec<AccordionPanel>,
    style: AccordionStyle,
    /// Whether more than one panel may be open at once.
    multiple: bool,
    open: Arc<Mutex<Vec<bool>>>,
    on_change: Option<Arc<dyn Fn(usize, bool) + Send + Sync>>,
    content_transition: Option<PresenceTransition>,
}

impl std::fmt::Debug for Accordion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Accordion")
            .field("panels", &self.panels.len())
            .field("multiple", &self.multiple)
            .field("open", &self.open_state())
            .finish_non_exhaustive()
    }
}

impl Accordion {
    pub fn new() -> Self {
        Self {
            panels: Vec::new(),
            style: AccordionStyle::default(),
            multiple: false,
            open: Arc::new(Mutex::new(Vec::new())),
            on_change: None,
            content_transition: None,
        }
    }

    /// Adds a panel.
    ///
    /// `indicator` is handed the panel's current state and returns the widget
    /// that shows it. `content` builds the body, which is mounted only while the
    /// panel is open (and while its exit transition is still playing).
    pub fn panel(
        mut self,
        title: impl Into<String>,
        indicator: impl Fn(bool) -> Box<dyn WidgetAny> + Send + Sync + 'static,
        content: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        let index = self.panels.len();
        self.lock().resize(index + 1, false);
        self.panels.push(AccordionPanel {
            title: title.into(),
            indicator: Arc::new(indicator),
            content: Arc::new(content),
        });
        self
    }

    /// Allow more than one panel open at a time. Off by default, which is the
    /// accordion rule: opening one closes the others.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Called with `(panel_index, expanded)` whenever a panel's state changes
    /// from a click.
    ///
    /// `Send + Sync` is required, not incidental: the click lands on a
    /// [`Pressable`] built inside a `Child` builder closure, and the host may
    /// hand that closure between threads.
    pub fn on_change(mut self, callback: impl Fn(usize, bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The resolved style. See [`AccordionStyle`].
    pub fn style(mut self, style: AccordionStyle) -> Self {
        self.style = style;
        self
    }

    /// A transition played by a panel's content as it appears and disappears.
    ///
    /// Optional and compositor-only — see the type's note on what cannot be
    /// animated. Left unset, the content mounts and unmounts instantly, which is
    /// what v1's `dyn_view` did.
    pub fn content_transition(mut self, transition: PresenceTransition) -> Self {
        self.content_transition = Some(transition);
        self
    }

    /// Which panels are open right now.
    pub fn open_state(&self) -> Vec<bool> {
        self.lock().clone()
    }

    /// Whether panel `index` is open.
    pub fn is_open(&self, index: usize) -> bool {
        self.lock().get(index).copied().unwrap_or(false)
    }

    /// Opens or closes panel `index` without going through a click.
    ///
    /// The imperative door, the twin of [`Tabs::set_selected`]. It does **not**
    /// fire [`Accordion::on_change`]: that callback reports what a *person* did,
    /// and a programmatic caller already knows. It does apply the
    /// single/multiple rule, so opening a panel in accordion mode closes the
    /// others.
    pub fn set_open(&mut self, index: usize, open: bool) {
        let mut guard = self.lock();
        if guard.len() <= index {
            guard.resize(index + 1, false);
        }
        if !self.multiple {
            for slot in guard.iter_mut() {
                *slot = false;
            }
        }
        guard[index] = open;
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn titles(&self) -> Vec<&str> {
        self.panels.iter().map(|p| p.title.as_str()).collect()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<bool>> {
        // A poisoned lock means some other holder panicked mid-toggle. The data
        // is a `Vec<bool>` only ever written whole, so it is still coherent, and
        // refusing to draw an accordion because an unrelated callback panicked
        // would be the worse failure.
        self.open.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Flips panel `index` and returns its new state, applying the
    /// single/multiple rule.
    fn toggle(open: &Mutex<Vec<bool>>, index: usize, multiple: bool) -> bool {
        let mut guard = open.lock().unwrap_or_else(|e| e.into_inner());
        let next = !guard.get(index).copied().unwrap_or(false);
        if !multiple {
            for slot in guard.iter_mut() {
                *slot = false;
            }
        }
        if let Some(slot) = guard.get_mut(index) {
            *slot = next;
        }
        next
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::decorative()
    }
}

impl Default for Accordion {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Accordion {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty.
    ///
    /// Nothing changed *here*: the accordion still owns the same panel slots, and
    /// what moved is the shared open set. Every panel reads that set in its own
    /// `update` and reports the change itself, so marking dirty here too would
    /// be a second report of a change already reported — the same reasoning (and
    /// deliberately the same wording) as [`Crossfade::update`].
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(self.panels.len());
        for (index, panel) in self.panels.iter().enumerate() {
            let open = self.open.clone();
            let on_change = self.on_change.clone();
            let multiple = self.multiple;
            let toggle = Arc::new(move || {
                let next = Self::toggle(&open, index, multiple);
                if let Some(callback) = &on_change {
                    callback(index, next);
                }
            }) as Arc<dyn Fn() + Send + Sync>;

            let mut body = Conditional::new(true);
            // Seeded closed, and *seeded* rather than left at the constructor's
            // "not yet seen": see `Conditional::sync_when`.
            body.sync_when(false);
            let content = panel.content.clone();
            body = body.show_with(
                ChildKey::index(1),
                LayoutRequest::new().margin(content_insets(self.style)),
                move || content(),
            );
            if let Some(transition) = self.content_transition {
                body = body.presence(transition);
            }

            let leaf = AccordionPanelLeaf {
                index,
                open: self.open.clone(),
                last_open: false,
                style: self.style,
                header: header_child(self, index, panel, toggle),
                body,
            };
            out.push(Child::new(
                ChildKey::index(index),
                LayoutRequest::new(),
                move || Box::new(leaf.clone()),
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, self.style.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}

/// The inset a revealed panel's body sits inside its panel box.
///
/// v1 wrote this as `padding_horiz(content_padding + indent)` and
/// `padding_vert(content_padding)` on the box holding the body. 2.0's `Container`
/// is a leaf, so the inset is a *margin on the body's slot* instead — see
/// `LayoutRequest::margin`.
fn content_insets(style: AccordionStyle) -> Insets {
    let horizontal = f64::from(style.content_padding + style.content_indent);
    let vertical = f64::from(style.content_padding);
    Insets {
        left: horizontal,
        right: horizontal,
        top: vertical,
        bottom: vertical,
    }
}

/// The header slot: the whole of the panel's top row, clickable.
///
/// Keyed by [`ChildKey::index`] on purpose. The row has to survive the toggle —
/// a rebuilt header is a new node, so the keyboard focus [`Pressable`] requests
/// on the press that *causes* the toggle would be thrown away by activating it.
/// Everything inside that has to change therefore has to be a widget that
/// re-declares its own children, which is what [`AccordionIndicatorSlot`] is.
fn header_child(
    accordion: &Accordion,
    index: usize,
    panel: &AccordionPanel,
    toggle: Arc<dyn Fn() + Send + Sync + 'static>,
) -> Child {
    let style = accordion.style;
    let open = accordion.open.clone();
    let title = panel.title.clone();
    let indicator = panel.indicator.clone();
    Child::new(
        ChildKey::index(0),
        LayoutRequest::new().height(style.header_height).fill(),
        move || {
            let open_flag = open.clone();
            let row = Stack::new(Axis::Horizontal)
                .align(Align::Start, Align::Center)
                .push({
                    let title = title.clone();
                    move || {
                        Box::new(
                            Text::new(title.clone())
                                .size(style.title_size)
                                .color(style.title_color),
                        )
                    }
                })
                // v1 asked for `justify_between`, which 2.0's single `Align`
                // cannot say; a flexible spacer is the flex spelling of "push
                // these apart".
                .push(|| Box::new(floom_widgets::Spacer::flexible()))
                .push({
                    let indicator = indicator.clone();
                    let open_flag = open_flag.clone();
                    move || {
                        Box::new(AccordionIndicatorSlot {
                            open: open_flag.clone(),
                            index,
                            indicator: indicator.clone(),
                            last_open: false,
                        })
                    }
                });

            Box::new(
                Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().fill(),
                    move || Box::new(row.clone()),
                ))
                .label(title.clone())
                .expanded_from(move || is_open(&open_flag, index))
                .on_click({
                    let toggle = toggle.clone();
                    move || toggle()
                }),
            )
        },
    )
}

/// One panel of an [`Accordion`]: the header plus the body its open flag mounts
/// and unmounts.
///
/// Stable for the panel's whole life, which is what keeps the header's focus.
/// Everything it shows is derived from the shared open set on every `update`.
///
/// It also paints the panel's own outline, which is the one place this port
/// departs from v1: v1 outlined the *header row* with
/// `.border(1.0).border_color(..)`, and 2.0 cannot reproduce that shape — the
/// box that would have to carry the outline is a `Container`, which is a leaf and
/// so cannot hold the row it would be outlining (see `docs/`). Outlining the
/// panel instead is both what is available and what Material 3's outlined
/// expansion panel is: one box holding the header and, when open, the content.
#[derive(Clone)]
pub(crate) struct AccordionPanelLeaf {
    index: usize,
    open: Arc<Mutex<Vec<bool>>>,
    /// The open state this leaf last acted on.
    last_open: bool,
    style: AccordionStyle,
    header: Child,
    /// The reveal, held as a value rather than mounted as a node of its own so
    /// the presence transition is declared on *this* node — one node instead of a
    /// wrapper, and the transition's `animation_finished` lands here too.
    body: Conditional,
}

impl Widget for AccordionPanelLeaf {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if is_open(&self.open, self.index) == self.last_open {
            return;
        }
        self.last_open = !self.last_open;
        // `Conditional::update` is what marks this node dirty *and* reconciles
        // its children; passing this node's id is deliberate — the transition is
        // declared here, so the host re-syncs the declaration against it.
        self.body.set_when(self.last_open);
        self.body.update(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        let mut out = vec![self.header.clone()];
        out.extend(self.body.children());
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, 0.0))
    }

    fn animation(&self) -> Option<floom_core::AnimationSpec> {
        self.body.animation()
    }

    fn animation_finished(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.body.animation_finished(cx, id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if self.style.border_width <= 0.0 {
            return;
        }
        let bounds = cx.bounds();
        floom_widgets::stroke_rounded_outline(
            cx,
            bounds,
            f64::from(self.style.radius),
            f64::from(self.style.border_width),
            self.style.border_color,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(SemanticsHint::decorative(), cx);
    }
}

/// The disclosure indicator of one panel.
///
/// Stable across toggles — that is what lets the header be — but it re-declares
/// its own single child keyed by the open flag, which is what swaps one glyph for
/// the other. A bare [`Child`] could not do this: the host only rebuilds a child
/// whose key *changed*, and everything inside a stable header has a stable key.
#[derive(Clone)]
pub(crate) struct AccordionIndicatorSlot {
    open: Arc<Mutex<Vec<bool>>>,
    index: usize,
    indicator: AccordionIndicator,
    last_open: bool,
}

impl Widget for AccordionIndicatorSlot {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if is_open(&self.open, self.index) == self.last_open {
            return;
        }
        self.last_open = !self.last_open;
        // Reconcile for the new glyph, and repaint so the row is re-measured
        // around an indicator of a different size.
        cx.mark_children_dirty(id);
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        let indicator = self.indicator.clone();
        let open = self.last_open;
        vec![Child::new(
            // The flag is the key, so the host mounts a *different* widget rather
            // than re-using the previous glyph and diffing two builders that
            // both mean "the indicator".
            ChildKey::keyed(INDICATOR_KEY_BASE ^ (open as u64)),
            LayoutRequest::new(),
            move || indicator(open),
        )]
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(SemanticsHint::decorative(), cx);
    }
}

/// Distinct from the positional keys an accordion's slots use, so a keyed
/// indicator can never collide with one of them.
const INDICATOR_KEY_BASE: u64 = 0xACC0_0000_0000_0000;

/// Whether panel `index` of a shared open set is open.
fn is_open(open: &Mutex<Vec<bool>>, index: usize) -> bool {
    open.lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(index)
        .copied()
        .unwrap_or(false)
}

/// How many rows of a [`SelectGroup`] may be selected at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// At most one. Choosing a row replaces the selection.
    #[default]
    Single,
    /// Any number. Choosing a row toggles it.
    Multiple,
}

/// The two appearances of a [`SelectGroup`] row.
///
/// A resolved style struct rather than a builder chain, because every row needs
/// both appearances and the choice between them happens in the row's `update`,
/// long after any builder ran.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectGroupStyle {
    /// Gap between rows.
    pub gap: f32,
    /// The direction the rows run in.
    ///
    /// Vertical is the default and what every existing caller gets. A group of
    /// chips runs the other way -- v1's chip group was a *row* of chips, not a
    /// column of them -- and the direction is the caller's answer, not the
    /// engine's.
    pub axis: Axis,
    /// A fixed height for every row, when the rows have one.
    ///
    /// `None` lets each row be as tall as its content, which is what a list of
    /// items wants. A chip wants a fixed height, and the content's own height
    /// cannot be relied on to produce it.
    pub height: Option<f32>,
    /// Row padding: `pad` horizontally, `pad / 2` vertically, as v1 wrote it.
    pub pad: f32,
    /// Where a row's content sits inside the row's box.
    ///
    /// The two shapes the floem-ui kits ask for are not the same shape: a list of
    /// items puts its text directly under the top padding, while a chip centres
    /// its label in the chip. The padding is symmetric, so centring the content
    /// in the content box puts it in the middle of the whole box -- which is
    /// what v1's `items_center` did.
    pub content_placement: ContentPlacement,
    /// Whether tapping the chosen row in single mode un-chooses it.
    ///
    /// A radio does not; a chip does. v1's item group kept the radio's answer
    /// and v1's chip group kept the chip's, and the difference is the whole
    /// reason a chip group is not an item group.
    pub toggles_off_in_single_mode: bool,
    /// Whether `mandatory` is enforced when several rows may be chosen.
    ///
    /// v1's item group enforced it and v1's chip group did not -- in v1's chip
    /// group the `mandatory` prop was only read in the single-mode branch, so
    /// several chips could all be un-chosen. The flag is here so both answers
    /// survive.
    pub mandatory_in_multiple_mode: bool,
    pub radius: f32,
    /// The outline's thickness. `0.0` draws none.
    pub border_width: f32,
    /// Unselected row.
    pub background: Color,
    pub foreground: Color,
    pub border_color: Color,
    /// Selected row.
    pub selected_background: Color,
    pub selected_foreground: Color,
    pub selected_border_color: Color,
}

impl Default for SelectGroupStyle {
    fn default() -> Self {
        Self {
            gap: 4.0,
            axis: Axis::Vertical,
            height: None,
            pad: 16.0,
            content_placement: ContentPlacement::Top,
            toggles_off_in_single_mode: false,
            mandatory_in_multiple_mode: true,
            radius: 8.0,
            border_width: 1.0,
            background: Color::rgb(0xF5, 0xF5, 0xF5),
            foreground: Color::rgb(0x1C, 0x1B, 0x1F),
            border_color: Color::rgb(0x79, 0x79, 0x7E),
            selected_background: Color::rgb(0xCF, 0xD8, 0xF7),
            selected_foreground: Color::rgb(0x00, 0x2A, 0x77),
            selected_border_color: Color::rgb(0x41, 0x4F, 0x7B),
        }
    }
}

/// Where a row's content sits inside the row's box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContentPlacement {
    /// At the top of the content box, under the padding.
    ///
    /// v1's item row: the text sat directly under the top padding, and the row
    /// grew to fit. This is the default because it is what a list of items wants.
    #[default]
    Top,
    /// Centred in the content box.
    ///
    /// v1's chip: the label was centred in the chip. The padding is symmetric,
    /// so a label centred in the content box is centred in the whole chip too.
    Centred,
}

/// One row's content.
struct SelectRow {
    /// What assistive technology calls the row. v1 had no semantics at all for
    /// these rows — see [`SelectGroup`] on why the port adds them.
    label: Option<String>,
    /// The row's content, built knowing whether *this* row is selected.
    ///
    /// The flag is the point: v1 coloured a chosen row's **ink** as well as its
    /// fill, and 2.0's `Surface` paints a fill while only the child knows it is
    /// text. A content builder that could not see the selection could not
    /// follow it, so a kit whose ink changes with the selection had no way to
    /// say so.
    content: Arc<dyn Fn(bool) -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>,
}

/// A stack of rows, any number of which can be selected.
///
/// This is the **selectable** shape the floom-ui kits all share, and the third
/// promoted widget for the same reason [`Accordion`] was: `md-item_group`,
/// `cupertino-item_group` and `fluent-item_group` are 209, 208 and 208 lines with
/// **123 of 149** substantive lines identical — only the style struct's field
/// names and the `design_system` string differ. Porting three times would be three
/// chances to get one state machine wrong.
///
/// ## Where the state lives, and why it is shared
///
/// The click lands on a [`Pressable`] inside a row, while the answer — which rows
/// are selected — is needed by *every* row, because selecting one changes the fill
/// and outline of the others as well. No row owns the group's node. So the
/// selection is a shared `Arc<Mutex<Vec<bool>>>`, exactly as [`Accordion`] shares
/// its open set and [`Tabs`] shares its selection, and every row reads it in its
/// own `update` and marks itself. The host visits every node each frame, so no
/// notification has to travel.
///
/// ## Two things this adds that v1 did not have
///
/// * **Semantics.** v1's rows were a `Container` with a click handler and a
///   pointer cursor: invisible to a screen reader, which could neither name a row
///   nor learn which was chosen. A row here is a [`Pressable`] with
///   [`SemanticsRole::Radio`] in single mode and [`SemanticsRole::CheckBox`] in
///   multiple mode, reading its own selected state through
///   [`Pressable::selected_from`]. That is a deliberate addition rather than a
///   translation, and it is called out in the porting document for that reason.
/// * **A selected row reports its state by reader, not by value.** A row is not
///   rebuilt when the selection moves, so a captured value would go stale the
///   instant another row was chosen — and would be the *opposite* of what the
///   user sees.
///
/// ## What this cannot do
///
/// v1 hid the whole group when it had no rows (`apply_if(n == 0, |s| s.hide())`).
/// 2.0 has no `hide()`: [`LayoutRequest`] carries no visibility, and a hidden
/// node and an absent one differ in layout — one takes no space, the other takes
/// whatever the parent gives it. An empty group here is a stack with no children.
pub struct SelectGroup {
    rows: Vec<SelectRow>,
    selected: Arc<Mutex<Vec<bool>>>,
    mode: SelectionMode,
    mandatory: bool,
    style: SelectGroupStyle,
    on_change: Option<Arc<dyn Fn(Vec<usize>) + Send + Sync>>,
}

impl SelectGroup {
    pub fn new() -> Self {
        Self {
            rows: Vec::new(),
            selected: Arc::new(Mutex::new(Vec::new())),
            mode: SelectionMode::Single,
            mandatory: false,
            style: SelectGroupStyle::default(),
            on_change: None,
        }
    }

    /// Append a row.
    pub fn row(
        mut self,
        label: impl Into<String>,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(SelectRow {
            label: Some(label.into()),
            content: Arc::new(move |_| content()),
        });
        self
    }

    /// Append a row whose content knows whether its row is selected.
    ///
    /// The flag is the point. v1 coloured a chosen row's **ink** as well as its
    /// fill, and 2.0's `Surface` paints a fill while only the child knows it is
    /// text — so a kit whose text changes with the selection needs the content
    /// builder to be able to see it. Use [`Self::row`] when the content does not
    /// change.
    pub fn row_with_state(
        mut self,
        label: impl Into<String>,
        content: impl Fn(bool) -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(SelectRow {
            label: Some(label.into()),
            content: Arc::new(content),
        });
        self
    }

    /// Append a row with no accessible name — one whose content speaks for itself.
    pub fn row_unlabelled(
        mut self,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(SelectRow {
            label: None,
            content: Arc::new(move |_| content()),
        });
        self
    }

    pub fn mode(mut self, mode: SelectionMode) -> Self {
        self.mode = mode;
        self
    }

    /// Several rows may be chosen.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.mode = if multiple {
            SelectionMode::Multiple
        } else {
            SelectionMode::Single
        };
        self
    }

    /// The selection may not be emptied: in single mode the chosen row cannot be
    /// un-chosen, and in multiple mode the last chosen row cannot be un-chosen.
    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.mandatory = mandatory;
        self
    }

    pub fn style(mut self, style: SelectGroupStyle) -> Self {
        self.style = style;
        self
    }

    /// The style as declared — the token-to-colour translation, readable back.
    pub fn declared_style(&self) -> SelectGroupStyle {
        self.style
    }

    /// Called with the selected indices, in order, after every change.
    pub fn on_change(mut self, callback: impl Fn(Vec<usize>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// Choose a row before mount.
    pub fn selected(self, indices: impl IntoIterator<Item = usize>) -> Self {
        for index in indices {
            if self
                .selected
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len()
                <= index
            {
                self.selected
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .resize(index + 1, false);
            }
            self.selected.lock().unwrap_or_else(|e| e.into_inner())[index] = true;
        }
        self
    }

    /// The colour a row's *content* should be drawn in.
    ///
    /// The third component in a row where v1 wrote `.color(fg)` on the box: 2.0's
    /// `Surface` paints a fill, and only the child knows it is text. So the ink is
    /// a question for the caller rather than a property of the decoration, and the
    /// pair is reported together so a caller cannot style a selected row's box and
    /// leave its label in the unselected ink.
    pub fn foreground(&self, selected: bool) -> Color {
        if selected {
            self.style.selected_foreground
        } else {
            self.style.foreground
        }
    }

    /// The selection as indices — the shape a caller reports or compares.
    pub fn selection(&self) -> Vec<usize> {
        is_selected_indices(&self.selected)
    }

    /// Whether one row is chosen, for a caller mirroring the state.
    pub fn is_selected(&self, index: usize) -> bool {
        is_selected(&self.selected, index)
    }

    /// How many rows may be chosen, as declared.
    pub fn selection_mode(&self) -> SelectionMode {
        self.mode
    }

    /// Whether the selection may be emptied, as declared.
    pub fn is_mandatory(&self) -> bool {
        self.mandatory
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        // A `Group` with no label rather than an invented one: v1 had no group
        // name, and an empty string is not a name.
        SemanticsHint {
            role: SemanticsRole::Group,
            label: None,
            state: Default::default(),
        }
    }
}

impl Default for SelectGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SelectGroup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty, for the reason [`Accordion::update`] gives: every row
    /// reads the shared selection and reports its own change, so marking dirty here
    /// as well would be a second report of a change already reported.
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(self.rows.len());
        for (index, row) in self.rows.iter().enumerate() {
            let selected = self.selected.clone();
            let label = row.label.clone();
            let content = row.content.clone();
            let mode = self.mode;
            let mandatory = self.mandatory;
            let style = self.style;
            let on_change = self.on_change.clone();

            let toggle = Arc::new(move || {
                // The rules live in `SelectGroup::choose`, reached through a
                // closure that owns the same four values a row does. A row cannot
                // call `&self`: it is not the group.
                let changed = apply_choose(
                    &selected,
                    index,
                    mode,
                    mandatory,
                    style.toggles_off_in_single_mode,
                    style.mandatory_in_multiple_mode,
                );
                if changed {
                    if let Some(callback) = &on_change {
                        callback(is_selected_indices(&selected));
                    }
                }
            }) as Arc<dyn Fn() + Send + Sync + 'static>;

            let leaf = SelectGroupRow {
                index,
                selected: self.selected.clone(),
                last_selected: false,
                mode,
                label,
                style,
                content,
                toggle,
            };
            let height = style.height;
            out.push(Child::new(
                ChildKey::index(index),
                match height {
                    Some(height) => LayoutRequest::new().height(height),
                    None => LayoutRequest::new(),
                },
                move || Box::new(leaf.clone()),
            ));
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(self.style.axis, self.style.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}

/// One row of a [`SelectGroup`]: the click target, the decorated box, and the
/// child inside it.
#[derive(Clone)]
pub(crate) struct SelectGroupRow {
    index: usize,
    selected: Arc<Mutex<Vec<bool>>>,
    /// The value this row last acted on, so a change is a change rather than a
    /// fresh read every frame.
    last_selected: bool,
    mode: SelectionMode,
    label: Option<String>,
    style: SelectGroupStyle,
    content: Arc<dyn Fn(bool) -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>,
    toggle: Arc<dyn Fn() + Send + Sync>,
}

impl Widget for SelectGroupRow {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = is_selected(&self.selected, self.index);
        if now == self.last_selected {
            return;
        }
        self.last_selected = now;
        // The fill and the outline are this row's own decoration, so this row is
        // what has to invalidate them. A dirty mark and nothing else: the box does
        // not move, so there is no layout to redo, and `mark_layout_dirty` here
        // would re-lay out the group for a change of colour.
        cx.mark_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            {
                let index = self.index;
                let selected = self.selected.clone();
                let label = self.label.clone();
                let content = self.content.clone();
                let style = self.style;
                let mode = self.mode;
                let toggle = self.toggle.clone();
                move || {
                    let role = match mode {
                        SelectionMode::Single => SemanticsRole::Radio,
                        SelectionMode::Multiple => SemanticsRole::CheckBox,
                    };
                    let selected_flag = selected.clone();
                    let surface =
                        row_surface(style, is_selected(&selected_flag, index), content.clone());
                    let mut pressable = floom_widgets::Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(surface.clone()),
                    ))
                    .role(role)
                    .selected_from(move || is_selected(&selected_flag, index));
                    // Cloned, not moved: this closure is replayed, so the label has to survive
                    // being handed to a rebuilt `Pressable`.
                    if let Some(label) = label.clone() {
                        pressable = pressable.label(label);
                    }
                    // The toggle is cloned into the click closure, not moved: this slot is
                    // replayed, so a rebuilt row has to be able to hand out its own handle.
                    let click = toggle.clone();
                    Box::new(pressable.on_click(move || click()))
                }
            },
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(self.style.axis, self.style.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(SemanticsHint::decorative(), cx);
    }
}

/// The decorated box of one row, in the appearance its selection calls for.
fn row_surface(
    style: SelectGroupStyle,
    selected: bool,
    content: Arc<dyn Fn(bool) -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>,
) -> floom_widgets::Surface {
    let (background, border) = if selected {
        (style.selected_background, style.selected_border_color)
    } else {
        (style.background, style.border_color)
    };
    let mut surface = floom_widgets::Surface::new()
        .background(background)
        .radius(style.radius)
        // v1: `padding_horiz(pad).padding_vert(pad / 2.0)`
        .padding(Insets {
            left: f64::from(style.pad),
            right: f64::from(style.pad),
            top: f64::from(style.pad) / 2.0,
            bottom: f64::from(style.pad) / 2.0,
        });
    if style.border_width > 0.0 {
        surface = surface.border(style.border_width, border);
    }
    // A chip centres its label; a list of items puts its text under the top
    // padding. The padding is symmetric, so a label centred in the content box
    // is centred in the whole chip -- which is what v1's `items_center` did.
    if style.content_placement == ContentPlacement::Centred {
        surface = surface.request(LayoutRequest::new().cross_align(Align::Center));
    }
    // The content is built with the row's own selection, so an ink that changes
    // with the selection can change.
    surface.push(move || content(selected))
}

/// `SelectGroup::choose`, as a free function over the shared state.
fn apply_choose(
    selected: &Mutex<Vec<bool>>,
    index: usize,
    mode: SelectionMode,
    mandatory: bool,
    toggles_off_in_single_mode: bool,
    mandatory_in_multiple_mode: bool,
) -> bool {
    let mut guard = selected.lock().unwrap_or_else(|e| e.into_inner());
    if guard.len() <= index {
        guard.resize(index + 1, false);
    }
    let was = guard[index];
    match mode {
        SelectionMode::Single => {
            if mandatory && was {
                return false;
            }
            // A chip can be tapped off; a radio cannot.
            if toggles_off_in_single_mode && was {
                guard[index] = false;
                return true;
            }
            for slot in guard.iter_mut() {
                *slot = false;
            }
            guard[index] = true;
            true
        }
        SelectionMode::Multiple => {
            if was {
                let chosen = guard.iter().filter(|s| **s).count();
                if mandatory_in_multiple_mode && mandatory && chosen <= 1 {
                    return false;
                }
                guard[index] = false;
            } else {
                guard[index] = true;
            }
            true
        }
    }
}

fn is_selected(selected: &Mutex<Vec<bool>>, index: usize) -> bool {
    selected
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(index)
        .copied()
        .unwrap_or(false)
}

fn is_selected_indices(selected: &Mutex<Vec<bool>>) -> Vec<usize> {
    selected
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .enumerate()
        .filter(|(_, chosen)| **chosen)
        .map(|(index, _)| index)
        .collect()
}

#[cfg(test)]
mod select_group_tests {
    //! The **selectable** reference widget.
    //!
    //! Where `accordion_tests` pins a value that shows and hides a subtree, this
    //! pins one that *restyles* the rows around it: choosing a row changes the fill
    //! and outline of every row, not only the chosen one. The three things that make
    //! that hard are each a test here.
    //!
    //! The tests stand in for the host, for the reason `accordion_tests` gives: the
    //! update pass visits every node every frame and dirtiness is *reported by*
    //! `update()`. So what is checked is that each row reports its own change.

    use floom_accessibility::SemanticsState;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use floom_core::{Edges, Point, PointerEvent, Widget};
    use floom_types::Rect;

    use super::*;
    use floom_widgets::Text;
    use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext, TestEventContext};

    const ROW: Rect = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: 48.0,
    };
    const AT: Point = Point { x: 20.0, y: 20.0 };

    fn three_rows() -> SelectGroup {
        let mut group = SelectGroup::new();
        for label in ["One", "Two", "Three"] {
            group = group.row(label, || Box::new(Text::new("row")));
        }
        group
    }

    /// Mounts row `index` the way the reconciler would.
    fn mount_row(group: &SelectGroup, index: usize) -> SelectGroupRow {
        let child = group.children()[index].clone();
        let mut widget = child.build();
        widget
            .as_any_mut()
            .downcast_mut::<SelectGroupRow>()
            .expect("a row slot mounts a row")
            .clone()
    }

    fn tick(row: &mut SelectGroupRow) -> RecordingUpdateContext {
        let mut create = AllocatingCreateContext::new();
        let id = row.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        row.update(&mut cx, id);
        cx
    }

    /// Clicks row `index` through the real dispatch path: a press and a release,
    /// both consumed by that row's own `Pressable`.
    fn click_row(group: &SelectGroup, index: usize) {
        let mut mounted = group.children()[index].build().children().remove(0).build();
        let pressable = mounted
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("a row's slot mounts a pressable");
        let mut pressable = std::mem::replace(
            pressable,
            Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                Box::new(Text::new(""))
            })),
        );
        let mut create = AllocatingCreateContext::new();
        let id = pressable.create(&mut create);
        for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
            let mut cx = TestEventContext::new(id, ROW, phase.into());
            pressable.event(&mut cx, id);
        }
    }

    // --- structure ---------------------------------------------------------

    #[test]
    fn every_declared_row_gets_one_slot() {
        let group = three_rows();
        let children = group.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].key, ChildKey::index(0));
        assert_eq!(children[2].key, ChildKey::index(2));
    }

    #[test]
    fn the_stack_declares_its_axis_and_the_token_gap() {
        let group = three_rows().style(SelectGroupStyle {
            gap: 7.0,
            ..SelectGroupStyle::default()
        });
        let req = group.layout_request().expect("the group is a stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(req.gap, Some(7.0));
    }

    /// The mutation this catches: an empty group whose `children()` is wrong would
    /// mount nothing, and v1's `apply_if(n == 0, |s| s.hide())` hid the *node*.
    /// 2.0 has no `hide()` — see `SelectGroup`'s doc — so an empty group is a
    /// stack with no children, and this says so rather than leaving it implied.
    #[test]
    fn an_empty_group_is_a_stack_with_no_rows() {
        let group = SelectGroup::new();
        assert!(group.children().is_empty());
        assert_eq!(
            group.layout_request().and_then(|r| r.axis),
            Some(Axis::Vertical),
            "and it is still a stack"
        );
    }

    // --- the click rules ---------------------------------------------------

    #[test]
    fn clicking_a_row_selects_it() {
        let group = three_rows();
        assert!(
            group.selection().is_empty(),
            "nothing is chosen to begin with"
        );
        click_row(&group, 1);
        assert_eq!(group.selection(), vec![1]);
        assert!(group.is_selected(1));
        assert!(!group.is_selected(0));
    }

    /// The control for the above, and the reason `apply_choose` clears the rest: if
    /// choosing did *not* clear, the two tests would agree with each other and
    /// nothing would notice.
    #[test]
    fn choosing_a_second_row_replaces_the_first_in_single_mode() {
        let group = three_rows();
        click_row(&group, 0);
        click_row(&group, 2);
        assert_eq!(group.selection(), vec![2], "not [0, 2]");
    }

    #[test]
    fn multiple_mode_accumulates_and_toggles() {
        let group = three_rows().multiple(true);
        click_row(&group, 0);
        click_row(&group, 2);
        assert_eq!(group.selection(), vec![0, 2]);
        click_row(&group, 0);
        assert_eq!(
            group.selection(),
            vec![2],
            "clicking a chosen row un-chooses it"
        );
    }

    /// v1's single-mode mandatory rule: the chosen row cannot be un-chosen.
    #[test]
    fn mandatory_single_mode_refuses_to_empty_the_selection() {
        let group = three_rows().mandatory(true);
        click_row(&group, 1);
        assert_eq!(group.selection(), vec![1]);
        click_row(&group, 1);
        assert_eq!(
            group.selection(),
            vec![1],
            "clicking the chosen row again is a no-op"
        );
        click_row(&group, 2);
        assert_eq!(group.selection(), vec![2], "but another row still takes it");
    }

    /// v1's multiple-mode mandatory rule, which is narrower than the name suggests
    /// and narrower than this test first asserted: `mandatory` refuses to remove a
    /// row only when it is the *only* chosen one (`cur.len() == 1` in v1). With two
    /// chosen, clicking either one removes it — mandatory keeps the set non-empty,
    /// it does not freeze it.
    ///
    /// Recorded because getting this backwards is invisible until a user tries to
    /// deselect one of two chosen rows and nothing happens.
    #[test]
    fn mandatory_multiple_mode_keeps_the_selection_non_empty() {
        let group = three_rows().multiple(true).mandatory(true);
        click_row(&group, 0);
        click_row(&group, 1);
        assert_eq!(group.selection(), vec![0, 1]);

        click_row(&group, 1);
        assert_eq!(
            group.selection(),
            vec![0],
            "one of two may go: mandatory is not a freeze"
        );

        click_row(&group, 0);
        assert_eq!(group.selection(), vec![0], "and the last one may not");

        click_row(&group, 2);
        assert_eq!(group.selection(), vec![0, 2], "adding still works");
    }

    #[test]
    fn a_row_chosen_before_mount_starts_selected() {
        let group = three_rows().selected([1]);
        assert_eq!(group.selection(), vec![1]);
        // And it is *reported*, not merely stored.
        let mut row = mount_row(&group, 1);
        let cx = tick(&mut row);
        assert_eq!(
            row.last_selected, true,
            "the row noticed it was already chosen"
        );
        let _ = cx;
    }

    // --- the report --------------------------------------------------------

    #[test]
    fn a_change_reports_the_new_selection() {
        let seen: Arc<Mutex<Vec<Vec<usize>>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let group = three_rows().multiple(true).on_change(move |next| {
            sink.lock().unwrap().push(next);
        });
        click_row(&group, 0);
        click_row(&group, 2);
        assert_eq!(
            *seen.lock().unwrap(),
            vec![vec![0], vec![0, 2]],
            "one report per change, in order"
        );
    }

    /// A refused change must not report: v1 called its callback only when it had
    /// actually changed something, and a consumer counting notifications would
    /// otherwise see a phantom event for a no-op click.
    #[test]
    fn a_refused_change_reports_nothing() {
        let count = Arc::new(AtomicUsize::new(0));
        let sink = count.clone();
        let group = three_rows().mandatory(true).on_change(move |_| {
            sink.fetch_add(1, Ordering::SeqCst);
        });
        click_row(&group, 1);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        click_row(&group, 1);
        assert_eq!(
            count.load(Ordering::SeqCst),
            1,
            "the second click was refused, so nothing was reported"
        );
    }

    // --- the shared state, which is the whole design -----------------------

    /// The load-bearing claim. A click lands inside one row, but the answer is
    /// needed by every row; if the state were per-row, choosing row 1 would leave
    /// row 0 believing it was still chosen.
    #[test]
    fn choosing_one_row_is_visible_to_the_others() {
        let group = three_rows();
        click_row(&group, 1);
        for index in 0..3 {
            assert_eq!(
                group.is_selected(index),
                index == 1,
                "row {index} reads the shared selection"
            );
        }
    }

    /// The mutation this catches: a row whose `update` compares against nothing
    /// and so never notices. Every other test here drives the *group's* accessor,
    /// which would keep passing.
    #[test]
    fn a_row_reports_its_own_change_rather_than_being_told() {
        let group = three_rows();
        let mut row = mount_row(&group, 0);
        assert_eq!(row.last_selected, false, "unselected to begin with");

        // A first tick with nothing changed reports nothing.
        let quiet = tick(&mut row);
        assert!(
            quiet.dirty_count() == 0,
            "an unselected row that stays unselected says nothing"
        );

        click_row(&group, 0);
        let loud = tick(&mut row);
        assert!(loud.dirty_count() > 0, "and this row noticed by itself");
        assert_eq!(row.last_selected, true);

        // And settles: a second tick after the change is quiet again.
        let settled = tick(&mut row);
        assert!(
            settled.dirty_count() == 0,
            "a change is reported once, not every frame"
        );
    }

    #[test]
    fn a_row_that_loses_the_selection_reports_too() {
        let group = three_rows().selected([0]);
        let mut row = mount_row(&group, 0);
        tick(&mut row);
        assert_eq!(row.last_selected, true);

        click_row(&group, 1);
        let cx = tick(&mut row);
        assert!(
            cx.dirty_count() > 0,
            "losing the selection is a change worth reporting"
        );
        assert_eq!(row.last_selected, false);
    }

    // --- the two appearances ----------------------------------------------

    /// The decoration each row draws is the other half of "the state is shared":
    /// the row that changed appearance is the row that reports.
    #[test]
    fn a_row_paints_the_selected_appearance_when_selected() {
        let style = SelectGroupStyle::default();
        let content: Arc<dyn Fn(bool) -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static> =
            Arc::new(|_| Box::new(Text::new("row")));

        let unselected = row_surface(style, false, content.clone());
        let selected = row_surface(style, true, content);

        let fill_of = |surface: &floom_widgets::Surface| {
            surface
                .decoration()
                .background_color()
                .expect("a row is filled")
        };
        assert_eq!(
            fill_of(&unselected),
            style.background,
            "the unselected fill"
        );
        assert_eq!(fill_of(&selected), style.selected_background);
        assert_ne!(
            fill_of(&unselected),
            fill_of(&selected),
            "and the two are different, so the change is visible at all"
        );
    }

    #[test]
    fn a_row_outlines_itself_at_the_style_width() {
        let style = SelectGroupStyle {
            border_width: 2.0,
            ..SelectGroupStyle::default()
        };
        let surface = row_surface(style, false, Arc::new(|_| Box::new(Text::new("row"))));
        let border = surface
            .decoration()
            .border_spec()
            .expect("v1 wrote `.border(1.0)` unconditionally");
        assert_eq!(border.width(), 2.0);
        assert_eq!(border.color(), style.border_color);
        assert_eq!(border.edges(), Edges::ALL, "v1 outlined all four sides");
    }

    #[test]
    fn a_zero_border_width_draws_none() {
        let style = SelectGroupStyle {
            border_width: 0.0,
            ..SelectGroupStyle::default()
        };
        let surface = row_surface(style, false, Arc::new(|_| Box::new(Text::new("row"))));
        assert!(surface.decoration().border_spec().is_none());
    }

    /// v1 wrote `padding_horiz(pad).padding_vert(pad / 2.0)`, which is not a
    /// uniform inset and is the kind of asymmetry a "just use `uniform`" cleanup
    /// would quietly lose.
    #[test]
    fn the_row_padding_is_horizontal_pad_and_vertical_half() {
        let style = SelectGroupStyle {
            pad: 16.0,
            ..SelectGroupStyle::default()
        };
        let surface = row_surface(style, false, Arc::new(|_| Box::new(Text::new("row"))));
        let declared = surface.layout_request().expect("a padded surface");
        assert_eq!(
            declared.padding,
            Some(floom_core::Insets {
                left: 16.0,
                right: 16.0,
                top: 8.0,
                bottom: 8.0,
            })
        );
    }

    // --- semantics ---------------------------------------------------------

    /// A deliberate *addition*, not a translation: v1's rows were a click handler
    /// and a pointer cursor, invisible to a screen reader.
    #[test]
    fn a_row_is_a_radio_in_single_mode_and_a_checkbox_in_multiple() {
        let role_of = |mode: SelectionMode| {
            let mut group = SelectGroup::new().row("One", || Box::new(Text::new("row")));
            group = match mode {
                SelectionMode::Single => group,
                SelectionMode::Multiple => group.multiple(true),
            };
            let pressable = group.children()[0].build().children().remove(0).build();
            pressable
                .as_any()
                .downcast_ref::<Pressable>()
                .expect("a pressable")
                .semantics_hint()
                .role
        };
        assert_eq!(role_of(SelectionMode::Single), SemanticsRole::Radio);
        assert_eq!(role_of(SelectionMode::Multiple), SemanticsRole::CheckBox);
    }

    /// A chip can be tapped off; a radio cannot.
    ///
    /// v1's chip group un-chose the chosen chip when it was tapped again in
    /// single mode, and v1's item group did not. The style says which.
    #[test]
    fn a_row_tapped_again_is_un_chosen_when_the_style_says_so() {
        let mut group = SelectGroup::new().row("One", || Box::new(Text::new("row")));
        group = group.style(SelectGroupStyle {
            toggles_off_in_single_mode: true,
            ..SelectGroupStyle::default()
        });
        click_row(&group, 0);
        assert_eq!(group.selection(), vec![0], "the first tap chooses it");
        click_row(&group, 0);
        assert!(
            group.selection().is_empty(),
            "and the second tap takes it off, the way a chip does"
        );
    }

    /// The control for the above: without the flag, the row stays chosen.
    #[test]
    fn a_row_tapped_again_stays_chosen_by_default() {
        let group = three_rows();
        click_row(&group, 0);
        click_row(&group, 0);
        assert_eq!(group.selection(), vec![0], "a radio does not come off");
    }

    /// v1's chip group read `mandatory` only in the single-mode branch, so
    /// several chips could all be un-chosen. v1's item group enforced it in both.
    #[test]
    fn mandatory_can_be_left_out_of_the_multiple_mode_rule() {
        let mut group = SelectGroup::new()
            .row("One", || Box::new(Text::new("row")))
            .row("Two", || Box::new(Text::new("row")));
        group = group.multiple(true).mandatory(true).style(SelectGroupStyle {
            mandatory_in_multiple_mode: false,
            ..SelectGroupStyle::default()
        });
        click_row(&group, 0);
        click_row(&group, 0);
        assert!(
            group.selection().is_empty(),
            "the last chosen chip comes off, the way v1's chip group did"
        );
    }

    /// The control for the above: by default, the last chosen row stays.
    #[test]
    fn mandatory_keeps_the_last_chosen_row_by_default() {
        let mut group = SelectGroup::new()
            .row("One", || Box::new(Text::new("row")))
            .row("Two", || Box::new(Text::new("row")));
        group = group.multiple(true).mandatory(true);
        click_row(&group, 0);
        click_row(&group, 0);
        assert_eq!(group.selection(), vec![0], "the last one stays chosen");
    }

    /// A chip centres its label; a list of items puts its text under the top
    /// padding. The padding is symmetric, so a label centred in the content box
    /// is centred in the whole chip -- which is what v1's `items_center` did.
    #[test]
    fn a_row_can_centre_its_content() {
        let mut group = SelectGroup::new().row("One", || Box::new(Text::new("row")));
        group = group.style(SelectGroupStyle {
            content_placement: ContentPlacement::Centred,
            ..SelectGroupStyle::default()
        });
        // The row holds a pressable, the pressable holds the surface, and the
        // surface is the box the content sits in.
        let row = group.children()[0].build().children().remove(0).build();
        let pressable = row.children().remove(0).build();
        let surface = pressable
            .as_any()
            .downcast_ref::<floom_widgets::Surface>()
            .expect("a surface");
        let request = surface.layout_request().expect("a surface asks for its own space");
        assert_eq!(request.cross_align, Some(Align::Center));
    }

    /// The rows run along the direction the style names.
    ///
    /// A group of chips runs the other way -- v1's chip group was a *row* of chips
    /// -- and the direction is the caller's answer, not the engine's.
    #[test]
    fn the_rows_run_along_the_styles_axis() {
        let mut group = SelectGroup::new().row("One", || Box::new(Text::new("row")));
        group = group.style(SelectGroupStyle {
            axis: Axis::Horizontal,
            ..SelectGroupStyle::default()
        });
        let request = group.layout_request().expect("a group asks for its own space");
        assert_eq!(request.axis, Some(Axis::Horizontal));
        assert_eq!(request.gap, Some(4.0), "and the style's gap between them");
    }

    /// A row can have a fixed height, when the rows are chips rather than items.
    #[test]
    fn a_row_can_have_a_fixed_height() {
        let mut group = SelectGroup::new().row("One", || Box::new(Text::new("row")));
        group = group.style(SelectGroupStyle {
            height: Some(32.0),
            ..SelectGroupStyle::default()
        });
        let request = group.children()[0].layout;
        assert_eq!(request.height, Some(floom_types::Length::Px(32.0)));
    }

    /// A row's content is built knowing whether *that* row is selected.
    ///
    /// v1 coloured a chosen row's **ink** as well as its fill, and 2.0's
    /// `Surface` paints a fill while only the child knows it is text -- so a kit
    /// whose text changes with the selection needs the content builder to be
    /// able to see it. `row_with_state` is that door.
    #[test]
    fn a_row_s_content_is_built_with_its_own_selection() {
        let mut group = SelectGroup::new()
            .row_with_state("One", |selected| {
                Box::new(Text::new(if selected { "on" } else { "off" }))
            })
            .row_with_state("Two", |selected| {
                Box::new(Text::new(if selected { "on" } else { "off" }))
            });
        group = group.selected([0]);

        let content_of = |index: usize| {
            // The row holds a pressable, and the pressable holds the surface, and
            // the surface holds the content.
            let row = group.children()[index].build().children().remove(0).build();
            let pressable = row.children().remove(0).build();
            let surface = pressable
                .as_any()
                .downcast_ref::<floom_widgets::Surface>()
                .expect("a surface")
                .clone();
            let text = surface.children()[0].build();
            text.as_any()
                .downcast_ref::<Text>()
                .expect("a text")
                .content()
                .to_string()
        };

        assert_eq!(content_of(0), "on", "the chosen row says so");
        assert_eq!(content_of(1), "off", "and the unchosen one does not");
    }

    /// The reader, not a captured value: a row is not rebuilt when the selection
    /// moves, so a value baked in at build time would report the opposite of what
    /// the user sees.
    #[test]
    fn a_row_reports_its_selected_state_by_reader() {
        let group = three_rows();
        let hint_of = |index: usize| {
            let mounted = group.children()[index].build().children().remove(0).build();
            mounted
                .as_any()
                .downcast_ref::<Pressable>()
                .expect("a pressable")
                .semantics_hint()
        };
        let hint = hint_of(1);
        assert!(
            !hint.state.contains(SemanticsState::SELECTED),
            "unselected says nothing: {hint:?}"
        );

        click_row(&group, 1);

        // Asked again *after* the click: the reader is read, not stored, so a
        // widget that was built before the selection moved reports the new state.
        let hint = hint_of(1);
        assert!(
            hint.state.contains(SemanticsState::SELECTED),
            "and the same widget, asked again after the click, says selected: {hint:?}"
        );
    }

    #[test]
    fn the_group_itself_is_a_labelled_nothing() {
        let hint = three_rows().semantics_hint();
        assert_eq!(hint.role, SemanticsRole::Group);
        assert!(
            hint.label.is_none(),
            "v1 had no group name, and an empty string is not a name: {hint:?}"
        );
    }
}

#[cfg(test)]
mod accordion_tests {
    //! The **stateful** reference widget.
    //!
    //! `tests/badge.rs` pins a description and `tests/controls.rs` a single
    //! interaction. This pins the part that is easy to get wrong and hard to see:
    //! a value that changes after mount, a subtree that mounts and unmounts with
    //! it, and a header that must *keep its identity* across the toggle.
    //!
    //! The tests stand in for the host, which is enough because the host's
    //! contribution here is a documented one — the update pass visits every node
    //! every frame and dirtiness is *reported by* `update()`. So what is checked
    //! is that each widget reports its own change, not that it was told.

    use floom_accessibility::SemanticsState;

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use floom_core::{Point, PointerEvent, Widget, WidgetAny};
    use floom_types::Rect;

    use super::*;
    use floom_widgets::Text;
    use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext, TestEventContext};

    const ROW: Rect = Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 300.0,
        y1: 48.0,
    };
    const AT: Point = Point { x: 20.0, y: 20.0 };

    /// One panel, with a counter that can tell "the body was *declared*" from
    /// "the body widget was *built*".
    fn one_panel() -> (Accordion, Arc<AtomicUsize>) {
        one_panel_with(false)
    }

    fn one_panel_with(multiple: bool) -> (Accordion, Arc<AtomicUsize>) {
        let builds = Arc::new(AtomicUsize::new(0));
        let sink = builds.clone();
        (
            Accordion::new()
                .multiple(multiple)
                .panel("Only", indicator(), move || {
                    sink.fetch_add(1, Ordering::SeqCst);
                    Box::new(Text::new("body"))
                }),
            builds,
        )
    }

    fn indicator() -> impl Fn(bool) -> Box<dyn WidgetAny> + Send + Sync {
        |open| Box::new(Text::new(if open { "v" } else { ">" }))
    }

    fn three_panels() -> Accordion {
        let mut accordion = Accordion::new();
        for title in ["One", "Two", "Three"] {
            accordion = accordion.panel(title, indicator(), || Box::new(Text::new("body")));
        }
        accordion
    }

    /// Mounts panel `index` the way the reconciler would.
    fn mount_panel(accordion: &Accordion, index: usize) -> AccordionPanelLeaf {
        let child = accordion.children()[index].clone();
        let mut widget = child.build();
        widget
            .as_any_mut()
            .downcast_mut::<AccordionPanelLeaf>()
            .expect("a panel slot mounts a panel leaf")
            .clone()
    }

    fn tick(leaf: &mut AccordionPanelLeaf) -> RecordingUpdateContext {
        let mut create = AllocatingCreateContext::new();
        let id = leaf.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        leaf.update(&mut cx, id);
        cx
    }

    /// Clicks panel `header` through the real dispatch path: a press and then a
    /// release, both consumed by the header's own `Pressable`.
    fn click_header(accordion: &Accordion, panel: usize) {
        let mut header = accordion.children()[panel]
            .build()
            .children()
            .remove(0)
            .build();
        let pressable = header
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("the header slot mounts a pressable");
        let mut create = AllocatingCreateContext::new();
        let id = pressable.create(&mut create);
        for phase in [PointerEvent::primary_down(AT), PointerEvent::primary_up(AT)] {
            let mut cx = TestEventContext::new(id, ROW, phase.into());
            pressable.event(&mut cx, id);
        }
    }

    // --- structure ---------------------------------------------------------

    #[test]
    fn every_declared_panel_gets_one_slot() {
        let accordion = three_panels();
        let children = accordion.children();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].key, ChildKey::index(0));
        assert_eq!(children[2].key, ChildKey::index(2));
        assert_eq!(accordion.titles(), vec!["One", "Two", "Three"]);
    }

    #[test]
    fn the_stack_declares_its_axis_and_the_token_gap() {
        let accordion = three_panels().style(AccordionStyle {
            gap: 7.0,
            ..AccordionStyle::default()
        });
        let req = accordion
            .layout_request()
            .expect("the accordion is a stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(req.gap, Some(7.0));
    }

    #[test]
    fn a_collapsed_panel_mounts_only_its_header() {
        let (accordion, _) = one_panel();
        let mut leaf = mount_panel(&accordion, 0);
        assert_eq!(
            leaf.children().len(),
            1,
            "header only, before the panel has ever been opened"
        );
        let cx = tick(&mut leaf);
        assert!(
            !cx.reconciled(),
            "a panel standing still must not ask to be reconciled"
        );
    }

    #[test]
    fn opening_a_panel_reconciles_and_builds_its_content() {
        let (mut accordion, builds) = one_panel();
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);

        let cx = tick(&mut leaf);

        assert!(
            cx.reconciled(),
            "the body's mount state changed, so the subtree has to be re-declared"
        );
        assert_eq!(leaf.children().len(), 2, "header plus the revealed body");
        assert_eq!(
            builds.load(Ordering::SeqCst),
            0,
            "declaring a slot does not build it"
        );
        let body = leaf.children()[1].build();
        assert_eq!(builds.load(Ordering::SeqCst), 1, "mounting does");
        assert!(body.as_any().is::<Text>());
    }

    #[test]
    fn closing_a_panel_unmounts_its_content_again() {
        let (mut accordion, _) = one_panel();
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);
        tick(&mut leaf);
        assert_eq!(leaf.children().len(), 2);

        accordion.set_open(0, false);
        tick(&mut leaf);

        assert_eq!(
            leaf.children().len(),
            1,
            "the body is gone once the panel is closed"
        );
    }

    // --- the state machine -------------------------------------------------

    #[test]
    fn every_panel_starts_collapsed() {
        assert_eq!(three_panels().open_state(), vec![false, false, false]);
    }

    #[test]
    fn multiple_panels_may_be_open_together() {
        let mut accordion = three_panels().multiple(true);
        accordion.set_open(0, true);
        accordion.set_open(2, true);
        assert_eq!(
            accordion.open_state(),
            vec![true, false, true],
            "two open at once, the middle one shut"
        );
    }

    /// The imperative door applies the same rule a click does — it is the one
    /// place a caller could otherwise reach a state the UI cannot.
    #[test]
    fn the_imperative_door_applies_the_single_panel_rule_too() {
        let mut accordion = three_panels();
        accordion.set_open(0, true);
        accordion.set_open(1, true);
        assert_eq!(accordion.open_state(), vec![false, true, false]);
    }

    /// A programmatic change does **not** fire `on_change`: that callback reports
    /// what a person did, and a caller that just set the state knows already.
    #[test]
    fn the_imperative_door_does_not_fire_on_change() {
        let changes = Arc::new(AtomicUsize::new(0));
        let sink = changes.clone();
        let (mut accordion, _) = one_panel();
        accordion = accordion.on_change(move |_, _| {
            sink.fetch_add(1, Ordering::SeqCst);
        });

        accordion.set_open(0, true);
        assert_eq!(changes.load(Ordering::SeqCst), 0);
    }

    // --- clicking ----------------------------------------------------------

    #[test]
    fn clicking_a_header_opens_that_panel_and_reports_it() {
        let seen: Arc<Mutex<Vec<(usize, bool)>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = seen.clone();
        let (mut accordion, _) = one_panel();
        accordion = accordion.on_change(move |index, open| {
            sink.lock().unwrap().push((index, open));
        });

        click_header(&accordion, 0);

        assert!(accordion.is_open(0), "the click reached the shared state");
        assert_eq!(
            *seen.lock().unwrap(),
            vec![(0, true)],
            "and the callback reported which panel moved which way"
        );
    }

    #[test]
    fn clicking_the_open_panel_closes_it_again() {
        let (mut accordion, _) = one_panel();
        accordion.set_open(0, true);
        click_header(&accordion, 0);
        assert!(!accordion.is_open(0));
    }

    #[test]
    fn a_click_does_not_dirty_the_accordion_itself() {
        let mut accordion = one_panel().0;
        let mut create = AllocatingCreateContext::new();
        let id = accordion.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        accordion.update(&mut cx, id);
        assert_eq!(
            cx.dirty_count(),
            0,
            "the accordion owns the same panel slots either way; each panel \
             reports its own change, so a report here would be redundant work"
        );
    }

    // --- the header survives its own toggle --------------------------------

    /// The header must not be rebuilt when the panel opens: a rebuilt header is a
    /// new node, so the keyboard focus the press requested is gone — thrown away
    /// by the very act of activating it.
    #[test]
    fn the_header_slot_key_does_not_depend_on_the_open_flag() {
        let (mut accordion, _) = one_panel();
        let closed = mount_panel(&accordion, 0).children()[0].clone();

        accordion.set_open(0, true);
        let open = mount_panel(&accordion, 0).children()[0].clone();

        assert_eq!(
            closed.key, open.key,
            "the reconciler reuses a child only when the key still matches"
        );
        assert_eq!(closed.key, ChildKey::index(0));
    }

    /// A header that is not rebuilt cannot know a value it captured at build
    /// time, so its reported state has to be read from the shared flag. Without
    /// this the header would announce "collapsed" on a visibly open panel.
    #[test]
    fn a_header_built_before_the_toggle_still_reports_the_new_state() {
        let (mut accordion, _) = one_panel();
        let mut header = mount_panel(&accordion, 0).children()[0].clone().build();
        let pressable = header
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("the header is a pressable");

        assert!(
            pressable
                .semantics_hint()
                .state
                .contains(SemanticsState::COLLAPSED)
        );

        accordion.set_open(0, true);

        assert!(
            pressable
                .semantics_hint()
                .state
                .contains(SemanticsState::EXPANDED),
            "the same widget instance now reports what the user can see"
        );
    }

    /// The indicator is a widget that re-declares its own child, because a bare
    /// `Child` cannot change: the host only rebuilds a child whose key changed,
    /// and everything inside a stable header has a stable key.
    ///
    /// Reaching it is the argument in miniature — the header is a `Pressable`,
    /// its child is the row `Stack`, and the indicator is the row's third slot.
    #[test]
    fn the_indicator_rebuilds_its_glyph_when_the_panel_opens() {
        let (mut accordion, _) = one_panel();
        let leaf = mount_panel(&accordion, 0);
        let mut header = leaf.children().remove(0).build();
        let pressable = header
            .as_any_mut()
            .downcast_mut::<Pressable>()
            .expect("the header slot mounts a pressable");
        let mut row = pressable.children().remove(0).build();
        let row = row
            .as_any_mut()
            .downcast_mut::<Stack>()
            .expect("the pressable's child is the row")
            .clone();
        assert_eq!(row.children().len(), 3, "title, spacer, indicator");
        let mut indicator_child = row.children().remove(2).build();
        let slot = indicator_child
            .as_any_mut()
            .downcast_mut::<AccordionIndicatorSlot>()
            .expect("the third slot is the indicator");

        let collapsed = slot.children()[0].clone();
        let mut create = AllocatingCreateContext::new();
        let id = slot.create(&mut create);
        let mut cx = RecordingUpdateContext::new();
        slot.update(&mut cx, id);
        assert!(!cx.reconciled(), "closed is where it started");

        accordion.set_open(0, true);
        let mut cx = RecordingUpdateContext::new();
        slot.update(&mut cx, id);

        assert!(
            cx.reconciled(),
            "a different glyph is a different child, so the slot has to \
             re-declare it"
        );
        let opened = slot.children()[0].clone();
        assert_ne!(
            collapsed.key, opened.key,
            "and the key carries the flag, so the host mounts rather than reuses"
        );
    }

    // --- what cannot be animated -------------------------------------------

    /// Without a transition the reveal is instant, matching v1's `dyn_view`.
    #[test]
    fn without_a_transition_nothing_is_declared_while_shown() {
        let (mut accordion, _) = one_panel();
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);
        tick(&mut leaf);
        assert!(leaf.animation().is_none(), "a mount is not a motion");
    }

    /// With one, the *panel's own node* declares it — which is why the leaf holds
    /// the reveal as a value instead of mounting it as a node of its own.
    #[test]
    fn a_content_transition_is_declared_on_the_panel_node() {
        let (mut accordion, _) = one_panel();
        accordion = accordion.content_transition(PresenceTransition::fade(0.2));
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);
        tick(&mut leaf);

        let spec = leaf.animation().expect("the reveal declares its motion");
        assert!(
            spec.duration_secs > 0.0,
            "and it is the caller's duration, not a default"
        );
    }

    /// The control for the whole point: **2.0 cannot animate the reveal's
    /// height.** v1's `md-expand_transition` tweened `height: 0 -> computed`,
    /// which needs a re-layout every frame, and the host only lays out when
    /// layout is dirty (`WidgetHost::frame` -> `if self.layout_dirty`). So the
    /// panel's box is its final height from the first frame and the transition
    /// can only be compositor-only.
    #[test]
    fn the_reveal_does_not_resize_the_panel_over_time() {
        let (mut accordion, _) = one_panel();
        accordion = accordion.content_transition(PresenceTransition::fade(0.2));
        accordion.set_open(0, true);
        let mut leaf = mount_panel(&accordion, 0);

        // Mid-flight: the transition is declared and the body is mounted...
        tick(&mut leaf);
        assert!(leaf.animation().is_some());
        assert_eq!(leaf.children().len(), 2);

        // ...while the panel's own layout request is unchanged, because nothing
        // here scales a box by the animation. A height reveal would have to.
        let req = leaf.layout_request().expect("a vertical stack");
        assert_eq!(req.axis, Some(Axis::Vertical));
        assert_eq!(
            req.height, None,
            "the panel claims no height of its own, so the host has nothing to \
             re-derive per frame — which is exactly why this is unavailable"
        );
    }
}
