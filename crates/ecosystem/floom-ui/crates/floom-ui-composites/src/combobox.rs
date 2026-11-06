use std::sync::{Arc, Mutex};

use floom_core::{
    Align, Axis, Child, ChildKey, Color, CreateContext, ElementId, LayoutRequest, OverlayRequest,
    PaintContext, SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_types::OverlayKind;

use crate::{popup_placement_rect, PopupPlacement};
use floom_widgets::text_input::SharedBuffer;

/// The selection state: which option is chosen, and whether the list is up.
///
/// Separate from the query because the query is a [`SharedBuffer`] — it belongs to a
/// [`TextInput`](crate::TextInput) — and because the two change for different reasons:
/// typing changes the query and not the selection until the user commits, while
/// opening the list changes neither.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComboPick {
    /// The chosen option, by index into the option list.
    pub selected: Option<usize>,
    /// Whether the list is up.
    pub open: bool,
}

/// The shared selection state.
pub type SharedPick = Arc<Mutex<ComboPick>>;

/// Whether `option` survives `query`, by v1's rule.
///
/// Case-insensitive substring, and an empty query matches everything. The query is
/// lowercased once per row in v1 and once per row here too — the list is short enough
/// that the allocation is noise next to laying out the rows, and matching v1 exactly is
/// worth more than avoiding it.
pub fn matching(option: &str, query: &str) -> bool {
    let q = query.to_lowercase();
    q.is_empty() || option.to_lowercase().contains(&q)
}

/// The options that survive `query`, in order.
///
/// Order is preserved rather than re-sorted by relevance: v1 did not sort, and a
/// dropdown whose order jumps around while you type is disorienting in a way a
/// slightly worse match order is not.
pub fn visible_options<'a>(options: &'a [String], query: &str) -> Vec<&'a str> {
    options
        .iter()
        .map(String::as_str)
        .filter(|option| matching(option, query))
        .collect()
}

/// Record a choice: the field reads the option and the list closes.
///
/// Returns whether anything changed, so a caller can skip repainting when a re-click on
/// the already-selected option fired — which it will, because the row is still there
/// behind the list.
///
/// `options` is taken so the query can be set to the chosen option's text, which is the
/// whole point of a combobox over a plain text field: after choosing, the field shows
/// what you chose rather than what you last typed.
pub fn select(pick: &SharedPick, query: &SharedBuffer, options: &[String], index: usize) -> bool {
    let Some(option) = options.get(index) else {
        // A row that has gone — the option list shrank under a list that was open.
        // Nothing to choose, and nothing to report.
        return false;
    };
    let changed = match pick.lock() {
        Ok(mut guard) => {
            // Read the old state before overwriting it: a caller that repaints on
            // every `true` would repaint on every click on a row that is already
            // chosen, and the row is still there behind the list.
            let changed = guard.selected != Some(index) || guard.open;
            guard.selected = Some(index);
            guard.open = false;
            changed
        }
        Err(_) => return false,
    };
    if let Ok(mut buffer) = query.lock() {
        *buffer = option.clone();
    }
    changed
}

/// Put a chosen option's text into the field without closing the list.
///
/// This is what `selected(..)` does on a combobox that is handed a pre-selected index:
/// v1 set `query` from `options[selected]` in the constructor and in every setter, and
/// a combobox that opens showing an empty field while `selected` says otherwise is a
/// combobox that has already chosen something and is hiding it.
pub fn sync_query_to_selection(query: &SharedBuffer, options: &[String], selected: Option<usize>) {
    let Some(option) = selected.and_then(|index| options.get(index)) else {
        return;
    };
    if let Ok(mut buffer) = query.lock() {
        *buffer = option.clone();
    }
}

/// What losing focus does to a free-text query that names no particular option.
///
/// `autocomplete` is the reason this exists, and it is a different control from a
/// combobox in one specific way: **its field accepts text that is not an option.** A
/// combobox's callback reports an index; an autocomplete's reports the text. Reconciling
/// one to the other on blur is a real decision, so it is named rather than inlined at a
/// call site — all three kits' v1 sources wrote it out identically, at four lines each,
/// and none of them looked like a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commit {
    /// Nothing to report.
    ///
    /// Not "nothing happened": the selection may well have changed. See
    /// [`commit`] for the case where it does and this is still the answer.
    Unchanged,
    /// Report this: `None` for cleared, `Some(text)` for free text.
    Report(Option<String>),
}

/// Reconcile a free-text query to an option, the way losing focus does.
///
/// v1's rule, in v1's order:
///
/// * an **empty** query clears the selection and reports `None`;
/// * a query with **no exact option** clears the selection and reports the typed text —
///   because the point of an autocomplete is that the field is allowed to hold
///   something that is not on the list;
/// * a query that **exactly matches** an option (ASCII case-insensitively) points the
///   selection at it.
///
/// And then the part that reads like an oversight: **an exact match reports nothing**.
/// v1's third branch sets `selected` and calls no callback at all. So typing "banana",
/// letting it match, and losing focus moves the selection *silently* — a caller watching
/// `on_change` never learns.
///
/// That is preserved, and it is worth preserving deliberately: this is a port, and a
/// caller written against v1 may depend on not being told. It is also the kind of thing
/// that should be fixed as a decision rather than by accident, which is why it is a
/// named variant rather than an absence of code. See the test that pins it.
pub fn commit(pick: &SharedPick, query: &SharedBuffer, options: &[String]) -> Commit {
    let text = query.lock().map(|guard| guard.clone()).unwrap_or_default();
    if text.is_empty() {
        set_selected(pick, None);
        return Commit::Report(None);
    }
    // `eq_ignore_ascii_case` rather than the filter's `to_lowercase().contains(..)`: the
    // filter asks "does this row survive", and this asks "is this row *the* one". A
    // substring test here would make typing "an" select "banana".
    let exact = options
        .iter()
        .position(|option| option.eq_ignore_ascii_case(&text));
    match exact {
        None => {
            set_selected(pick, None);
            Commit::Report(Some(text))
        }
        Some(index) => {
            if selected(pick) != Some(index) {
                set_selected(pick, Some(index));
            }
            Commit::Unchanged
        }
    }
}

/// The chosen option, if any.
fn selected(pick: &SharedPick) -> Option<usize> {
    pick.lock().ok().and_then(|guard| guard.selected)
}

fn set_selected(pick: &SharedPick, index: Option<usize>) {
    if let Ok(mut guard) = pick.lock() {
        guard.selected = index;
    }
}

/// How the panel is placed under the field.
///
/// v1 positioned the panel with `.absolute().inset_top(origin.y + trigger_height)` plus
/// an inline `.margin_top(4.0)`, reading the trigger's origin and height off a
/// `LayoutChangedListener`. Two of those three numbers were a re-derivation of "the
/// bottom edge of the thing I own", which is what [`PopupPlacement::Below`] is, and
/// the third — the 4.0 — was a constant written at the call site rather than a token.
///
/// So the gap is a parameter here. **Material's 4.0 and the other kits' absence of it
/// are preserved as a caller-supplied number rather than reconciled**: two of the three
/// v1 crates write no gap at all, which puts their panel flush against the field, and
/// that difference is a design disagreement this port records rather than resolves.
pub fn panel_placement(gap: f64, max_height: f64) -> PopupPlacement {
    PopupPlacement::Below {
        gap,
        height: max_height,
    }
}

/// The rect a combobox's panel asks the host for.
pub fn panel_rect(placement: PopupPlacement, anchor: floom_types::Rect) -> floom_types::Rect {
    popup_placement_rect(placement, anchor)
}

/// The request a combobox's panel makes of the host.
///
/// Split out so the four facts — a popup, non-modal, the placed rect, content built
/// by a factory — are in one place and a test can read them off without mounting
/// anything. The content is a *factory* because the panel must be able to close itself
/// when a row is chosen, and it cannot hold a handle that does not exist until the host
/// has agreed to mount it.
pub fn panel_request(
    placement: PopupPlacement,
    anchor: floom_types::Rect,
    content: impl FnOnce(floom_types::OverlayHandle) -> Box<dyn WidgetAny> + 'static,
) -> OverlayRequest {
    OverlayRequest::new(
        OverlayKind::Popup,
        // Never modal: a dropdown is dismissed by choosing a row, Escape, or the
        // opener closing it, and the host's outside-press rule only makes sense for a
        // panel that does not block.
        false,
        panel_rect(placement, anchor),
        content,
    )
}

/// How a combobox looks.
///
/// v1 wrote most of this inline at the call site — the 8.0 gap between field and
/// chevron, the 12.0 row padding, the hover alpha of 8, the selected-row alpha of 24,
/// the four disabled alphas. All of those are design decisions, so all of them are
/// fields here with v1's numbers as the defaults. A kit that disagrees passes its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComboboxStyle {
    // --- from `ComboboxTokens` ---
    /// The field's height.
    pub height: f32,
    /// The field's horizontal inner padding.
    pub padding_horizontal: f32,
    /// The field's and the panel's corner radius.
    pub border_radius: f32,
    /// The field's and the panel's minimum width.
    pub min_width: f32,
    /// One row's height.
    pub item_height: f32,
    /// The panel's maximum height.
    ///
    /// v1 declared this in `ComboboxTokens` and **never read it** — the panel was as
    /// tall as its rows. It is read now, because the host lays overlay content out
    /// *against* the requested rect and clips, so a panel needs a bound. Kept for the
    /// same reason `ToolbarTokens::height` is.
    pub panel_max_height: f32,

    // --- v1's call-site constants ---
    /// Gap between the field and the chevron. v1's `.gap(8.0)`.
    pub trigger_gap: f32,
    /// One row's horizontal padding. v1's `.padding_horiz(12.0)`.
    pub row_padding_horizontal: f32,
    /// Gap between the field's bottom and the panel's top. Material wrote
    /// `.margin_top(4.0)`; Cupertino and Fluent wrote nothing, so their default is 0.
    pub panel_gap: f64,
    /// The chevron's size. v1's `.size(14.0)`.
    pub chevron_size: f32,

    // --- v1's colours, as alpha values off theme colours ---
    /// The field's text colour when disabled. v1's `with_alpha(on_surface, 38)`.
    pub disabled_text_alpha: u8,
    /// The chevron's colour when disabled. v1's `with_alpha(on_surface, 97)`.
    pub disabled_chevron_alpha: u8,
    /// The field's border when disabled. v1's `with_alpha(outline, 97)`.
    pub disabled_border_alpha: u8,
    /// The chosen row's fill. v1's `with_alpha(primary, 24)`.
    pub selected_row_alpha: u8,
    /// A hovered row's fill. v1's `with_alpha(on_surface, 8)`.
    pub hover_row_alpha: u8,
    /// The panel's border. v1's `with_alpha(outline, 120)`.
    pub panel_border_alpha: u8,
    /// The panel's shadow. v1's `with_alpha(rgba(0,0,0,255), 20)`.
    pub panel_shadow_alpha: u8,
    /// The outline's own alpha, applied before the border colour is used. v1's
    /// `.border(1.0)`.
    pub border_width: f32,

    // --- resolved theme colours ---
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub primary: Color,
    pub text_size: f32,
}

impl Default for ComboboxStyle {
    fn default() -> Self {
        ComboboxStyle {
            height: 56.0,
            padding_horizontal: 16.0,
            border_radius: 4.0,
            min_width: 240.0,
            item_height: 48.0,
            panel_max_height: 320.0,
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            // Material's 4.0, which is the only one of the three v1 crates that wrote
            // a gap at all. See `panel_gap`'s note above.
            panel_gap: 4.0,
            chevron_size: 14.0,
            disabled_text_alpha: 38,
            disabled_chevron_alpha: 97,
            disabled_border_alpha: 97,
            selected_row_alpha: 24,
            hover_row_alpha: 8,
            panel_border_alpha: 120,
            panel_shadow_alpha: 20,
            border_width: 1.0,
            surface: Color::rgb(0xFF, 0xFB, 0xFE),
            on_surface: Color::rgb(0x1C, 0x1B, 0x1F),
            on_surface_variant: Color::rgb(0x49, 0x45, 0x46),
            outline: Color::rgb(0x79, 0x74, 0x7E),
            primary: Color::rgb(0x67, 0x50, 0xA4),
            text_size: 16.0,
        }
    }
}

impl ComboboxStyle {
    /// The field's border colour, which v1 chose between three cases.
    ///
    /// Order matters and is v1's: disabled wins over open, so a disabled combobox that
    /// somehow has its list up shows the disabled border. Getting this backwards
    /// produces a combobox that looks live while it is not.
    pub fn border_color(&self, disabled: bool, open: bool) -> Color {
        if disabled {
            alpha(self.outline, self.disabled_border_alpha)
        } else if open {
            self.primary
        } else {
            self.outline
        }
    }

    /// The field's text colour.
    pub fn text_color(&self, disabled: bool) -> Color {
        if disabled {
            alpha(self.on_surface, self.disabled_text_alpha)
        } else {
            self.on_surface
        }
    }

    /// The chevron's colour.
    ///
    /// v1 used `on_surface` at alpha 97 when disabled but plain `on_surface_variant`
    /// when not — so the disabled chevron is a *faded version of the normal colour's
    /// source*, not a faded version of the normal colour. Preserved, because a reader
    /// comparing the two states will otherwise assume it is a typo and "fix" it.
    pub fn chevron_color(&self, disabled: bool) -> Color {
        if disabled {
            alpha(self.on_surface, self.disabled_chevron_alpha)
        } else {
            self.on_surface_variant
        }
    }

    /// A chosen row's fill.
    pub fn selected_row_color(&self) -> Color {
        alpha(self.primary, self.selected_row_alpha)
    }

    /// A hovered row's fill.
    pub fn hover_row_color(&self) -> Color {
        alpha(self.on_surface, self.hover_row_alpha)
    }

    /// The panel's shadow.
    ///
    /// Black at `panel_shadow_alpha`, and *not* a shaded theme colour: v1 wrote
    /// `with_alpha(Color::rgba(0, 0, 0, 255), 20)`, which is a shadow tinted by the
    /// design system's own shadow colour in every other component. Recorded rather than
    /// harmonised, because a neutral shadow under a tinted one is the kind of
    /// difference that should be a decision and not an accident.
    pub fn panel_shadow_color(&self) -> Color {
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: self.panel_shadow_alpha,
        }
    }

    /// The panel's placement under the field.
    pub fn placement(&self) -> PopupPlacement {
        panel_placement(self.panel_gap, f64::from(self.panel_max_height))
    }
}

/// `color` with `alpha` applied, in the space v1's `with_alpha` worked in.
///
/// An alpha of 255 means unchanged, which is what makes `with_alpha(colour, 255)` the
/// identity v1 relied on for `panel_shadow_color`.
fn alpha(color: Color, alpha: u8) -> Color {
    Color { a: alpha, ..color }
}
// ---------------------------------------------------------------------------
// The widget
// ---------------------------------------------------------------------------

use floom_types::Shadow;

use crate::OverlayHost;
use floom_widgets::controls::Pressable;
use floom_widgets::primitives::{Role, SemanticsHint, StateBits as SemanticsState, Text};
use floom_widgets::surface::Surface;
use floom_widgets::text_input::{TextInput, TextInputStyle};
use floom_widgets::vector_icon::VectorIcon;
use floom_widgets::Stack;

/// A text field, a chevron, and a filtered list under them.
pub struct Combobox {
    /// The option list. Shared because a row's click handler needs the option text to
    /// put in the field, and a row is built inside a `Send + Sync` closure that cannot
    /// borrow from this struct.
    options: Arc<Mutex<Vec<String>>>,
    query: SharedBuffer,
    pick: SharedPick,
    placeholder: Option<String>,
    style: ComboboxStyle,
    /// Icon path data by name, for the same reason `TreeView` takes one: three kits'
    /// icon sets differ and the engine has no business choosing between them.
    icons: Arc<dyn Fn(&str) -> Option<VectorIcon> + Send + Sync + 'static>,
    disabled: bool,
    on_change: Option<Arc<dyn Fn(Option<usize>) + Send + Sync + 'static>>,
    /// Held as a field rather than returned as a child — see `OverlayHost`'s note on
    /// why unmounting a node does not close the overlays it opened.
    host: OverlayHost,
}

impl std::fmt::Debug for Combobox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Combobox")
            .field("style", &self.style)
            .field("disabled", &self.disabled)
            .field("open", &self.is_open())
            .finish_non_exhaustive()
    }
}

impl Combobox {
    /// A combobox over `options`, which becomes the shared option list.
    pub fn new(options: Vec<String>) -> Self {
        Combobox {
            options: Arc::new(Mutex::new(options)),
            query: Arc::new(Mutex::new(String::new())),
            pick: Arc::new(Mutex::new(ComboPick::default())),
            placeholder: None,
            style: ComboboxStyle::default(),
            icons: Arc::new(|_| None),
            disabled: false,
            on_change: None,
            host: OverlayHost::new(false),
        }
    }

    /// Adopt existing shared state, so a caller that already has one — or that wants
    /// to watch the selection — does not have to hand over ownership.
    pub fn with_shared(options: Vec<String>, query: SharedBuffer, pick: SharedPick) -> Self {
        Combobox {
            options: Arc::new(Mutex::new(options)),
            query,
            pick,
            placeholder: None,
            style: ComboboxStyle::default(),
            icons: Arc::new(|_| None),
            disabled: false,
            on_change: None,
            host: OverlayHost::new(false),
        }
    }

    pub fn style(mut self, style: ComboboxStyle) -> Self {
        self.style = style;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Icon lookup, because three kits' icon sets differ.
    pub fn icons(
        mut self,
        icons: impl Fn(&str) -> Option<VectorIcon> + Send + Sync + 'static,
    ) -> Self {
        self.icons = Arc::new(icons);
        self
    }

    pub fn on_change(mut self, callback: impl Fn(Option<usize>) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    // --- state, readable and writable by a caller ---

    /// The shared selection state.
    pub fn pick(&self) -> SharedPick {
        self.pick.clone()
    }

    /// The field's shared buffer.
    pub fn query(&self) -> SharedBuffer {
        self.query.clone()
    }

    pub fn is_open(&self) -> bool {
        self.pick.lock().map(|guard| guard.open).unwrap_or(false)
    }

    pub fn selected(&self) -> Option<usize> {
        self.pick.lock().ok().and_then(|guard| guard.selected)
    }

    /// Open or close the list. Returns whether this changed anything.
    pub fn set_open(&mut self, open: bool) -> bool {
        match self.pick.lock() {
            Ok(mut guard) if guard.open != open => {
                guard.open = open;
                true
            }
            _ => false,
        }
    }

    /// Choose an option, as a click on its row would.
    ///
    /// Public because v1's `selected(..)` setter did this — it set the index *and*
    /// copied the option's text into the field — and a caller that sets `selected` on a
    /// combobox expects to see the option, not an index the widget disagrees with.
    pub fn select(&mut self, index: usize) -> bool {
        let options = self.options();
        let changed = select(&self.pick, &self.query, &options, index);
        if changed {
            if let Some(on_change) = &self.on_change {
                on_change(Some(index));
            }
        }
        changed
    }

    /// Replace the option list.
    ///
    /// A selection is tracked **by index** — that is what v1 stored, and what
    /// `on_change(Option<usize>)` reports — and an index stops meaning anything the
    /// moment the list behind it is replaced. So the selection is re-pointed **by
    /// text**: the option that was chosen is looked up in the new list, and becomes
    /// the new index.
    ///
    /// Keeping the raw index instead would leave the widget contradicting itself: a
    /// combobox whose field reads "banana" while `selected()` says `Some(1)` and the
    /// list's index 1 is "apple". Two of the three disagree and the user sees the one
    /// that is wrong.
    ///
    /// If the chosen option is **gone**, the selection is dropped *and the field is
    /// cleared*, because a field still reading "banana" above a list with no banana
    /// looks right — which is worse than one reading nothing.
    pub fn set_options(&mut self, options: Vec<String>) {
        // Read the chosen option's *text* before the list is replaced.
        let chosen_text = {
            let selected = self.pick.lock().ok().and_then(|guard| guard.selected);
            selected.and_then(|index| {
                self.options
                    .lock()
                    .ok()
                    .and_then(|guard| guard.get(index).cloned())
            })
        };

        if let Ok(mut guard) = self.options.lock() {
            *guard = options.clone();
        }

        let repointed = match &chosen_text {
            None => None,
            // Two identical options: the first, which is the one the user can see.
            Some(text) => options.iter().position(|candidate| candidate == text),
        };

        if let Ok(mut guard) = self.pick.lock() {
            guard.selected = repointed;
        }
        if let Ok(mut buffer) = self.query.lock() {
            match &chosen_text {
                // Keep showing what was chosen, even if it moved.
                Some(text) => *buffer = text.clone(),
                // Nothing was chosen, so the field is the user's typing: leave it be.
                None => {}
            }
        }
        if repointed.is_none() {
            if let Ok(mut buffer) = self.query.lock() {
                if chosen_text.is_some() {
                    buffer.clear();
                }
            }
        }
    }

    pub fn options(&self) -> Vec<String> {
        self.options
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// The options that survive the current query.
    pub fn visible(&self) -> Vec<String> {
        let options = self.options();
        let query = self
            .query
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        visible_options(&options, &query)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    // --- the two halves, as children ---

    /// The panel: a surface holding the matching rows, or `None` when none match.
    ///
    /// `None` rather than an empty panel, because a dropdown with nothing in it should
    /// not be a box with a border and a shadow around nothing.
    fn panel_content(&self) -> Option<Child> {
        let options = self.options();
        let query = self
            .query
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();
        let rows = visible_options(&options, &query);
        if rows.is_empty() {
            return None;
        }
        let style = self.style;
        let pick = self.pick.clone();
        let shared_options = self.options.clone();
        let query_buffer = self.query.clone();
        let on_change = self.on_change.clone();

        let mut list = Stack::new(Axis::Vertical);
        for (row, option) in rows.into_iter().enumerate() {
            // The *option's* index, not the row's, so a row that survives a keystroke
            // is reused rather than rebuilt at a new key.
            let index = match options.iter().position(|candidate| candidate == option) {
                Some(index) => index,
                // Two identical options; the first is the one the user can see.
                None => row,
            };
            let pick = pick.clone();
            let shared_options = shared_options.clone();
            let query_buffer = query_buffer.clone();
            let on_change = on_change.clone();
            let label = option.to_string();
            list = list.push(move || {
                // One level of cloning, then the row is assembled by a function that
                // takes ownership. Nesting three closures here and cloning at each
                // level is what `TreeView` does, and it is why its note says so: the
                // row builder, the surface builder and the label builder each may run
                // again, and each must hand its children a value it will see twice.
                Box::new(row_widget(
                    style,
                    index,
                    label.clone(),
                    pick.clone(),
                    shared_options.clone(),
                    query_buffer.clone(),
                    on_change.clone(),
                ))
            });
        }

        let panel_layout = LayoutRequest::new()
            .padding_vert(4.0)
            .padding_horiz(4.0)
            .min_width(style.min_width);
        // Built here rather than inside the surface's closure: a `move` closure nested
        // inside another `move` closure *takes ownership* of what it captures, which
        // would make the outer one `FnOnce` — and a slot builder has to be replayable
        // `Fn`, because it is called once per key for as long as the row lives.
        let rows_child = Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
            Box::new(list.clone()) as Box<dyn WidgetAny>
        });

        Some(Child::new(ChildKey::index(0), panel_layout, move || {
            Box::new(
                Surface::new()
                    .request(panel_layout)
                    .background(style.surface)
                    .radius(style.border_radius)
                    .border(
                        style.border_width,
                        alpha(style.outline, style.panel_border_alpha),
                    )
                    .shadow(Shadow {
                        color: style.panel_shadow_color(),
                        blur: 8.0,
                        spread: 1.0,
                        offset_x: 0.0,
                        offset_y: 0.0,
                    })
                    .child(rows_child.clone()),
            ) as Box<dyn WidgetAny>
        }))
    }

    /// The field and the chevron, in a bordered surface.
    fn trigger_content(&self) -> Child {
        let style = self.style;
        let disabled = self.disabled;
        let query = self.query.clone();
        let pick = self.pick.clone();
        let icons = self.icons.clone();
        let placeholder = self.placeholder.clone();
        let open = self.is_open();

        Child::new(ChildKey::index(0), self.trigger_layout(), move || {
            let row = Stack::new(Axis::Horizontal)
                .gap(style.trigger_gap)
                .align(Align::Center, Align::Center)
                .request(LayoutRequest::new().fill())
                .child(Child::new(ChildKey::index(0), LayoutRequest::new(), {
                    // Built here rather than outside and copied in: `TextInput`
                    // and `Pressable` are builders, so there is nothing to clone.
                    let query = query.clone();
                    let pick = pick.clone();
                    let placeholder = placeholder.clone();
                    move || {
                        let mut field = TextInput::new(query.clone()).style(TextInputStyle {
                            font_size: style.text_size,
                            color: style.text_color(disabled),
                            // The field's own border is the trigger surface's
                            // border; two outlines around one control is a widget
                            // that looks like a nest of dolls, and v1 had one.
                            border_width: 0.0,
                            background: None,
                            ..TextInputStyle::default()
                        });
                        if let Some(placeholder) = &placeholder {
                            field = field.placeholder(placeholder.clone());
                        }
                        field = field.on_change({
                            // v1 opened the list on `FocusGained`. 2.0 has no
                            // way for a parent to observe a child's focus, so
                            // the list opens on the first keystroke instead —
                            // the same moment minus the "tap the field, see
                            // nothing happen" one. Recorded, not worked around.
                            let pick = pick.clone();
                            move |_text| {
                                if let Ok(mut guard) = pick.lock() {
                                    guard.open = true;
                                }
                            }
                        });
                        Box::new(field) as Box<dyn WidgetAny>
                    }
                }))
                .child(Child::new(ChildKey::index(1), LayoutRequest::new(), {
                    let pick = pick.clone();
                    let icons = icons.clone();
                    move || {
                        let chevron =
                            Pressable::new(Child::new(ChildKey::index(0), LayoutRequest::new(), {
                                let icons = icons.clone();
                                move || match icons("expand_more") {
                                    Some(mut icon) => {
                                        icon.set_size(style.chevron_size);
                                        icon.set_color(style.chevron_color(disabled));
                                        Box::new(icon) as Box<dyn WidgetAny>
                                    }
                                    // v1's `MdIcon::new("expand_more")` always
                                    // resolved, so an unresolved name is a kit's
                                    // missing icon rather than a blank on purpose.
                                    // The box keeps its size either way, so the
                                    // layout does not depend on the icon set.
                                    None => Box::new(
                                        Surface::new().request(
                                            LayoutRequest::new()
                                                .width(style.chevron_size)
                                                .height(style.chevron_size),
                                        ),
                                    ),
                                }
                            }))
                            .on_click({
                                let pick = pick.clone();
                                move || {
                                    if let Ok(mut guard) = pick.lock() {
                                        guard.open = !guard.open;
                                    }
                                }
                            })
                            .label("Show options");
                        Box::new(chevron) as Box<dyn WidgetAny>
                    }
                }));

            Box::new(
                Surface::new()
                    .request(
                        LayoutRequest::new()
                            .height(style.height)
                            .min_width(style.min_width)
                            .padding_horiz(f64::from(style.padding_horizontal)),
                    )
                    .background(style.surface)
                    .radius(style.border_radius)
                    .border(style.border_width, style.border_color(disabled, open))
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(row.clone()) as Box<dyn WidgetAny>,
                    )),
            ) as Box<dyn WidgetAny>
        })
    }

    fn trigger_layout(&self) -> LayoutRequest {
        LayoutRequest::new()
            .height(self.style.height)
            .min_width(self.style.min_width)
            .padding_horiz(f64::from(self.style.padding_horizontal))
    }
}

impl Widget for Combobox {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The two must be told apart, and this is where they meet. `pick.open` is what
        // the user asked for and what the chevron, a keystroke and the host's own
        // dismissal all write; `host.present` is what the overlay bookkeeping acts on.
        // Nothing wires them together implicitly — a widget that owns both has to say
        // so once per frame, and forgetting is silent: the list state says open while
        // no panel was ever requested.
        self.host.set_present(self.is_open());
        let content = self.panel_content();
        let placement = self.style.placement();
        self.host.reconcile(cx, id, move |bounds, _viewport| {
            let content = content.clone()?;
            Some(panel_request(placement, bounds, move |_handle| {
                content.build()
            }))
        });
    }

    fn children(&self) -> Vec<Child> {
        let mut children = vec![self.trigger_content()];
        children.extend(
            self.host
                .fallback_children(self.panel_content(), ChildKey::index(1)),
        );
        children
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(self.trigger_layout())
    }

    /// Forwarded, and the forwarding is the point — see `OverlayHost`'s note.
    fn overlay_opened(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: floom_types::OverlayHandle,
    ) {
        self.host.opened(handle);
        cx.mark_children_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: floom_types::OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.host.closed(handle) {
            // ...and back the other way. `OverlayHost::closed` clears *its own*
            // `present`, but the state a caller reads is `pick.open`, and leaving that
            // set means the next `update` re-opens a panel the user just dismissed.
            // Two sources of truth, so both directions need saying out loud.
            if let Ok(mut guard) = self.pick.lock() {
                guard.open = false;
            }
            cx.mark_children_dirty(id);
        }
    }

    /// Nothing: the field and the panel are children, and the field's own chrome
    /// belongs to the trigger surface.
    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::primitives::emit_semantics(self.hint(), cx);
    }
}

impl WidgetAny for Combobox {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// A text field that is or is not expanded.
    ///
    /// `TextField` rather than a combobox role, because the accessibility vocabulary in
    /// 2.0 has no combobox and inventing one is a platform decision rather than a
    /// widget's. Recorded here so the gap is visible where it would be fixed.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(self.hint().into())
    }
}

impl Combobox {
    fn hint(&self) -> SemanticsHint {
        let state = if self.is_open() {
            SemanticsState::EXPANDED
        } else {
            SemanticsState::COLLAPSED
        };
        SemanticsHint::labeled(
            Role::TextField,
            self.placeholder.clone().unwrap_or_else(|| {
                self.selected()
                    .and_then(|index| self.options().get(index).cloned())
                    .unwrap_or_default()
            }),
        )
        .with_state(state)
    }
}

/// One option row: a `Pressable` over a surface holding its label.
///
/// A free function taking ownership, rather than another nested closure, because the
/// row is three `Fn` levels deep otherwise — the list's builder, the surface's
/// builder, and the label's — and each level would need its own clone of the same five
/// values. Here there is one level, and it hands everything on by value.
///
/// The selected fill is read when the row is built, not when it is painted. That is
/// what a child-keyed rebuild gives us: choosing an option changes `selected`, the
/// reconciler is asked for children again, and every row is rebuilt, so a row that is
/// now chosen paints its fill. A `Pressable` that read it at paint time would need a
/// repaint request the widget does not otherwise make.
fn row_widget(
    style: ComboboxStyle,
    index: usize,
    label: String,
    pick: SharedPick,
    options: Arc<Mutex<Vec<String>>>,
    query: SharedBuffer,
    on_change: Option<Arc<dyn Fn(Option<usize>) + Send + Sync + 'static>>,
) -> impl WidgetAny {
    let chosen = pick
        .lock()
        .map(|guard| guard.selected == Some(index))
        .unwrap_or(false);
    let row_layout = LayoutRequest::new()
        .height(style.item_height)
        .padding_horiz(f64::from(style.row_padding_horizontal));
    let background = if chosen {
        style.selected_row_color()
    } else {
        Color {
            a: 0,
            ..style.surface
        }
    };
    let accessible = label.clone();

    Pressable::new(Child::new(ChildKey::index(0), row_layout, move || {
        let label = label.clone();
        Box::new(
            Surface::new()
                .request(row_layout)
                .background(background)
                .child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || {
                        Box::new(
                            Text::new(label.clone())
                                .size(style.text_size)
                                .color(style.on_surface),
                        ) as Box<dyn WidgetAny>
                    },
                )),
        ) as Box<dyn WidgetAny>
    }))
    .on_click({
        // Cloned here because `on_click` takes the handle, and the semantics reader
        // below needs one too — a reader written afterwards would find `pick` moved.
        let pick = pick.clone();
        move || {
            let options = options
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            if select(&pick, &query, &options, index) {
                if let Some(on_change) = &on_change {
                    on_change(Some(index));
                }
            }
        }
    })
    .label(accessible)
    .selected_from({
        let pick = pick.clone();
        move || {
            pick.lock()
                .map(|guard| guard.selected == Some(index))
                .unwrap_or(false)
        }
    })
}
