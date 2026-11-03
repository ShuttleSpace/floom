
use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyEvent, KeyPhase, NamedKey,
    OverlayHandle, OverlayKind, OverlayRequest, PaintContext, Point, Rect, SemanticsContext, Size,
    UpdateContext, Widget, WidgetAny,
};
use floom_types::{FontWeight, PointerButton, PointerPhase, Shadow};

use floom_widgets::{
    emit_semantics, paint_icon, text_width_at, InteractionState, Revision, SemanticsHint,
    VectorIcon,
};

pub type IndexCallback = Box<dyn Fn(usize)>;

/// The open option list, mounted as an overlay rather than painted under the
/// Select's own bounds.
///
/// The inline version could never be clicked. A click is routed by hit-testing
/// the tree, and the rows were raw paint ops inside the Select's own node —
/// which is one row tall — so every point below the header landed somewhere
/// else entirely. Layering it as an overlay fixes both halves at once: the
/// rows become real nodes that hit-test, and the overlay paints after the main
/// tree so it escapes its parent's clip instead of being cut off by it.
///
/// The popup owns the highlight while it is up — the arrow keys belong to it,
/// not to the Select, because it is what the host gave focus to — and reports
/// the row the user picked back to the opening Select through
/// [`Select::overlay_closed`]. `None` means it was dismissed without choosing.
pub struct SelectPopup {
    options: Vec<String>,
    /// Row the pointer is over or the arrow keys last moved to.
    highlight: usize,
    /// The row the Select had chosen before the list opened; it keeps its own
    /// tint while the highlight moves over other rows.
    selected: Option<usize>,
    /// Copied from the opening Select rather than shared: the popup outlives the
    /// frame in which it was built, and a style is `Copy` precisely so this can
    /// be a snapshot.
    style: SelectStyle,
    /// Set once the user picks a row; `None` means dismissed.
    chosen: Option<usize>,
    /// Handed in by the content factory so the popup can take itself down
    /// without reaching back into the Select that opened it.
    handle: Option<OverlayHandle>,
}

impl std::fmt::Debug for SelectPopup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SelectPopup")
            .field("options", &self.options)
            .field("highlight", &self.highlight)
            .field("chosen", &self.chosen)
            .finish_non_exhaustive()
    }
}

impl SelectPopup {
    /// Builds the list, highlighting `selected` so the keyboard continues from
    /// where the Select left off.
    pub fn new(
        options: Vec<String>,
        selected: Option<usize>,
        style: SelectStyle,
        handle: OverlayHandle,
    ) -> Self {
        Self {
            highlight: selected.unwrap_or(0),
            options,
            selected,
            style,
            chosen: None,
            handle: Some(handle),
        }
    }

    pub fn options(&self) -> &[String] {
        &self.options
    }

    /// One row's height, from the style it was opened with.
    pub fn row_height(&self) -> f64 {
        f64::from(self.style.item_height)
    }

    /// The offset of row 0 from the popup's own top edge — the panel's padding,
    /// which the rows sit inside.
    pub fn row_origin(&self) -> f64 {
        f64::from(self.style.panel_padding)
    }

    pub fn style(&self) -> SelectStyle {
        self.style
    }

    /// The row the user picked, if they picked one rather than dismissing.
    pub fn chosen(&self) -> Option<usize> {
        self.chosen
    }

    pub fn highlight(&self) -> usize {
        self.highlight
    }

    /// Row whose band contains `y` within `bounds`, if any.
    ///
    /// The band starts *after* the panel's padding, so a click in the padding
    /// above the first row hits no row — which is what v1's padded panel did
    /// and what the popup's rect (padding included) promises.
    pub fn row_at(&self, bounds: Rect, y: f64) -> Option<usize> {
        let row_height = self.row_height();
        if self.options.is_empty() || row_height <= 0.0 {
            return None;
        }
        let offset = y - bounds.y0 - self.row_origin();
        if offset < 0.0 {
            return None;
        }
        let index = (offset / row_height) as usize;
        (index < self.options.len()).then_some(index)
    }

    fn commit(&mut self, cx: &mut dyn EventContext, chosen: Option<usize>) {
        self.chosen = chosen;
        if let Some(handle) = self.handle.take() {
            cx.close_overlay(handle);
        }
    }

    fn move_highlight(&mut self, to: usize) -> bool {
        if self.options.is_empty() {
            return false;
        }
        let next = to.min(self.options.len() - 1);
        let moved = next != self.highlight;
        self.highlight = next;
        moved
    }

    /// The list describes itself by the row under the highlight, which is what
    /// a screen reader should read out as the user walks it.
    pub fn semantics_hint(&self) -> SemanticsHint {
        let label = self
            .options
            .get(self.highlight)
            .cloned()
            .unwrap_or_default();
        SemanticsHint::labeled(SemanticsRole::List, label)
            .with_state(SemanticsState::EXPANDED | SemanticsState::FOCUSED)
    }
}

impl Widget for SelectPopup {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        match cx.event() {
            Event::Pointer(pointer) => match pointer.phase {
                PointerPhase::Move | PointerPhase::Enter => {
                    // Hovering is not a change worth reporting: the highlight
                    // follows the pointer, and the host repaints the frame
                    // either way.
                    if let Some(index) = self.row_at(cx.bounds(), pointer.position.y) {
                        self.highlight = index;
                    }
                    false
                }
                PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => true,
                PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                    let row = self.row_at(cx.bounds(), pointer.position.y);
                    let picked = row.or(Some(self.highlight));
                    self.commit(cx, picked);
                    true
                }
                _ => false,
            },
            Event::Key(key) => {
                if key.phase == KeyPhase::Up {
                    return false;
                }
                match key.key {
                    Key::Named(NamedKey::ArrowDown) => {
                        let next = self.highlight + 1;
                        self.move_highlight(next);
                        true
                    }
                    Key::Named(NamedKey::ArrowUp) => {
                        let next = self.highlight.saturating_sub(1);
                        self.move_highlight(next);
                        true
                    }
                    Key::Named(NamedKey::Home) => {
                        self.move_highlight(0);
                        true
                    }
                    Key::Named(NamedKey::End) => {
                        let last = self.options.len().saturating_sub(1);
                        self.move_highlight(last);
                        true
                    }
                    Key::Named(NamedKey::Enter) | Key::Char(' ') => {
                        let picked = Some(self.highlight);
                        self.commit(cx, picked);
                        true
                    }
                    Key::Named(NamedKey::Escape) => {
                        self.commit(cx, None);
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        let row_height = self.row_height();

        cx.draw_shadow(
            bounds,
            style.border_radius,
            &Shadow {
                color: style.panel_shadow,
                blur: f64::from(style.panel_shadow_blur),
                offset_x: 0.0,
                offset_y: 0.0,
                spread: f64::from(style.panel_shadow_spread),
            },
        );
        if style.panel_border.a > 0 {
            let inset = f64::from(style.border_width.max(0.0));
            cx.fill_rounded_rect(bounds, style.border_radius, style.panel_border);
            cx.fill_rounded_rect(
                Rect {
                    x0: bounds.x0 + inset,
                    y0: bounds.y0 + inset,
                    x1: (bounds.x1 - inset).max(bounds.x0 + inset),
                    y1: (bounds.y1 - inset).max(bounds.y0 + inset),
                },
                (style.border_radius - inset as f32).max(0.0),
                style.surface,
            );
        } else {
            cx.fill_rounded_rect(bounds, style.border_radius, style.surface);
        }

        let top = bounds.y0 + self.row_origin();
        for (index, option) in self.options.iter().enumerate() {
            let row = Rect {
                x0: bounds.x0,
                y0: top + index as f64 * row_height,
                x1: bounds.x1,
                y1: top + (index + 1) as f64 * row_height,
            };
            if row.y0 >= bounds.y1 {
                // The panel was capped by `panel_max_height`; the rows that
                // fall past it are not visible and painting them would draw
                // outside the box the host gave us.
                break;
            }
            if Some(index) == self.selected && style.selected_row.a > 0 {
                cx.fill_rect(row, style.selected_row);
            } else if index == self.highlight && style.hover_row.a > 0 {
                cx.fill_rect(row, style.hover_row);
            }
            cx.fill_text(
                Point::new(
                    row.x0 + f64::from(style.row_padding_horizontal),
                    row.y0 + (row_height - f64::from(style.text_size)) / 2.0,
                ),
                option,
                style.text_size,
                FontWeight::Normal,
                style.outline,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

/// A single-choice selector.
///
/// Choosing opens [`SelectPopup`] as an overlay, so the list escapes the
/// control's own box instead of being painted inside it — see that type for
/// why an inline list could not be clicked.

/// How a select (dropdown) looks.
///
/// Every field here is read by [`Select`] or [`SelectPopup`], and the coverage
/// is what the three kits' disagreement needs rather than what one kit's markup
/// happened to spell out. Colours are *resolved* rather than carried as an
/// alpha over a theme colour: the engine has no theme, and the one place the
/// kits genuinely disagree — a hovered row is `on_surface`@8 in Material and
/// Fluent but `primary`@12 in Cupertino — is a different *source* colour, not a
/// different alpha, so an `alpha: u8` field could not have carried it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectStyle {
    // --- geometry, from `SelectTokens` ---
    /// The trigger's height.
    pub height: f32,
    /// The trigger's horizontal inner padding.
    pub padding_horizontal: f32,
    /// The trigger's and the popup's corner radius.
    pub border_radius: f32,
    /// The trigger's and the popup's minimum width.
    pub min_width: f32,
    /// One row's height in the popup.
    pub item_height: f32,
    /// The popup's maximum height; the list stops growing here.
    pub panel_max_height: f32,

    // --- geometry, v1's call-site constants ---
    /// Gap between the trigger's label and its chevron.
    pub trigger_gap: f32,
    /// One row's horizontal padding in the popup.
    pub row_padding_horizontal: f32,
    /// Gap between the trigger's bottom and the popup's top.
    pub panel_gap: f64,
    /// Padding between the popup's edge and its first row.
    pub panel_padding: f32,
    /// The chevron's edge length.
    pub icon_size: f32,
    /// The trigger's border thickness. `0.0` draws no border.
    pub border_width: f32,
    /// Label font size, from the theme's body-medium.
    pub text_size: f32,
    /// The popup's shadow blur radius.
    pub panel_shadow_blur: f32,
    /// The popup's shadow spread.
    pub panel_shadow_spread: f32,

    // --- resolved colours ---
    /// The trigger's and the popup's background.
    pub surface: Color,
    /// The chosen option's label.
    pub on_surface: Color,
    /// The trigger's border, and the popup's rows' text.
    pub outline: Color,
    /// The label when nothing is chosen.
    pub placeholder: Color,
    /// The chevron.
    pub chevron: Color,
    /// The band behind the row the pointer or the arrow keys are on.
    pub hover_row: Color,
    /// The band behind the chosen row.
    pub selected_row: Color,
    /// The popup's border; `TRANSPARENT` draws none.
    pub panel_border: Color,
    /// The popup's shadow.
    pub panel_shadow: Color,
    /// The label when the control is disabled.
    pub disabled_text: Color,
    /// The chevron when the control is disabled.
    pub disabled_chevron: Color,
    /// The trigger's border when the control is disabled.
    pub disabled_border: Color,
    /// The trigger's background when the control is disabled.
    pub disabled_background: Color,
}

impl Default for SelectStyle {
    fn default() -> Self {
        SelectStyle {
            height: 40.0,
            padding_horizontal: 16.0,
            border_radius: 4.0,
            min_width: 160.0,
            item_height: 32.0,
            panel_max_height: 240.0,
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            panel_gap: 0.0,
            panel_padding: 4.0,
            icon_size: 16.0,
            border_width: 1.0,
            text_size: 14.0,
            panel_shadow_blur: 8.0,
            panel_shadow_spread: 1.0,
            surface: Color::rgb(0xFF, 0xFF, 0xFF),
            on_surface: Color::rgb(0x1C, 0x1B, 0x1F),
            outline: Color::rgb(0x79, 0x74, 0x7E),
            placeholder: Color::rgb(0x49, 0x45, 0x46),
            chevron: Color::rgb(0x49, 0x45, 0x46),
            hover_row: Color::rgba(0x1C, 0x1B, 0x1F, 8),
            selected_row: Color::rgba(0x67, 0x50, 0xA4, 24),
            panel_border: Color::rgba(0x79, 0x74, 0x7E, 120),
            panel_shadow: Color::rgba(0x00, 0x00, 0x00, 20),
            disabled_text: Color::rgba(0x1C, 0x1B, 0x1F, 97),
            disabled_chevron: Color::rgba(0x1C, 0x1B, 0x1F, 97),
            disabled_border: Color::rgba(0x79, 0x74, 0x7E, 97),
            disabled_background: Color::rgba(0x1C, 0x1B, 0x1F, 12),
        }
    }
}

/// Height of the popup for `rows` options: one [`SelectStyle::item_height`]
/// each, plus the panel's padding, capped at [`SelectStyle::panel_max_height`].
///
/// Shared by [`Select::open_popup`] (which has to guess the rect before the
/// host measures anything) and by the tests that assert the guess — one
/// definition so the two cannot drift.
pub fn popup_height(style: &SelectStyle, rows: usize) -> f64 {
    let wanted = f64::from(style.item_height) * rows as f64 + f64::from(style.panel_padding) * 2.0;
    wanted.min(f64::from(style.panel_max_height))
}

/// A single-choice selector.

/// Choosing opens [`SelectPopup`] as an overlay, so the list escapes the
/// control's own box instead of being painted inside it — see that type for
/// why an inline list could not be clicked.
pub struct Select {
    options: Vec<String>,
    selected: Option<usize>,
    /// Shown in place of a label while nothing is chosen.
    placeholder: String,
    /// The chevron. A kit supplies it rather than the engine: three icon sets
    /// disagree on what "expand" looks like, and picking one belongs to the kit.
    chevron: Option<VectorIcon>,
    on_change: Option<IndexCallback>,
    state: InteractionState,
    revision: Revision,
    /// The open popup, if any. This *is* the open/closed state: it is set when
    /// the host confirms the overlay opened and cleared from
    /// [`Widget::overlay_closed`], so the control can never believe a list is
    /// up that nothing is painting.
    popup: Option<OverlayHandle>,
    /// Visual style.
    style_: SelectStyle,
}

impl std::fmt::Debug for Select {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Select")
            .field("options", &self.options)
            .field("selected", &self.selected)
            .field("popup", &self.popup)
            .finish_non_exhaustive()
    }
}

impl Select {
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: None,
            placeholder: String::new(),
            chevron: None,
            on_change: None,
            state: InteractionState::default(),
            revision: Revision::default(),
            popup: None,
            style_: SelectStyle::default(),
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = Some(index);
        self.revision.bump();
        self
    }

    /// The label shown while nothing is chosen. Empty means [`Select::selected_label`]
    /// falls back to the em dash it has always used.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self.revision.bump();
        self
    }

    /// The chevron drawn at the trigger's trailing edge. Sized from
    /// [`SelectStyle::icon_size`] and tinted from [`SelectStyle::chevron`] —
    /// the *shape* is the kit's, the rest is the style's.
    pub fn chevron(mut self, icon: VectorIcon) -> Self {
        self.chevron = Some(icon);
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn style(mut self, style: SelectStyle) -> Self {
        self.style_ = style;
        self.revision.bump();
        self
    }

    pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn options(&self) -> &[String] {
        &self.options
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.selected
    }

    pub fn is_open(&self) -> bool {
        self.popup.is_some()
    }

    /// The label the trigger shows: the chosen option, or the placeholder
    /// while nothing is chosen, or an em dash when there is neither.
    pub fn selected_label(&self) -> &str {
        match self.selected.and_then(|i| self.options.get(i)) {
            Some(label) => label,
            None if !self.placeholder.is_empty() => &self.placeholder,
            None => "—",
        }
    }

    /// Whether the trigger is currently showing the placeholder rather than a
    /// chosen option — the two are painted in different colours.
    pub fn showing_placeholder(&self) -> bool {
        self.selected.and_then(|i| self.options.get(i)).is_none()
    }

    /// The placeholder as configured; empty when none was set.
    pub fn placeholder_text(&self) -> &str {
        &self.placeholder
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// The open popup's handle, for a caller that wants to close it from
    /// outside the control.
    pub fn popup_handle(&self) -> Option<OverlayHandle> {
        self.popup
    }

    /// Chooses an option and closes the list; no-op when out of range.
    pub fn select(&mut self, index: usize) -> bool {
        self.choose(index)
    }

    /// Chooses an option.
    ///
    /// Returns whether the event was handled — re-picking the selected option
    /// still counts, because it closes the list.
    fn choose(&mut self, index: usize) -> bool {
        if index >= self.options.len() {
            return false;
        }
        let changed = self.selected != Some(index);
        self.selected = Some(index);
        self.revision.bump();
        if changed {
            if let Some(callback) = &self.on_change {
                callback(index);
            }
        }
        true
    }

    /// Moves the selection by `delta`. From nothing selected, either direction
    /// lands on the end it is heading for.
    fn move_selection(&mut self, delta: isize) -> bool {
        let len = self.options.len();
        if len == 0 {
            return false;
        }
        let next = match self.selected {
            Some(index) => (index as isize + delta).clamp(0, len as isize - 1) as usize,
            None if delta > 0 => 0,
            None => len - 1,
        };
        self.choose(next)
    }

    /// Opens the option list as an overlay anchored just below the control.
    ///
    /// The rect is a guess, and deliberately so: the host lays the content out
    /// against it, so the only thing the Select has to get right is how tall
    /// the list is and where its top-left corner goes. Asking the host to
    /// measure first would mean opening the overlay to find out whether it
    /// should be opened.
    pub fn open_popup(&mut self, cx: &mut dyn EventContext) -> bool {
        if self.is_open() || self.options.is_empty() {
            return false;
        }
        let bounds = cx.bounds();
        let style = self.style_;
        let rect = Rect::from_origin_size(
            Point::new(bounds.x0, bounds.y1 + style.panel_gap),
            Size::new(
                (bounds.width()).max(f64::from(style.min_width)),
                popup_height(&style, self.options.len()),
            ),
        );
        let options = self.options.clone();
        let selected = self.selected;
        let opened = cx.open_overlay(OverlayRequest::new(
            OverlayKind::Popup,
            false,
            rect,
            move |handle| Box::new(SelectPopup::new(options, selected, style, handle)),
        ));
        if opened {
            self.revision.bump();
        }
        opened
    }

    /// Closes the open list without choosing anything.
    pub fn close_popup(&mut self, cx: &mut dyn EventContext) -> bool {
        match self.popup.take() {
            Some(handle) => {
                cx.close_overlay(handle);
                self.revision.bump();
                true
            }
            None => false,
        }
    }

    fn toggle_popup(&mut self, cx: &mut dyn EventContext) -> bool {
        if self.is_open() {
            self.close_popup(cx)
        } else {
            self.open_popup(cx)
        }
    }

    /// Keyboard control for the focused, *closed* select: open the list, walk
    /// the selection, jump to either end.
    ///
    /// Once the list is open the keyboard belongs to [`SelectPopup`] — focus
    /// moved into the overlay when it opened — so navigation while open is the
    /// popup's business, and picking a row there is what changes the selection.
    fn key_event(&mut self, cx: &mut dyn EventContext, key: &KeyEvent) -> bool {
        if !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        match key.key {
            Key::Named(NamedKey::Enter) | Key::Char(' ') => self.toggle_popup(cx),
            Key::Named(NamedKey::Escape) => self.close_popup(cx),
            _ if self.is_open() => false,
            Key::Named(NamedKey::ArrowDown) => self.move_selection(1),
            Key::Named(NamedKey::ArrowUp) => self.move_selection(-1),
            Key::Named(NamedKey::Home) => self.choose(0),
            Key::Named(NamedKey::End) => self.choose(self.options.len().saturating_sub(1)),
            _ => false,
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.is_open() {
            state |= SemanticsState::EXPANDED;
        }
        SemanticsHint::labeled(SemanticsRole::List, self.selected_label().to_string())
            .with_state(state)
    }
}

impl Widget for Select {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Select::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size of the *closed* control: the widest label (selected or
    /// option) at the style's font size, plus the style's padding and — when a
    /// chevron was supplied — its edge and the gap before it;
    /// [`SelectStyle::height`] tall. The open list is an overlay sized from this
    /// box, so it deliberately does not contribute to the intrinsic extent —
    /// otherwise the control would grow a list-height hole in its parent's flow
    /// every time it opened.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        let widest = self
            .options
            .iter()
            .map(|option| text_width_at(option, self.style_.text_size))
            .chain(std::iter::once(text_width_at(
                self.selected_label(),
                self.style_.text_size,
            )))
            .fold(0.0_f32, f32::max);
        let trailing = if self.chevron.is_some() {
            self.style_.trigger_gap + self.style_.icon_size
        } else {
            0.0
        };
        Some(
            floom_core::LayoutRequest::new()
                .width(
                    (widest + self.style_.padding_horizontal * 2.0 + trailing)
                        .max(self.style_.min_width),
                )
                .height(self.style_.height),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        if self.state.disabled {
            return false;
        }
        let Event::Pointer(pointer) = cx.event() else {
            return match cx.event() {
                Event::Key(key) => self.key_event(cx, &key),
                _ => false,
            };
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                self.state.hovered = true;
                false
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                self.state.hovered = false;
                false
            }
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                self.state.hovered = true;
                cx.request_focus(id);
                false
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                // Only the header can be hit: the list is an overlay with its
                // own nodes, so a click below it never reaches the Select.
                self.toggle_popup(cx)
            }
            _ => false,
        }
    }

    /// The open list came down. If it carries a chosen row, that is the new
    /// selection; either way the control stops believing a popup is up.
    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        content: Box<dyn WidgetAny>,
    ) {
        if self.popup != Some(handle) {
            return;
        }
        self.popup = None;
        let chosen = content
            .as_any()
            .downcast_ref::<SelectPopup>()
            .and_then(SelectPopup::chosen);
        if let Some(index) = chosen {
            self.choose(index);
        }
        self.revision.bump();
        cx.mark_dirty(id);
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.popup = Some(handle);
        self.revision.bump();
        cx.mark_dirty(id);
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style_;
        let disabled = self.state.disabled;

        // A border is a larger box in the border colour with a smaller box on
        // top of it: `PaintContext` has a rounded-rect fill and no stroke, and
        // drawing one honestly beats a stroke op nobody implemented.
        let (border, background) = if disabled {
            (style.disabled_border, style.disabled_background)
        } else {
            (style.outline, style.surface)
        };
        let inset = f64::from(style.border_width.max(0.0));
        if inset > 0.0 && border.a > 0 {
            cx.fill_rounded_rect(bounds, style.border_radius, border);
        }
        cx.fill_rounded_rect(
            Rect {
                x0: bounds.x0 + inset,
                y0: bounds.y0 + inset,
                x1: (bounds.x1 - inset).max(bounds.x0 + inset),
                y1: (bounds.y1 - inset).max(bounds.y0 + inset),
            },
            (style.border_radius - inset as f32).max(0.0),
            background,
        );

        let text_color = if disabled {
            style.disabled_text
        } else if self.showing_placeholder() {
            style.placeholder
        } else {
            style.on_surface
        };
        // Left-aligned at the padding, vertically centred — a select's label
        // sits at the leading edge with the chevron opposite it, unlike a
        // button's centred caption.
        cx.fill_text(
            Point::new(
                bounds.x0 + f64::from(style.padding_horizontal),
                bounds.y0 + (bounds.height() - f64::from(style.text_size)) / 2.0,
            ),
            self.selected_label(),
            style.text_size,
            FontWeight::Normal,
            text_color,
        );

        if let Some(chevron) = &self.chevron {
            let edge = f64::from(style.icon_size);
            let icon_bounds = Rect::from_origin_size(
                Point::new(
                    bounds.x1 - f64::from(style.padding_horizontal) - edge,
                    bounds.y0 + (bounds.height() - edge) / 2.0,
                ),
                Size::new(edge, edge),
            );
            let tint = if disabled {
                style.disabled_chevron
            } else {
                style.chevron
            };
            paint_icon(cx, chevron, icon_bounds, tint);
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }

    fn cursor(&self) -> floom_types::CursorStyle {
        // A disabled control gets the platform arrow: a pointer over a
        // thing that cannot be clicked is a promise the control cannot keep.
        if self.state.disabled {
            floom_types::CursorStyle::Default
        } else {
            floom_types::CursorStyle::Pointer
        }
    }
}
