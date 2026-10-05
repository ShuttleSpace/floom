//! Material 3 item group, ported to the Floom 2.0 widget API.
//!
//! This is a **token-to-style wrapper**, and the shortest port in the library —
//! deliberately. Where `md-toolbar` kept its own composition and `md-footer` kept
//! its own layout registration, this component's whole job turned out to be the
//! state machine, and the state machine is not this component's:
//!
//! `md-item_group`, `cupertino-item_group` and `fluent-item_group` are 209, 208
//! and 208 lines with **123 of 149** substantive lines identical. Everything they
//! share — the shared selection state, the single/multiple/mandatory click rules,
//! the row's two appearances, the click target, and the padding that is `pad`
//! across and `pad / 2` down — is [`floom_widgets::SelectGroup`], promoted into
//! the engine for the same reason [`floom_widgets::Accordion`] was.
//!
//! What is left here is the part that genuinely differs per design system: which
//! tokens, and which colours mean "chosen".
//!
//! # What changed from v1
//!
//! * **Two signals became one shared state.** v1 held `single: RwSignal<usize>`
//!   *and* `multi: RwSignal<Vec<usize>>`, kept in step by hand, and read
//!   `multi.contains(&i)` to decide a row's appearance. A value that changes after
//!   mount cannot be held in a signal and read during render in 2.0, so the
//!   selection is one `Arc<Mutex<Vec<bool>>>` and the two accessors
//!   ([`MdItemGroup::selected`] and [`MdItemGroup::selected_multi`]) are derived
//!   from it rather than stored beside it.
//! * **`cursor(CursorStyle::Pointer)` is gone.** 2.0's `PaintContext` has no
//!   cursor vocabulary, so the pointer is not carried over; the row is still a
//!   `Pressable`, so it is still activatable by keyboard, which the pointer was
//!   not. Recorded rather than worked around.
//! * **`hide()` on an empty group has no 2.0 equivalent**, so an empty group is a
//!   stack with no children. See `SelectGroup`'s doc.
//! * **Semantics are an addition, not a translation.** v1's rows were a
//!   `Container` with a click handler: invisible to a screen reader. Rows here are
//!   radios (single mode) or checkboxes (multiple).

use floom_legacy_reactive::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets};
use floom_widgets::{SelectGroup, SelectGroupStyle, SelectionMode};

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ItemGroupTokens};
use floom_core_theme::props::item_group::ItemGroupProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// A builder for one row's content.
///
/// An [`std::sync::Arc`] because a 2.0 slot is a replayed `Fn`: every rebuild
/// needs its own copy to move into it.
pub type RowBuilder =
    std::sync::Arc<dyn Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static>;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn item_group_tokens(theme: &MultiTheme) -> ItemGroupTokens {
    match theme.component(ComponentId::ItemGroup) {
        ComponentTokens::ItemGroup(t) => t,
        _ => unreachable!("ComponentId::ItemGroup must resolve to ItemGroupTokens"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdItemGroupStyle {
    pub design_system: &'static str,
    pub surface_variant: Color,
    pub on_surface: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub primary: Color,
    pub outline: Color,
    pub gap: f32,
    pub pad: f32,
    pub radius: f32,
}

/// One row, as it was declared.
struct ItemRow {
    label: Option<String>,
    content: RowBuilder,
}

pub struct MdItemGroup {
    props: ItemGroupProps,
    theme: RwSignal<MultiTheme>,
    rows: Vec<ItemRow>,
    /// The chosen row before mount. v1's `selected(usize)` set both signals; here
    /// it seeds the one shared state.
    initial: Option<usize>,
}

impl MdItemGroup {
    pub fn new() -> Self {
        Self {
            props: ItemGroupProps::default(),
            theme: theme_signal(),
            rows: Vec::new(),
            initial: None,
        }
    }

    /// Append a named row.
    pub fn row(
        mut self,
        label: impl Into<String>,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(ItemRow {
            label: Some(label.into()),
            content: std::sync::Arc::new(content),
        });
        self
    }

    /// Append a row with no accessible name.
    pub fn row_unlabelled(
        mut self,
        content: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.rows.push(ItemRow {
            label: None,
            content: std::sync::Arc::new(content),
        });
        self
    }

    /// Choose a row before mount. v1's `selected(usize)`.
    pub fn selected(mut self, selected: usize) -> Self {
        self.initial = Some(selected);
        self
    }

    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn mandatory(mut self, mandatory: bool) -> Self {
        self.props.mandatory = mandatory;
        self
    }

    pub fn props(&self) -> &ItemGroupProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdItemGroupStyle {
        let theme = self.theme.get();
        let tokens = item_group_tokens(&theme);
        MdItemGroupStyle {
            design_system: "material",
            surface_variant: theme.colors().surface_variant,
            on_surface: theme.colors().on_surface,
            primary_container: theme.colors().primary_container,
            on_primary_container: theme.colors().on_primary_container,
            primary: theme.colors().primary,
            outline: theme.colors().outline,
            gap: tokens.gap,
            pad: tokens.pad,
            radius: tokens.radius,
        }
    }

    /// The engine's style, which is the whole translation.
    ///
    /// The row's two appearances are named here rather than at the call site
    /// because "chosen" means a *pair* of colours plus a third for the outline, and
    /// a caller that styled a chosen row's box while leaving its label in the
    /// unchosen ink would have a bug the compiler cannot see.
    pub fn select_group_style(&self) -> SelectGroupStyle {
        let style = self.resolved_style();
        SelectGroupStyle {
            gap: style.gap,
            pad: style.pad,
            radius: style.radius,
            // v1 wrote `.border(1.0)` unconditionally — a fixed 1px, not a token.
            border_width: 1.0,
            background: to_paint(style.surface_variant),
            foreground: to_paint(style.on_surface),
            border_color: to_paint(style.outline),
            selected_background: to_paint(style.primary_container),
            selected_foreground: to_paint(style.on_primary_container),
            selected_border_color: to_paint(style.primary),
        }
    }

    /// The colour a row's *content* should be drawn in.
    ///
    /// v1 wrote `.color(fg)` on the row's box. 2.0's `Surface` paints a fill and
    /// only the child knows it is text, so the ink is a question for the caller —
    /// the same answer `md-footer` gives, and the third component to need it.
    pub fn foreground(&self, selected: bool) -> PaintColor {
        if selected {
            to_paint(self.resolved_style().on_primary_container)
        } else {
            to_paint(self.resolved_style().on_surface)
        }
    }

    /// The row's padding, for a caller that wants it.
    ///
    /// v1 wrote `padding_horiz(pad).padding_vert(pad / 2.0)`, which is *not* a
    /// uniform inset; [`SelectGroup`] applies the same asymmetry and this reports
    /// it rather than rounding it to something tidier.
    pub fn padding(&self) -> Insets {
        let pad = f64::from(self.resolved_style().pad);
        Insets {
            left: pad,
            right: pad,
            top: pad / 2.0,
            bottom: pad / 2.0,
        }
    }

    /// The group as an engine widget: the rows, the shared selection, and the
    /// click rules.
    ///
    /// `on_change_single` and `on_change_multi` are both reported: v1 had two
    /// callbacks because it had two signals, and a caller that only cares about one
    /// mode should not have to know which. [`Self::selection`] is the one to read
    /// after mount.
    pub fn view(
        self,
        on_change_single: Option<impl Fn(usize) + Send + Sync + 'static>,
        on_change_multi: Option<impl Fn(Vec<usize>) + Send + Sync + 'static>,
    ) -> SelectGroup {
        let mode = if self.props.multiple {
            SelectionMode::Multiple
        } else {
            SelectionMode::Single
        };
        let single = on_change_single.map(std::sync::Arc::new);
        let multi = on_change_multi.map(std::sync::Arc::new);

        let mut group = SelectGroup::new()
            .mode(mode)
            .mandatory(self.props.mandatory)
            .style(self.select_group_style());
        if let Some(index) = self.initial {
            group = group.selected([index]);
        }
        group = group.on_change(move |next| {
            // Both callbacks, in the order a caller would expect: the single one
            // with the most recently chosen row, then the full set.
            if let Some(single) = &single {
                if let Some(last) = next.last() {
                    single(*last);
                }
            }
            if let Some(multi) = &multi {
                multi(next.clone());
            }
        });
        for row in self.rows {
            // Wrapped rather than passed as the `Arc` itself: the slot wants an
            // `Fn`, and an `Arc<dyn Fn>` is one by being callable. This is also why
            // the alias above exists — a rebuild needs its own handle to move into
            // the slot.
            let content = row.content;
            group = match row.label {
                Some(label) => group.row(label, move || content()),
                None => group.row_unlabelled(move || content()),
            };
        }
        group
    }
}

/// The chosen row, the way v1's `selected_signal` reported it.
///
/// A snapshot rather than a signal: the selection is shared state now, and a caller
/// that wants to *observe* it reads this from the widget it holds rather than
/// holding a second copy of the truth.
pub fn selected(group: &SelectGroup) -> Option<usize> {
    group.selection().last().copied()
}

/// The chosen rows, the way v1's `selected_multi_signal` reported them.
pub fn selected_multi(group: &SelectGroup) -> Vec<usize> {
    group.selection()
}

impl Default for MdItemGroup {
    fn default() -> Self {
        Self::new()
    }
}
