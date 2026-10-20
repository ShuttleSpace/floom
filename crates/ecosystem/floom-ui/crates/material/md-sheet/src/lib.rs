//! Material sheet, ported to the Floom 2.0 widget API.
//!
//! A sheet is a panel anchored to one edge of the window, over a scrim, that
//! the caller shows and hides.
//!
//! ## What the port uses
//!
//! The engine's [`Dialog`] composite, with the scrim and the panel as its
//! card. `Dialog` is the engine's host-modal surface driven by a shared
//! presence flag -- which is exactly v1's `open` / `open_signal` external
//! trigger -- and a sheet is a modal panel, so the two are the same shape.
//!
//! The engine's `BottomSheet` composite was read and not used: it is anchored
//! to the bottom edge only, it carries no scrim, it has no shared presence, and
//! it holds a title and a fixed height. v1's sheet has four sides, a scrim and
//! caller-supplied content, so none of those three things would have survived
//! the move.
//!
//! ## What this kit answers
//!
//! * the panel's colour: `surface`; the content's ink: `on_surface`;
//! * the panel's inner padding, the side sheets' width, the shadow's blur and
//!   spread and the scrim's colour: the tokens;
//! * the panel's corner radius: the token. Material's style and its paint
//!   agreed on it, and they still do;
//! * the scrim's alpha: 160, a v1 literal -- and it *replaces* the scrim
//!   token's own alpha rather than multiplying it, so the token's alpha never
//!   shows;
//! * the shadow's ink: black at 40, a v1 literal.
//!
//! ## The differences that are named rather than reproduced
//!
//! * **The slide-in and the fade.** v1 played a 300ms slide from the anchored
//!   edge and a 300ms fade of the scrim. 2.0 has `AnimationSpec` and the host
//!   drives animations, so this is reachable -- but the sheet's card is built
//!   inside an overlay's content factory, and wiring a presence motion into it
//!   is a separate piece of work. Recorded rather than half-done, as it was for
//!   `md-dialog`.
//! * **The panel's inherited ink.** v1 set `.color(on_surface)` on the panel,
//!   which is what the caller's own child inherited. 2.0's `Text` carries its
//!   own colour, so a caller's child keeps the colour it was given.
//! * **The overlay kind.** The sheet is mounted as the engine's `Dialog`
//!   overlay, so the host's overlay kind says `Dialog`; the enter and exit
//!   motion the host picks follows the kind.
//! * **Closed means unmounted.** v1 hid the sheet with `.hide()` and kept it
//!   in the tree. 2.0's shared presence unmounts it, so nothing of the sheet
//!   is in the tree while it is closed.
//! * **The theme is read when the sheet is built.** v1's panel style re-read
//!   the theme on every style recomputation. 2.0's card is built from colours
//!   captured at `view()` time, because the theme signal is `Rc`-based and
//!   cannot cross the `Send + Sync` slot boundary an overlay's content factory
//!   is.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The accessors.** v1's Material sheet had `props()` and `theme()` but no
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.
//! * **The empty content.** v1 mounted an `Empty` view when no content was
//!   supplied. Here the panel simply has no child. `Empty` had no size, so the
//!   only difference is structural.

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{Insets, Shadow};

use floom_ui_composites::Dialog;
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::sheet::{SheetProps, SheetSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

/// The scrim's alpha. A v1 literal, and it replaces the scrim token's own
/// alpha rather than multiplying it.
const SCRIM_ALPHA: u8 = 160;
/// The elevation shadow's alpha. A v1 literal.
const SHADOW_ALPHA: u8 = 40;

/// The sheet's appearance, in the terms v1 used.
///
/// The fields v1's style did not report but the paint used -- the padding, the
/// scrim, the shadow and the design system -- are added here, because a style
/// that does not say what the widget draws is not the widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSheetStyle {
    /// Added: v1's sheet style named no design system.
    pub design_system: &'static str,
    pub panel_color: Color,
    pub content_color: Color,
    /// The panel's corner radius, as painted.
    pub corner_radius: f32,
    /// The token's corner radius -- what v1's style reported.
    pub token_corner_radius: f32,
    pub side_width: f32,
    pub elevated: bool,
    /// Added: the panel's inner padding.
    pub padding: f32,
    /// Added: the scrim behind the panel.
    pub scrim_color: Color,
    /// Added: the elevation shadow's blur.
    pub shadow_blur: f32,
    /// Added: the elevation shadow's spread.
    pub shadow_spread: f32,
    /// Added: the elevation shadow's ink -- black at 40.
    pub shadow_color: Color,
}

/// Material's sheet.
pub struct MaterialSheet {
    props: SheetProps,
    /// The shared "is it up" flag.
    ///
    /// Not an `RwSignal`: that is `Rc`-based and cannot cross a `Send + Sync`
    /// slot boundary.
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl MaterialSheet {
    pub fn new() -> Self {
        let props = SheetProps::default();
        Self {
            // v1 seeded the signal from `props.open`, and the props default is
            // `false`.
            open: shared_flag(props.open),
            props,
            theme: theme_signal(),
            child: None,
        }
    }

    pub fn side(mut self, side: SheetSide) -> Self {
        self.props.side = side;
        self
    }

    /// Show or close the sheet.
    ///
    /// Writes the shared flag rather than asking the widget, because the widget
    /// is built by [`Self::view`] and a caller holding only this value must
    /// still be able to open it -- which is the whole reason v1 handed out
    /// `open_signal()`.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    /// Whether the sheet is up.
    pub fn is_open(&self) -> bool {
        read_flag(&self.open)
    }

    pub fn elevated(mut self, elevated: bool) -> Self {
        self.props.elevated = elevated;
        self
    }

    /// Supply the sheet's surface content.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// The shared open flag, so an external trigger (a button, a route) can
    /// drive it.
    ///
    /// v1's `open_signal()`, and the reason this whole shape exists.
    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Drive this sheet from a flag somebody else owns.
    ///
    /// v1's `with_open_signal(RwSignal<bool>)`.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self.props.open = read_flag(&self.open);
        self
    }

    pub fn props(&self) -> &SheetProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialSheetStyle {
        let theme = self.theme.get();
        let ComponentTokens::Sheet(tokens) = theme.component(ComponentId::Sheet) else {
            unreachable!("ComponentId::Sheet must resolve to SheetTokens")
        };
        MaterialSheetStyle {
            design_system: "material",
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            // v1's paint: `border_radius(tokens.corner_radius)` on the top and
            // bottom sheets.
            corner_radius: tokens.corner_radius,
            // v1's style reported the token, and for Material the two agree.
            token_corner_radius: tokens.corner_radius,
            side_width: tokens.side_width,
            elevated: self.props.elevated,
            padding: tokens.padding,
            scrim_color: tokens.scrim_color,
            shadow_blur: tokens.shadow_blur,
            shadow_spread: tokens.shadow_spread,
            shadow_color: with_alpha(BLACK, SHADOW_ALPHA),
        }
    }

    /// The sheet, ready to mount.
    ///
    /// The card the host mounts over the viewport:
    ///
    /// ```text
    /// Surface          the scrim: the whole window, tinted black @ 160
    ///  └ Stack         the panel on its edge: end, start, or the sides
    ///     └ Surface    the panel: surface fill, padding, radius, shadow
    ///        └ Child   the caller's content
    /// ```
    pub fn view(self) -> Dialog {
        let style = self.resolved_style();
        let side = self.props.side;
        let child = self.child.clone();
        let open = self.open.clone();

        Dialog::new()
            .with_shared_presence(open)
            .card_with(move || build_card(style, side, child.clone()))
    }
}

impl Default for MaterialSheet {
    fn default() -> Self {
        Self::new()
    }
}

/// The card: the scrim, and the panel anchored to the side.
///
/// A free function taking ownership, for the reason `md-dialog`'s card is: it
/// is two `Fn` levels deep otherwise, and each level would need its own copy
/// of the same values.
fn build_card(
    style: MaterialSheetStyle,
    side: SheetSide,
    child: Option<Child>,
) -> Box<dyn WidgetAny> {
    // v1: `with_alpha(scrim_color, 160)`. The token's own alpha is replaced,
    // not multiplied.
    let scrim = to_paint(with_alpha(style.scrim_color, SCRIM_ALPHA));
    let panel = build_panel(style, side, child);

    Box::new(
        Surface::new()
            .request(LayoutRequest::new().fill())
            .background(scrim)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().fill(),
                move || {
                    // The panel is a value, and the slot is a replayed `Fn`, so
                    // it takes a copy rather than a borrow.
                    let panel = panel.clone();
                    // The stack that puts the panel on its edge: v1's `flex_col`
                    // with `justify_end` for the bottom, `flex_row` with
                    // `justify_start` for the leading side, and so on.
                    let mut stack = match side {
                        SheetSide::Bottom => {
                            Stack::new(Axis::Vertical).align(Align::End, Align::Center)
                        }
                        SheetSide::Top => {
                            Stack::new(Axis::Vertical).align(Align::Start, Align::Center)
                        }
                        SheetSide::Start => {
                            Stack::new(Axis::Horizontal).align(Align::Start, Align::Center)
                        }
                        SheetSide::End => {
                            Stack::new(Axis::Horizontal).align(Align::End, Align::Center)
                        }
                    };
                    stack = stack.request(LayoutRequest::new().fill());
                    stack = stack.child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(panel.clone()),
                    ));
                    Box::new(stack)
                },
            )),
    )
}

/// The panel: the coloured, padded, rounded box the content sits in.
fn build_panel(style: MaterialSheetStyle, side: SheetSide, child: Option<Child>) -> Surface {
    let mut request = LayoutRequest::new().padding(Insets::uniform(f64::from(style.padding)));
    match side {
        // v1: the top and bottom sheets span the width; the side sheets are
        // full height and `side_width` wide.
        SheetSide::Bottom | SheetSide::Top => {
            request = request.width_pct(100.0);
        }
        SheetSide::Start | SheetSide::End => {
            request = request.height_pct(100.0).width(style.side_width);
        }
    }

    let mut panel = Surface::new()
        .request(request)
        .background(to_paint(style.panel_color));

    // v1 rounded the top and bottom sheets only.
    if matches!(side, SheetSide::Bottom | SheetSide::Top) {
        panel = panel.radius(style.corner_radius);
    }

    if style.elevated {
        panel = panel.shadow(Shadow {
            color: to_paint(style.shadow_color),
            blur: f64::from(style.shadow_blur),
            spread: f64::from(style.shadow_spread),
            // v1's `box_shadow_*` wrote no offset.
            offset_x: 0.0,
            offset_y: 0.0,
        });
    }

    if let Some(child) = child {
        panel = panel.child(child);
    }

    panel
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
