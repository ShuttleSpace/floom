//! The combobox: a text field, a chevron, and a filtered list under them.
//!
//! ## Why this is a decision layer first
//!
//! Every other component in this port came with markup to translate. This one did
//! not: 2.0 had no popup placement and no shared buffer, so both had to be built
//! (`PopupPlacement`, `TextInput`) before there was anything to translate. What is in
//! this file is the part that can be settled without a widget — the filter, the
//! selection state machine, the placement, and the token translation — and the
//! `Widget` impl follows once those are pinned.
//!
//! ## The filter is v1's, including its case handling
//!
//! v1 wrote, per row:
//!
//! ```ignore
//! let q = query.get().to_lowercase();
//! let matches = q.is_empty() || opt.to_lowercase().contains(&q);
//! ```
//!
//! Two properties of that are worth keeping rather than "improving":
//!
//! * **An empty query matches everything**, so the list opens showing all options.
//!   That is the behaviour a user expects from an empty combobox, and it is also what
//!   makes the list a *dropdown* rather than a search-only control.
//! * **Substring, not prefix or word.** "ban" matches "urban planning". A prefix
//!   filter would have been a quiet behaviour change in a component whose whole point
//!   is finding things.
//!
//! Both are in [`matching`] rather than inlined at the call site, because a filter that
//! lives at the call site is a filter each kit re-implements and eventually disagrees
//! about — which is exactly what happened to `list_item`'s tokens.
//!
//! ## What selecting an option does, in one place
//!
//! v1's row click set four things: `selected`, `query` (to the option's text),
//! `open = false`, and the callback. Three of those are a single decision — "the user
//! chose this option, so the field now reads it and the list goes away" — and a
//! decision that has to be written four times at four call sites is one that will be
//! written three and a half times somewhere. So it is [`select`].
//!
//! ## Mounted rows, not mounted-and-hidden
//!
//! v1 mounted **every** option and hid the non-matching ones with `.hide()`. Here only
//! the matching rows are mounted. That is a real difference and it is deliberate:
//!
//! * a hidden row is still a row, so v1 rebuilt and reconciled the whole option list
//!   on every keystroke;
//! * more importantly, keeping the index as the [`ChildKey`] means a row that stays
//!   visible across a keystroke is *reused* rather than rebuilt, which is the
//!   difference between a list that flickers and one that does not.
//!
//! The visible set is the same either way, so this is invisible to a user and visible
//! in a profile.

use std::sync::{Arc, Mutex};

use floom_core::{Color, OverlayRequest, WidgetAny};
use floom_types::OverlayKind;

use crate::compose::{PopupPlacement, popup_placement_rect};
use crate::text_input::SharedBuffer;

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
pub fn select(
    pick: &SharedPick,
    query: &SharedBuffer,
    options: &[String],
    index: usize,
) -> bool {
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
    PopupPlacement::Below { gap, height: max_height }
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