//! Cupertino list item, ported to the Floom 2.0 widget API.
//!
//! ## Why this crate is now a token translation
//!
//! **90 of 107 substantive lines** are identical across the three kits' v1
//! `list_item`s, and the row's whole structure is in that shared part. So the shape
//! went to the engine as [`floom_ui_composites::ListItemRow`]; what is left here is
//! Cupertino's five numbers, its two colours, and the title's font weight.
//!
//! ## The one place this kit is *not* like its two siblings
//!
//! Material is the only kit that reads its own token table for the row's height and
//! font sizes:
//!
//! | | `min_height` | `title_font_size` | `subtitle_font_size` |
//! |---|---|---|---|
//! | Material | `tokens.*` | `tokens.*` | `tokens.*` |
//! | Cupertino | `44.0` literal | `17.0` literal | `15.0` literal` |
//! | Fluent | `40.0` literal | `14.0` literal | `12.0` literal |
//!
//! and the token table says `56.0 / 16.0 / 14.0` for *every* kit
//! (`ListItemTokens::default()`), so the other two disagree with the tokens they
//! have. That is preserved rather than fixed: see the note on the engine widget.
//!
//! ## What the port changed
//!
//! * `min_height(..)` needed a capability that did not exist. 2.0's
//!   `LayoutRequest` had `width`/`height` — both *definite* sizes — and nothing for
//!   a floor, so a row that must be at least a touch target but may grow with its
//!   content had nowhere to say so. `min_width`/`min_height` were added, with a
//!   host-level suite in `floom-app/tests/min_size.rs` that proves the floor holds
//!   *and* that content taller than it still grows.
//! * `Label::new(..)` is [`floom_widgets::Text`], and `.style(|s| s.color(..))`
//!   became `.color(..)` on the widget. The title's `font_weight` came from the
//!   typography scale, not the list-item tokens, and is resolved here.
//! * `.action(move || if !disabled { .. })` became [`Pressable`] inside the engine
//!   widget, whose `disabled` gate covers the keyboard as well as the pointer.
//!   v1's in-closure guard covered only the pointer.
//! * `ListItemProps::nav` is **never read** by v1's `view()`. The prop's own doc
//!   claims "extra trailing affordance space + hover affordance", neither of which
//!   exists. Kept unread, and recorded here rather than quietly dropped: the same
//!   decision is recorded for `ToolbarTokens::height`.

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};

use floom_ui_composites::{SlotBuilder};
use floom_ui_composites::{ListItemRow, ListItemStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::list_item::ListItemProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

/// The theme's weight enum -> the engine's.
///
/// v1 called `FontWeight::to_floom()`, which returns a **`parley::style::FontWeight`**
/// — a third type with the same name, which is what v1's `.font_weight(..)` took.
/// 2.0's `Text::weight` takes `floom_types::FontWeight`, so the two have to meet
/// somewhere, and going through parley's numeric weight would mean reaching into a
/// dependency's internals from a kit crate to avoid naming nine variants.
///
/// Nine variants is not much, and naming them makes the mapping checkable. One
/// difference from v1, recorded rather than inherited:
///
/// * v1's `to_floom()` sends **`Black` to `EXTRA_BOLD` (800)**, because parley's
///   table has no 900. Here `Black` maps to `floom_types::Black` (900).
///
/// That is a bug fix, not a translation, and it is unverifiable from here: no kit
/// reads `Black` at any call site, so no ported row can observe the difference. It
/// is the same `to_floom()` mapping the porting plan already records as unfixed, and
/// fixing it here rather than in `to_floom()` would leave the two disagreeing — so
/// this is deliberately the *only* place the two answers differ, and it is noted
/// rather than quietly applied.
pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        // v1's name for 400 is `NORMAL`; the theme's is `Regular`.
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        // See the note above: v1 sent this to 800.
        ThemeFontWeight::Black => FontWeight::Black,
    }
}


/// Material's list item: a band with an optional leading slot, a title/subtitle
/// column, and an optional trailing slot.
///
/// Stateless in v1 and stateless here — the only state in the subtree is the
/// engine's ripple phase, which belongs to the engine widget.
pub struct CupertinoListItem {
    props: ListItemProps,
    theme: RwSignal<MultiTheme>,
    leading: Option<SlotBuilder>,
    trailing: Option<SlotBuilder>,
    /// The click callback in the shape 2.0 needs.
    ///
    /// `ListItemProps::on_click` is `Option<Box<dyn Fn()>>` — a v1 type, shared by
    /// all three kits and deliberately unchanged here — and the engine's slot
    /// requires `Send + Sync`. So the callback is stored twice: the v1-shaped `Box`
    /// so `props()` reports what v1 reported, and an `Arc<dyn Fn() + Send + Sync>`
    /// for the engine, from one closure. The last v1-shaped field in this crate.
    on_click: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl CupertinoListItem {
    pub fn new() -> Self {
        Self {
            props: ListItemProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            leading: None,
            trailing: None,
            on_click: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.props.subtitle = Some(subtitle.into());
        self
    }

    /// The slot before the text column.
    ///
    /// v1 took `impl IntoView`; 2.0 takes a replayed builder, so a caller with a
    /// widget in hand writes `move || Box::new(w)`.
    pub fn leading(mut self, leading: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static) -> Self {
        self.leading = Some(Arc::new(leading));
        self
    }

    /// The slot after the text column.
    pub fn trailing(mut self, trailing: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static) -> Self {
        self.trailing = Some(Arc::new(trailing));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// v1's `nav(..)`, which it stored and never read.
    ///
    /// Kept because it is part of the props contract callers write, and kept
    /// **unread** because reading it would mean inventing the "extra trailing
    /// affordance space" the prop's doc comment promises and v1 never delivered.
    pub fn nav(mut self, nav: bool) -> Self {
        self.props.nav = nav;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_click = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_click = Some(shared);
        self
    }

    pub fn props(&self) -> &ListItemProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Cupertino's numbers, as the engine's style.
    ///
    /// Every field is a read v1 also made, at the same moment. Three of them are
    /// literals here rather than token reads, because that is what v1 did — see the
    /// table at the top of this file, and `floom_ui_composites::ListItemRow`'s note on why
    /// the disagreement with `ListItemTokens` is preserved rather than fixed.
    pub fn resolved_style(&self) -> ListItemStyle {
        let theme = self.theme.get();
        let ComponentTokens::ListItem(tokens) = theme.component(ComponentId::ListItem) else {
            unreachable!("ComponentId::ListItem must resolve to ListItemTokens")
        };
        ListItemStyle {
            // The three numbers v1 hard-coded here, and the reason this kit does
            // not agree with its own token table: `ListItemTokens::default()` says
            // `56.0 / 16.0 / 14.0` for every kit, and Cupertino ignores it for these three.
            min_height: 44.0,
            padding_horizontal: f64::from(tokens.padding_horizontal),
            gap: tokens.gap,
            // v1 wrote `.gap(4.0)` at the call site rather than reading a token; the
            // engine takes it as a field so the literal is visible instead of buried.
            text_gap: 4.0,
            title_size: 17.0,
            // v1 read this from the *typography* scale, not from `ListItemTokens`:
            // `typography().title.medium.font_weight`.
            title_weight: to_engine_weight(theme.typography().title.medium.font_weight),
            title_color: to_paint(theme.colors().on_surface),
            subtitle_size: Some(15.0),
            subtitle_color: to_paint(theme.colors().on_surface_variant),
            background: to_paint(theme.colors().surface),
        }
    }

    /// The row, ready to mount.
    pub fn view(self) -> ListItemRow {
        let style = self.resolved_style();
        let mut row = ListItemRow::new(self.props.title.clone())
            .style(style)
            .disabled(self.props.disabled);

        if let Some(subtitle) = &self.props.subtitle {
            row = row.subtitle(subtitle.clone());
        }
        if let Some(leading) = &self.leading {
            row = row.leading({
                let leading = leading.clone();
                move || leading()
            });
        }
        if let Some(trailing) = &self.trailing {
            row = row.trailing({
                let trailing = trailing.clone();
                move || trailing()
            });
        }
        if let Some(on_click) = &self.on_click {
            row = row.on_click({
                let on_click = on_click.clone();
                move || on_click()
            });
        }
        row
    }
}

impl Default for CupertinoListItem {
    fn default() -> Self {
        Self::new()
    }
}
