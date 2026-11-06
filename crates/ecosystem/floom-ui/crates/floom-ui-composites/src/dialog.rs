use std::any::Any;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, OverlayHandle,
    OverlayKind, OverlayRequest, PaintContext, SemanticsContext, UpdateContext, Widget, WidgetAny,
};

use floom_widgets::primitives::{SemanticsHint, Text};
use floom_widgets::Stack;

use crate::overlay::Overlay;
use crate::overlay_bookkeeping::{empty_body, OverlayHost};

// Dialog

/// A modal dialog: a centred card rendered above the tree.
///
/// `present` toggles it; while hidden it contributes nothing, so the rest of the
/// tree lays out exactly as if it were not there. Showing it hands the card to
/// the host as a modal overlay, which is what makes it cover the window rather
/// than the box it happens to be declared in — a dialog declared inside a
/// scrolling list is still centred on the screen and still owns the keyboard.
#[derive(Clone)]
pub struct Dialog {
    surface: OverlayHost,
    title: String,
    body: Option<Child>,
    /// The caller's "should this be up" flag, when it has one.
    ///
    /// `None` for a dialog driven purely by [`Self::set_present`], which is what the
    /// engine's own tests and any single-owner caller do. `Some` for a dialog a caller
    /// outside the widget has to drive — which is every ported kit, because v1's
    /// dialogs exposed `open_signal()` "so an external trigger (e.g. a button) can
    /// drive it".
    presence: Option<floom_widgets::SharedFlag>,
    /// A caller-supplied card, when this dialog is not the engine's title-and-body one.
    ///
    /// Set by [`Self::card_with`]. `Box<dyn WidgetAny>` rather than a `Child` because the
    /// kit builds the whole thing, including a close affordance the engine's layout has
    /// no slot for.
    card_override: Option<std::sync::Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>>,
}

impl std::fmt::Debug for Dialog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dialog")
            .field("present", &self.surface.present)
            .finish()
    }
}

impl Dialog {
    pub fn new() -> Self {
        Self {
            surface: OverlayHost::new(false),
            title: String::new(),
            body: None,
            presence: None,
            card_override: None,
        }
    }

    /// A dialog whose **whole card** is the caller's, rather than the engine's
    /// title-above-body stack.
    ///
    /// This exists because v1's dialog card is a composition — a title row carrying a
    /// close button, then the content, then a row of action buttons — and the engine's
    /// [`Self::card`] has no slot for the close button. Rather than give the engine a
    /// `close_button: Option<Child>` that every kit would fill differently, the kit
    /// supplies the card and the engine keeps doing the one thing only it can: mounting
    /// it as a modal overlay over the viewport.
    ///
    /// [`Self::title`] still applies, but **only to semantics**: with a card override the
    /// title is what the dialog is called for accessibility and nothing is rendered from
    /// it, so a caller does not get two titles.
    pub fn card_with(
        mut self,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.card_override = Some(std::sync::Arc::new(build));
        self
    }

    /// A dialog whose presence is driven by a shared flag rather than by this widget's
    /// own [`Self::set_present`].
    ///
    /// This is what every ported kit needs, because v1's dialogs handed out their open
    /// signal — `open_signal()`, documented "so an external trigger (e.g. a button) can
    /// drive it" — and an `RwSignal` is `Rc`-based and cannot cross a `Send + Sync` slot
    /// boundary. A shared flag can, and it is the same move as `TextInput`'s buffer and
    /// `TreeView`'s forest.
    ///
    /// The flag is read in `update` and written in `overlay_closed`, so a caller setting
    /// it opens the dialog and the host dismissing the dialog clears it. Both directions
    /// are said out loud, because the alternative — a flag nobody reads back — is a
    /// dialog that comes straight back after the user closed it.
    pub fn with_shared_presence(mut self, presence: floom_widgets::SharedFlag) -> Self {
        self.presence = Some(presence);
        self
    }

    /// The shared presence flag, if this dialog has one.
    pub fn presence(&self) -> Option<floom_widgets::SharedFlag> {
        self.presence.clone()
    }

    /// The text shown at the top of the card.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// The card body (any widget subtree).
    pub fn body(mut self, body: Child) -> Self {
        self.body = Some(body);
        self
    }

    /// The card body built lazily with an explicit key and layout.
    pub fn body_with(
        mut self,
        key: impl Into<ChildKey>,
        layout: LayoutRequest,
        build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.body = Some(Child::new(key, layout, build));
        self
    }

    /// Shows or hides the dialog; the next frame reconciles.
    pub fn set_present(&mut self, present: bool) {
        self.surface.set_present(present);
    }

    /// Whether the dialog is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    /// A builder for the card, so the overlay path and the in-tree fallback each get
    /// their own instance rather than sharing one — a `Box<dyn WidgetAny>` is not
    /// `Clone`, and a card is a widget instance, so the only way to get a second one is
    /// to build a second one.
    ///
    /// `Arc<dyn Fn ..>` rather than `impl Fn ..` because there are two branches with two
    /// closure types, and `impl Fn` admits exactly one.
    fn card_builder(
        &self,
    ) -> std::sync::Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static> {
        if let Some(card) = self.card_override.clone() {
            return std::sync::Arc::new(move || card());
        }
        let title = self.title.clone();
        let body = self.body.clone();
        std::sync::Arc::new(move || {
            // Cloned inside: an `Fn` cannot move out of its captures, and this builder is
            // called once per mount. `Child` is `Clone`, and `String` is.
            Box::new(
                Stack::new(Axis::Vertical)
                    .gap(8.0)
                    .align(Align::Center, Align::Center)
                    .child(Child::new(ChildKey::index(0), LayoutRequest::new(), {
                        let title = title.clone();
                        move || Box::new(Text::new(title.clone()))
                    }))
                    .child(body.clone().unwrap_or_else(empty_body)),
            )
        })
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.surface.is_shown() {
            SemanticsHint::labeled(SemanticsRole::Dialog, self.title.clone())
        } else {
            SemanticsHint::decorative()
        }
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Dialog {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The caller's flag is the truth about *whether to ask*; `surface.present` is
        // what the bookkeeping acts on. Nothing wires them implicitly — and this exact
        // missing wire cost `Combobox` two bugs in a row.
        if let Some(presence) = self.presence.clone() {
            self.surface
                .set_present(floom_widgets::read_flag(&presence));
        }
        let build_card = self.card_builder();
        // A dialog covers the window, not its own box, so it is sized from the
        // viewport the host reports rather than from `bounds`.
        self.surface.reconcile(cx, id, move |_bounds, viewport| {
            Some(OverlayRequest::new(
                OverlayKind::Dialog,
                true,
                viewport,
                move |_handle| build_card(),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        // The host refused the overlay (or has none at all), so fall back to the
        // in-tree layer this widget used before it could escape its parent.
        //
        // `card()` is called *inside* the builder rather than captured and cloned: a
        // `Box<dyn WidgetAny>` is not `Clone`, and a card is a widget instance, so the
        // only way to get a second one is to build a second one.
        let build_card = self.card_builder();
        let surface =
            Overlay::new().content_with(ChildKey::index(0), LayoutRequest::new(), move || {
                build_card()
            });
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(surface.clone()),
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().absolute_fill())
    }

    fn overlay_opened(&mut self, cx: &mut dyn UpdateContext, id: ElementId, handle: OverlayHandle) {
        self.surface.opened(handle);
        cx.mark_dirty(id);
    }

    fn overlay_closed(
        &mut self,
        cx: &mut dyn UpdateContext,
        id: ElementId,
        handle: OverlayHandle,
        _content: Box<dyn WidgetAny>,
    ) {
        if self.surface.closed(handle) {
            // ...and back the other way, so a caller reading the flag sees the dialog
            // close. Two sources of truth, so both directions need saying out loud.
            if let Some(presence) = &self.presence {
                floom_widgets::write_flag(presence, false);
            }
            cx.mark_dirty(id);
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(self.semantics_hint(), cx);
    }
}
