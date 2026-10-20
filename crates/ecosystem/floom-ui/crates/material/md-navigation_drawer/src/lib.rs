//! Material navigation drawer, ported to the Floom 2.0 widget API.
//!
//! A navigation drawer is a panel that slides in from one edge of the app
//! over a dimmed scrim, with a header above its content. Clicking the scrim
//! closes it.
//!
//! ## What the port uses
//!
//! The engine's vocabulary, plus the composites that own the two things v1
//! drew by hand:
//!
//! * [`Overlay`] for the layer the drawer lives in;
//! * [`Conditional`] for the panel's *presence*, which is what carries the
//!   motion: a fade for the scrim and a slide for the panel.
//!
//! The two conditionals are siblings, so the panel paints over the scrim and
//! a click inside the panel never reaches the scrim's dismiss.
//!
//! ## What this kit answers
//!
//! * the panel's fill: `surface`; the scrim's: `scrim` at 150 alpha;
//! * the panel's width: the *prop's*, with the token's as the fallback when
//!   the prop is zero -- v1's rule, kept exactly;
//! * the panel's padding: the token's;
//! * the elevation shadow: v1's numbers, which differ per kit.
//!
//! ## The tokens that never showed
//!
//! `NavigationDrawerTokens::width` (300) is read by nobody in practice: the
//! prop's width defaults to 360, and every kit takes the prop's when it is
//! positive. The token is the *fallback*, and the fallback only fires when a
//! caller asks for a width of zero. The port keeps the fallback and names it
//! here.
//!
//! `NavigationDrawerTokens::elevation` (4) is never read at all: every kit
//! takes the elevation from the props, whose default is also 4. The port
//! reports the prop's elevation, as v1 did, and does not pretend the token
//! is in play.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The panel's inherited ink.** v1 set `.color(on_surface)` on the panel,
//!   which is what the caller's own child inherited. 2.0's `Text` carries
//!   its own colour, so a caller's child keeps the colour it was given.
//! * **The panel's click stopped the propagation.** v1's panel was a child of
//!   the scrim, so it had to swallow the click to keep the scrim from
//!   dismissing. Here the two are siblings, and a click inside the panel
//!   never reaches the scrim at all.
//! * **The scrim faded and the panel slid, and the panel faded with the
//!   scrim.** v1's panel was inside the scrim, so the scrim's fade took the
//!   panel with it. Here the panel has its own presence, and the only
//!   channel a presence transition can drive is one, so the panel slides and
//!   the scrim fades.
//! * **The width was published reactively.** v1 registered the drawer's
//!   width with the theme's layout slots from an effect over the open
//!   signal, so the content area re-inset itself whenever the drawer opened
//!   or closed. 2.0's `open` is a `SharedFlag`, which no effect can track,
//!   so the width is published once, when the view is built -- the same
//!   trade the ported footer makes.
//! * **The child is a slot, not a view.** v1's `header(..)` and `child(..)`
//!   took `impl IntoView`; 2.0's take a [`Child`], because a 2.0 widget is
//!   mounted by its parent rather than returned.

use floom_core::{Child, ChildKey, Inset, LayoutRequest, Position, PresenceTransition,
    WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{Insets, Shadow};
use floom_ui_composites::Overlay;
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

/// The scrim's alpha. A v1 literal.
const SCRIM_ALPHA: u8 = 150;
/// The motion's duration, in seconds. v1 wrote 260 milliseconds.
const MOTION_SECS: f64 = 0.26;
/// The shadow's alpha. A v1 literal.
const SHADOW_ALPHA: u8 = 40;

/// The navigation drawer's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialNavigationDrawerStyle {
    pub design_system: &'static str,
    /// The panel's fill.
    pub panel_color: Color,
    /// The ink the panel's children inherited in v1. Nothing inherits it in
    /// 2.0 -- see the module note.
    pub content_color: Color,
    /// The scrim's colour, before the alpha below.
    pub scrim_color: Color,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
    /// The alpha the scrim is painted at. v1's style did not report it.
    pub scrim_alpha: u8,
    /// The shadow's blur: v1 wrote `2 + 3 * elevation`.
    pub shadow_blur: f32,
    /// The shadow's spread. A v1 literal.
    pub shadow_spread: f32,
    /// The shadow's colour. A v1 literal.
    pub shadow_color: Color,
}

/// Material's navigation drawer.
pub struct MaterialNavigationDrawer {
    props: NavigationDrawerProps,
    /// Whether the drawer is open.
    open: SharedFlag,
    header: Option<Child>,
    content: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl MaterialNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: shared_flag(props.open),
            props,
            header: None,
            content: None,
            theme: theme_signal(),
        }
    }

    /// Open or close the drawer.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    /// The panel's width, in dp.
    pub fn width(mut self, width: f32) -> Self {
        self.props.width = width;
        self
    }

    /// The panel's elevation. v1's shadow grew with it.
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    /// Choose the edge the drawer slides in from (`Left` or `Right`).
    pub fn side(mut self, side: NavigationDrawerSide) -> Self {
        self.props.side = side;
        self
    }

    /// The header shown at the top of the drawer.
    pub fn header(mut self, header: Child) -> Self {
        self.header = Some(header);
        self
    }

    /// The drawer's content.
    pub fn child(mut self, child: Child) -> Self {
        self.content = Some(child);
        self
    }

    /// The shared "is the drawer open" flag.
    ///
    /// v1's `open_signal()`, and the reason it cannot be an `RwSignal`: that is
    /// `Rc`-based and cannot cross a `Send + Sync` slot boundary.
    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Drive the drawer from a flag somebody else owns.
    ///
    /// v1's `with_open_signal(RwSignal<bool>)`.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.props.open = read_flag(&open);
        self.open = open;
        self
    }

    /// Whether the drawer is open.
    pub fn is_open(&self) -> bool {
        read_flag(&self.open)
    }

    pub fn props(&self) -> &NavigationDrawerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        MaterialNavigationDrawerStyle {
            design_system: "material",
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            scrim_color: theme.colors().scrim,
            // v1's rule: the prop's width when it is positive, the token's
            // otherwise.
            width: if self.props.width > 0.0 {
                self.props.width
            } else {
                tokens.width
            },
            elevation: self.props.elevation,
            padding: tokens.padding,
            scrim_alpha: SCRIM_ALPHA,
            shadow_blur: 2.0 + 3.0 * f32::from(self.props.elevation),
            shadow_spread: 1.0,
            shadow_color: with_alpha(Color::rgba(0, 0, 0, 255), SHADOW_ALPHA),
        }
    }

    /// The drawer: the layer, the scrim and the panel.
    pub fn view(self) -> Overlay {
        let style = self.resolved_style();
        let side = self.props.side;
        let open = self.open.clone();

        // v1 registered the drawer's width with the theme's layout slots, so
        // the content area could inset itself around the drawer. It did it
        // from an effect over the open signal; 2.0's `open` is a shared flag,
        // which no effect can track, so the width is published once, here.
        floom_core_theme::layout::register_layout(
            match side {
                NavigationDrawerSide::Left => {
                    floom_core_theme::layout::LayoutSlot::NavDrawerLeft
                }
                NavigationDrawerSide::Right => {
                    floom_core_theme::layout::LayoutSlot::NavDrawerRight
                }
            },
            if read_flag(&open) { style.width } else { 0.0 },
        );

        // The scrim: a dimmed layer that dismisses the drawer when it is
        // clicked.
        //
        // `Pressable` is a builder rather than a `Clone` value, so the scrim
        // is built inside the closure that mounts it.
        let scrim_color = to_paint(with_alpha(style.scrim_color, style.scrim_alpha));
        let scrim = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            {
                let dismiss = open.clone();
                move || {
                    let dismiss = dismiss.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().absolute_fill(),
                            move || Box::new(Surface::new().background(scrim_color)),
                        ))
                        .on_click(move || {
                            write_flag(&dismiss, false);
                        }),
                    ) as Box<dyn WidgetAny>
                }
            },
        );

        // The panel: the header over the content, on the panel's own fill.
        let panel_color = to_paint(style.panel_color);
        let shadow = Shadow {
            color: to_paint(style.shadow_color),
            blur: f64::from(style.shadow_blur),
            offset_x: 0.0,
            offset_y: 0.0,
            spread: f64::from(style.shadow_spread),
        };
        let padding = style.padding;
        // The slots are replayed `Fn`s, so each takes a copy rather than the
        // value itself.
        let header = self.header.clone();
        let content = self.content.clone();
        let panel = Surface::new()
            .request(panel_request(side, style.width))
            .padding(Insets::uniform(f64::from(padding)))
            .background(panel_color)
            .shadow(shadow)
            .push(move || {
                // The slot is a replayed `Fn`, and the two inner slots want
                // the header and the content, so each takes a copy.
                let header = header.clone();
                let content = content.clone();
                // v1's `Stack::vertical((header, content))`: the header over
                // the content, eight points apart.
                // The two inner slots are replayed `Fn`s of their own, so
                // the header and the content are copied before they are handed
                // over.
                let header = header.clone();
                let content = content.clone();
                // The two inner slots are replayed `Fn`s of their own, so the
                // header and the content are copied before they are handed over.
                let header = header.clone();
                let content = content.clone();
                let mut column = Stack::new(Axis::Vertical)
                    .gap(8.0)
                    // v1's `items_start()`. The stack's alignment is
                    // `align(main, cross)`, and both axes start at the
                    // beginning here.
                    .align(Align::Start, Align::Start);
                column = column.child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                    move || {
                        match &header {
                            Some(header) => header.build(),
                            None => Box::new(Surface::new()) as Box<dyn WidgetAny>,
                        }
                    },
                ));
                column = column.child(Child::new(
                    ChildKey::index(1),
                    LayoutRequest::new(),
                    move || {
                        match &content {
                            Some(content) => content.build(),
                            None => Box::new(Surface::new()) as Box<dyn WidgetAny>,
                        }
                    },
                ));
                Box::new(column)
            });

        // The layer: the scrim, then the panel over it.
        let scrim_flag = open.clone();
        let panel_flag = open.clone();
        let layer = Stack::new(Axis::Vertical)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().absolute_fill(),
                move || {
                    Box::new(presence(
                        &scrim_flag,
                        // v1's opacity animation on the scrim.
                        PresenceTransition::fade(MOTION_SECS),
                        scrim.clone(),
                    )) as Box<dyn WidgetAny>
                },
            ))
            .child(Child::new(
                ChildKey::index(1),
                LayoutRequest::new().absolute_fill(),
                move || {
                    // The slot is a replayed `Fn`, so the panel is copied.
                    let panel = panel.clone();
                    let from = match side {
                        NavigationDrawerSide::Left => -style.width,
                        NavigationDrawerSide::Right => style.width,
                    };
                    Box::new(presence(
                        &panel_flag,
                        // v1's slide: the panel starts a whole width outside
                        // and travels in.
                        PresenceTransition::slide_x(MOTION_SECS, from),
                        Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
                            let panel = panel.clone();
                            Box::new(panel) as Box<dyn WidgetAny>
                        }),
                    )) as Box<dyn WidgetAny>
                },
            ));

        Overlay::new().content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(layer.clone()),
        ))
    }
}

impl Default for MaterialNavigationDrawer {
    fn default() -> Self {
        Self::new()
    }
}

/// A presence that starts where the drawer is.
///
/// `Conditional::new(false)` deliberately starts "not yet seen", so the first
/// update reads the flag as a *change*. For a drawer that starts closed that
/// would mount the panel and play the exit -- a drawer fading out on the
/// first frame. `sync_when` is the engine's door for a widget that already
/// knows the answer, and a drawer that starts open still plays its slide,
/// which is v1's `run_on_create(true)`.
fn presence(open: &SharedFlag, transition: PresenceTransition, child: Child) -> Conditional {
    let mut conditional = Conditional::new(false).when_flag(open.clone());
    if !read_flag(open) {
        conditional.sync_when(false);
    }
    conditional.presence(transition).show(child)
}

/// The panel's own box: v1's absolute panel, pinned to the chosen edge.
fn panel_request(side: NavigationDrawerSide, width: f32) -> LayoutRequest {
    let mut request = LayoutRequest::new().width(width).height_pct(100.0);
    request.position = Some(Position::Absolute);
    request.inset = Some(Inset {
        left: match side {
            NavigationDrawerSide::Left => Some(0.0),
            NavigationDrawerSide::Right => None,
        },
        top: Some(0.0),
        right: match side {
            NavigationDrawerSide::Left => None,
            NavigationDrawerSide::Right => Some(0.0),
        },
        bottom: None,
    });
    request
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
