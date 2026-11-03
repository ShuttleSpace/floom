//! Cupertino sheet, ported to the Floom 2.0 widget API.
//!

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{Insets, Shadow};

use floom_ui_composites::Dialog;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::sheet::{SheetProps, SheetSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

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
pub struct CupertinoSheetStyle {
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

/// Cupertino's sheet.
pub struct CupertinoSheet {
    props: SheetProps,
    /// The shared "is it up" flag.
    ///
    /// Not an `RwSignal`: that is `Rc`-based and cannot cross a `Send + Sync`
    /// slot boundary.
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl CupertinoSheet {
    pub fn new() -> Self {
        let props = SheetProps::default();
        Self {
            // v1 seeded the signal from `props.open`, and the props default is
            open: shared_flag(props.open),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoSheetStyle {
        let theme = self.theme.get();
        let ComponentTokens::Sheet(tokens) = theme.component(ComponentId::Sheet) else {
            unreachable!("ComponentId::Sheet must resolve to SheetTokens")
        };
        CupertinoSheetStyle {
            design_system: "cupertino",
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            // v1's paint: `border_radius(theme.radius().md)` on the top and
            corner_radius: theme.radius().md,
            // v1's style reported the token. For Cupertino the two are both
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

impl Default for CupertinoSheet {
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
    style: CupertinoSheetStyle,
    side: SheetSide,
    child: Option<Child>,
) -> Box<dyn WidgetAny> {
    // v1: `with_alpha(scrim_color, 160)`. The token's own alpha is replaced,
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
fn build_panel(style: CupertinoSheetStyle, side: SheetSide, child: Option<Child>) -> Surface {
    let mut request = LayoutRequest::new().padding(Insets::uniform(f64::from(style.padding)));
    match side {
        // v1: the top and bottom sheets span the width; the side sheets are
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

    if matches!(side, SheetSide::Bottom | SheetSide::Top) {
        panel = panel.radius(style.corner_radius);
    }

    if style.elevated {
        panel = panel.shadow(Shadow {
            color: to_paint(style.shadow_color),
            blur: f64::from(style.shadow_blur),
            spread: f64::from(style.shadow_spread),
            offset_x: 0.0,
            offset_y: 0.0,
        });
    }

    if let Some(child) = child {
        panel = panel.child(child);
    }

    panel
}
