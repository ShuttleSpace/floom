//! Cupertino 3 autocomplete, ported to the Floom 2.0 widget API.
//!
//! ## What this is, relative to a combobox
//!
//! A combobox and an autocomplete share 93% of their code and differ in exactly one
//! way: **an autocomplete's field accepts text that is not an option.** Everything else —
//! the field, the chevron, the filter, the rows, the panel placement — is the engine's
//! [`floom_widgets::Combobox`], promoted once and shared by both.
//!
//! That one difference moves two things, and both live here rather than in the engine:
//!
//! * **the callback's payload.** A combobox reports an `Option<usize>`; this reports an
//!   `Option<String>`, because "the user typed kiwi, which is not on the list" is a real
//!   answer and an index cannot express it. The engine's index-based `on_change` is
//!   therefore *adapted* here rather than replaced.
//! * **the commit point.** Losing focus reconciles the typed text against the list, which
//!   is [`floom_widgets::commit`]. v1 wrote that rule out at four lines; here it is one
//!   call.
//!
//! ## The one behaviour the port cannot reproduce
//!
//! **v1 committed on `FocusLost`, and 2.0 has no way for a parent to observe a child's
//! focus.** So [`CupertinoAutocomplete::commit`] is public: a caller wires it to whatever it
//! does have — a wrapper's blur handler, an `Escape` key on a parent, the end of a
//! search — and the rule inside it is the engine's, tested there.
//!
//! That is a real loss of fidelity and it is stated rather than papered over. The
//! alternative — committing on every keystroke — would be a different control: the field
//! would snap its selection to whatever the text currently matches, which is not what an
//! autocomplete does.
//!
//! ## The silent exact match
//!
//! Committing text that exactly matches an option moves the selection and **reports
//! nothing**. That is v1's behaviour, preserved, and it is why `Commit::Unchanged` is a
//! named variant in the engine rather than an absence of code. See
//! `floom_widgets::combobox::commit`.
//!
//! ## Cupertino's `.margin_top(4.0)`
//!
//! Cupertino is the only one of the three kits that wrote a panel gap; Cupertino and
//! Fluent wrote none. `panel_gap` is a parameter and 4.0 is the default here *because it
//! is the only one of the three that wrote a number* — a default invented here would have
//! been a fourth opinion.

use floom_legacy_reactive::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_widgets::{Combobox, ComboboxStyle, Commit, SharedPick, commit, select};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::autocomplete::AutocompleteProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Icon names are resolved by the kit, because three kits' icon sets differ and the
/// engine has no business choosing between them.
fn resolve_icon(name: &str) -> Option<floom_widgets::VectorIcon> {
    // This kit's own table. v1 used `cupertino_icon_svg`, and its `expand_more` is a
    // different glyph from Material's — a lookup that reached for the Material table
    // would draw a Material chevron in a Cupertino control and nothing would fail.
    Some(floom_widget_cupertino_icon::cupertino_icon(name))
}

/// Cupertino's autocomplete.
pub struct CupertinoAutocomplete {
    props: AutocompleteProps,
    theme: RwSignal<MultiTheme>,
    /// The shared selection state, read by the engine widget and by [`Self::commit`].
    pick: SharedPick,
    /// The field's shared buffer — the commit rule needs the typed text, and 2.0 has
    /// one buffer rather than a signal both halves could read.
    query: floom_widgets::SharedBuffer,
    /// v1's `Box<dyn Fn(Option<String>)>`, kept so `props()` still reports it.
    on_change: Option<std::sync::Arc<dyn Fn(Option<String>) + Send + Sync + 'static>>,
}

impl CupertinoAutocomplete {
    pub fn new() -> Self {
        CupertinoAutocomplete {
            pick: std::sync::Arc::new(std::sync::Mutex::new(Default::default())),
            query: floom_widgets::SharedBuffer::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            props: AutocompleteProps::default(),
            on_change: None,
        }
    }

    pub fn options(mut self, options: Vec<String>) -> Self {
        self.props.options = options;
        self
    }

    pub fn selected(mut self, selected: Option<usize>) -> Self {
        self.props.selected = selected;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the *text*, not an index.
    ///
    /// That is the whole difference from a combobox's callback, and it is why the
    /// engine's index-based `on_change` is adapted rather than handed through: "kiwi" is
    /// a legitimate answer from this field and has no index.
    pub fn on_change(
        mut self,
        callback: impl Fn(Option<String>) + Send + Sync + 'static,
    ) -> Self {
        self.props.on_change = Some(Box::new({
            let shared = std::sync::Arc::new(callback);
            self.on_change = Some(shared.clone());
            move |text: Option<String>| shared(text)
        }));
        self
    }

    pub fn props(&self) -> &AutocompleteProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The shared selection state, for a caller that wants to read it.
    pub fn pick(&self) -> SharedPick {
        self.pick.clone()
    }

    /// The field's shared buffer, for a caller that wants to read or write the text.
    pub fn query(&self) -> floom_widgets::SharedBuffer {
        self.query.clone()
    }

    /// Reconcile the typed text against the options, the way losing focus did.
    ///
    /// **Public because 2.0 cannot do it for us**: v1 hung this on the field's
    /// `FocusLost`, and there is no way for a parent to observe a child's focus. A caller
    /// wires this to whatever it does have — a wrapper's blur handler, an `Escape`, the
    /// end of a search.
    ///
    /// Returns whether anything was reported, so a caller can tell "committed and said
    /// so" from "committed silently" — and the second case is real, because an exact
    /// match moves the selection without saying anything.
    pub fn commit(&self) -> bool {
        let outcome = commit(&self.pick, &self.query, &self.props.options);
        match outcome {
            Commit::Unchanged => false,
            Commit::Report(text) => {
                if let Some(on_change) = &self.on_change {
                    on_change(text);
                }
                true
            }
        }
    }

    /// Choose an option, as clicking its row would: the selection moves, the field reads
    /// it, and the list closes.
    pub fn select(&mut self, index: usize) -> bool {
        let changed = select(&self.pick, &self.query, &self.props.options, index);
        if changed {
            if let Some(on_change) = &self.on_change {
                // An index becomes its text, which is this control's whole difference
                // from a combobox.
                on_change(self.props.options.get(index).cloned());
            }
        }
        changed
    }

    pub fn resolved_style(&self) -> ComboboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Autocomplete(tokens) = theme.component(ComponentId::Autocomplete)
        else {
            unreachable!("ComponentId::Autocomplete must resolve to AutocompleteTokens")
        };
        let surface = theme.colors().surface;
        let on_surface = theme.colors().on_surface;
        let outline = theme.colors().outline;
        ComboboxStyle {
            // --- from `AutocompleteTokens`, whose six fields are exactly
            //     `ComboboxTokens`' six ---
            height: tokens.height,
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,
            min_width: tokens.min_width,
            item_height: tokens.item_height,
            // v1 declared this and never read it; the host needs it now, because a
            // panel request is a bound the content is laid out against and clipped to.
            panel_max_height: tokens.panel_max_height,

            // --- v1's call-site constants, Cupertino's values ---
            trigger_gap: 8.0,
            row_padding_horizontal: 12.0,
            // **Zero, where Material wrote 4.0.** v1 positioned this panel with an
            // inset and no margin at all, so it sat flush against the field. Preserved
            // rather than reconciled — and stated here because the substitution that
            // produced this file copied Material's 4.0, which is why a derived kit's
            // numbers are worth grepping for rather than diffing.
            panel_gap: 0.0,
            chevron_size: 14.0,
            disabled_text_alpha: 38,
            disabled_chevron_alpha: 97,
            disabled_border_alpha: 97,
            selected_row_alpha: 24,
            hover_row_alpha: 8,
            panel_border_alpha: 120,
            panel_shadow_alpha: 20,
            border_width: 1.0,

            // --- resolved theme colours ---
            surface: to_paint(surface),
            on_surface: to_paint(on_surface),
            on_surface_variant: to_paint(theme.colors().on_surface_variant),
            outline: to_paint(outline),
            primary: to_paint(theme.colors().primary),
            text_size: theme.typography().body.medium.font_size,
        }
    }

    /// The control, ready to mount.
    ///
    /// A [`Combobox`] — the engine's — with its index-based callback adapted to this
    /// control's text-based one. The adaptation is the port: v1's two v1 crates differed
    /// here and now they do not, because the difference is one `map`.
    pub fn view(self) -> Combobox {
        let options = self.props.options.clone();
        let on_change = self.on_change.clone();
        let mut combo = Combobox::with_shared(
            self.props.options.clone(),
            self.query.clone(),
            self.pick.clone(),
        )
        .style(self.resolved_style())
        .icons(resolve_icon)
        .placeholder(self.props.placeholder.clone())
        .disabled(self.props.disabled);
        if let Some(on_change) = on_change {
            combo = combo.on_change(move |index| {
                // `None` cannot happen from a row click — the engine only reports an
                // index — but it is handled rather than unwrapped, because a callback
                // that panics is worse than one that says "cleared".
                on_change(index.and_then(|index| options.get(index).cloned()));
            });
        }
        combo
    }
}

impl Default for CupertinoAutocomplete {
    fn default() -> Self {
        Self::new()
    }
}