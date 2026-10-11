//! Material 3 expansion panels, ported to the Floom 2.0 widget API.
//!
//! The **stateful** reference port, alongside `md-card` (surface) and `md-button`
//! (interactive). It is here because a component that *changes* after mount
//! exercises things the other two never touch: a value every widget in a subtree
//! has to agree on, a body that mounts and unmounts with it, and a header that
//! must survive its own toggle so keyboard focus is not thrown away by the act of
//! activating it.
//!
//! # What changed from v1
//!
//! v1 held the state in an `RwSignal<Vec<bool>>` that the panels read inside
//! their `dyn_view` closures, and closed over each panel's index:
//!
//! ```ignore
//! let expanded: RwSignal<Vec<bool>> = RwSignal::new(vec![false; n]);
//! // per panel:
//! .on_event(listener::Click, move |_, _| { expanded.set(all); … })
//! ```
//!
//! 2.0 has no reactive read during render, so the state machine moved *into* the
//! engine as [`floom_widgets::Accordion`] — a widget that owns the open set and
//! hands each panel a shared handle, which is what lets the click that lands deep
//! in a header reach a panel body its sibling owns. All three kits ship this
//! component and their copies differ only in tokens and glyph names, so it is
//! promoted once rather than written three times.
//!
//! Three further translations, each load-bearing:
//!
//! * **The chevron is a `VectorIcon`,** not SVG markup. v1 handed
//!   `svg(move || …)` a *closure* that re-read the open flag on every use; 2.0
//!   resolves the glyph up front and hands [`Accordion::panel`] a builder that is
//!   told which way the panel is (the same pattern `md-icon` uses).
//! * **The header's clickable row is a [`floom_widgets::Pressable`].** v1 put
//!   `.on_event(listener::Click, ..)` on any view; 2.0's only clickable widget was
//!   `Button`, and `Button` *is* its label — it cannot carry a title plus a
//!   chevron.
//! * **The padding became a margin on the content's slot.** v1 wrote
//!   `Container::new(content).style(|s| s.padding_horiz(..).padding_vert(..))`.
//!   2.0's `Container` is a leaf with nothing inside it, so the inset belongs on
//!   the content's own `LayoutRequest::margin`.
//!
//! # What is deliberately *not* here
//!
//! **A height transition on the reveal.** v1 has a real one — `md-expand_transition`
//! tweens `height: 0 → computed` — and this port cannot reproduce it: 2.0's
//! implicit layer only carries compositor channels (opacity / translate / scale),
//! and the host re-runs layout only when layout is dirty
//! (`WidgetHost::frame` → `if self.layout_dirty`), so a box cannot change height
//! per frame. [`MdExpansionPanels::content_transition`] therefore offers a
//! compositor-only transition and the panel's box is its final height from the
//! first frame. This is recorded in `docs/2.0/MOTION-GAP.md` (B4, "size") and in
//! `docs/porting-floem-ui-to-2.0.md`.
//!
//! See `docs/porting-floem-ui-to-2.0.md`.

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_core::PresenceTransition;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Accordion, AccordionStyle, VectorIcon};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::expansion_panels::ExpansionPanelsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

use floom_widget_md_icon::{MD_ICON_VIEW_BOX, MdIcon};

/// The two glyphs a panel's indicator shows, resolved from the theme.
///
/// Kept as a pair of widgets rather than as icon *names* so the `Accordion` never
/// learns what a Material icon is: it is handed a builder and told which way the
/// panel is. That is the same seam `VectorIcon` established — the kit resolves,
/// the engine draws.
#[derive(Debug, Clone, PartialEq)]
pub struct MdPanelIndicator {
    pub collapsed: VectorIcon,
    pub expanded: VectorIcon,
}

/// Resolved, theme-agnostic style for a Material expansion panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdExpansionPanelsStyle {
    pub header_height: f32,
    pub header_padding: f32,
    pub content_padding: f32,
    pub content_indent: f32,
    pub gap: f32,
    pub radius: f32,
    pub border_width: f32,
    pub border_color: PaintColor,
    pub title_color: PaintColor,
    pub title_size: f32,
}

pub struct MdExpansionPanels {
    props: ExpansionPanelsProps,
    theme: RwSignal<MultiTheme>,
    /// One entry per declared panel, in declaration order.
    panels: Vec<PanelItem>,
    /// v1 stored the callback in `ExpansionPanelsProps`, whose field is a
    /// `Box<dyn Fn(..)>` — not shareable, and 2.0 needs it inside a `Child`
    /// builder closure that is `Send + Sync`. So the kit holds its own `Arc` and
    /// the shared props keep carrying the design-system-neutral shape.
    on_change: Option<Arc<dyn Fn(usize, bool) + Send + Sync>>,
    transition: Option<PresenceTransition>,
}

struct PanelItem {
    title: String,
    /// v1 took `impl Fn() -> AnyView + 'static`. 2.0's `Child` builder closure is
    /// `Send + Sync + 'static`, so the callback is shared as an `Arc` and rebuilt
    /// into the tree per call — the same constraint `md-button`'s `on_click`
    /// documents.
    content: Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync>,
}

impl MdExpansionPanels {
    pub fn new() -> Self {
        Self {
            props: ExpansionPanelsProps::default(),
            theme: theme_signal(),
            panels: Vec::new(),
            on_change: None,
            transition: None,
        }
    }

    /// Adds a panel. `content` builds the body, which is mounted only while the
    /// panel is open.
    pub fn panel(
        mut self,
        title: impl Into<String>,
        content: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.panels.push(PanelItem {
            title: title.into(),
            content: Arc::new(content),
        });
        self
    }

    /// Allow more than one panel open at a time. Off by default, which is the
    /// accordion rule.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    /// Called with `(panel_index, expanded)` whenever a panel's state changes
    /// from a click.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`.
    pub fn on_change(
        mut self,
        callback: impl Fn(usize, bool) + Send + Sync + 'static,
    ) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The shared props, for a caller that reads the design-system-neutral shape.
    ///
    /// `on_change` is deliberately absent from what it reports: the kit holds that
    /// as an `Arc` of its own (see the field's note), and `ExpansionPanelsProps`
    /// is the v1 carrier of a `Box<dyn Fn(..)>` that 2.0 cannot use.
    pub fn props(&self) -> ExpansionPanelsProps {
        ExpansionPanelsProps {
            multiple: self.props.multiple,
            on_change: None,
        }
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn panel_count(&self) -> usize {
        self.panels.len()
    }

    pub fn titles(&self) -> Vec<&str> {
        self.panels.iter().map(|p| p.title.as_str()).collect()
    }

    /// The theme's expansion-panel tokens, resolved.
    ///
    /// Exposed because `md-expand_transition` and anything composing a panel
    /// needs the same numbers, and restating them is how two components drift.
    pub fn resolved_style(&self) -> MdExpansionPanelsStyle {
        let theme = self.theme.get();
        let ComponentTokens::ExpansionPanels(tokens) = theme.component(ComponentId::ExpansionPanels)
        else {
            unreachable!("ComponentId::ExpansionPanels must resolve to ExpansionPanelsTokens")
        };
        MdExpansionPanelsStyle {
            header_height: tokens.header_height,
            header_padding: tokens.content_padding,
            content_padding: tokens.content_padding,
            content_indent: tokens.indent,
            gap: tokens.gap,
            radius: tokens.radius,
            border_width: tokens.border,
            // v1: `border_color(with_alpha(t.colors().outline, 30))`.
            border_color: to_paint(with_alpha(theme.colors().outline, 30)),
            // v1 drew the title with `Label::new(..)`, which takes the default text
            // colour — `on_surface`.
            title_color: to_paint(theme.colors().on_surface),
            title_size: LABEL_FONT_SIZE,
        }
    }

    /// The two indicator glyphs, at the token's size and the theme's muted ink.
    pub fn indicator(&self) -> MdPanelIndicator {
        let color = to_paint(self.theme.get().colors().on_surface_variant);
        MdPanelIndicator {
            collapsed: MdIcon::new("chevron_right")
                .with_theme(self.theme.get())
                .size(self.icon_size())
                .resolve()
                .color(color),
            expanded: MdIcon::new("expand_more")
                .with_theme(self.theme.get())
                .size(self.icon_size())
                .resolve()
                .color(color),
        }
    }

    /// The expand/collapse indicator's edge length, from the tokens.
    pub fn icon_size(&self) -> f32 {
        let theme = self.theme.get();
        let ComponentTokens::ExpansionPanels(tokens) = theme.component(ComponentId::ExpansionPanels)
        else {
            unreachable!("ComponentId::ExpansionPanels must resolve to ExpansionPanelsTokens")
        };
        tokens.icon_size
    }

    /// The layout request a panel's content should be mounted with.
    ///
    /// v1 wrote this padding on the box that held the body; 2.0's `Container` is a
    /// leaf, so the inset is a margin on the body's own slot. Handed out rather
    /// than applied because the body is mounted by the *accordion*, not by the
    /// panel — a caller that wants its own layout asks here instead of
    /// restating the tokens.
    pub fn content_layout(&self) -> LayoutRequest {
        LayoutRequest::new().margin_horiz(f64::from(
            self.resolved_style().content_padding + self.resolved_style().content_indent,
        ))
    }

    /// The engine view box every Material glyph is authored against.
    pub const fn icon_view_box() -> floom_types::Rect {
        MD_ICON_VIEW_BOX
    }

    /// A compositor-only transition for the reveal.
    ///
    /// Optional, and it does **not** animate the panel's height — see the module
    /// note. Left unset, the body mounts and unmounts instantly, which is what
    /// v1's `md-expansion-panels` did.
    pub fn content_transition(mut self, transition: PresenceTransition) -> Self {
        self.transition = Some(transition);
        self
    }

    /// The panels as one 2.0 widget, ready to mount with `Child::new(..)`.
    ///
    /// This is the whole port: resolve the theme (the kit's job) and hand back a
    /// widget. There is no `view()` terminal call, because 2.0 widgets are not
    /// composed — the *parent* declares them.
    pub fn resolve(&self) -> Accordion {
        let style = self.resolved_style();
        let indicator = self.indicator();
        let mut accordion = Accordion::new()
            .multiple(self.props.multiple)
            .style(AccordionStyle {
                header_height: style.header_height,
                header_padding: style.header_padding,
                content_padding: style.content_padding,
                content_indent: style.content_indent,
                gap: style.gap,
                title_color: style.title_color,
                title_size: style.title_size,
                border_width: style.border_width,
                border_color: style.border_color,
                radius: style.radius,
            });

        for panel in &self.panels {
            let (collapsed, expanded) = (indicator.collapsed.clone(), indicator.expanded.clone());
            accordion = accordion.panel(
                panel.title.clone(),
                move |open| {
                    Box::new(if open { expanded.clone() } else { collapsed.clone() })
                },
                {
                    let content = panel.content.clone();
                    move || content()
                },
            );
        }

        if let Some(callback) = &self.on_change {
            accordion = accordion.on_change({
                let callback = callback.clone();
                move |index, open| callback(index, open)
            });
        }
        if let Some(transition) = self.transition {
            accordion = accordion.content_transition(transition);
        }
        accordion
    }
}

impl Default for MdExpansionPanels {
    fn default() -> Self {
        Self::new()
    }
}

/// v1's `Label` drew at the controls' shared label size; kept here so the header
/// title does not silently change size when a component is ported.
const LABEL_FONT_SIZE: f32 = 14.0;

/// Design-system colour -> engine colour. Both are plain `{r, g, b, a}` byte
/// structs, so this is a field copy.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}