//! Cupertino navigation drawer, ported to the Floom 2.0 widget API.
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
//! * the panel's fill: `surface` at 235 alpha -- v1's translucent panel;
//! * the panel's width: the *prop's*, and only the prop's: v1 took
//!   `props.width.max(0.0)` with no token fallback, which is a different rule
//!   from the other two kits' and is kept as it is;
//! * the panel's padding: the token's;
//! * the panel's hairline: half a point of `on_surface` at 30 alpha, on the
//!   edge the drawer is *not* on;
//! * the elevation shadow: v1's numbers, which differ per kit.
//!
//! ## The tokens that never showed
//!
//! `NavigationDrawerTokens::width` (300) is read by nobody in practice: the
//! prop's width defaults to 360, and this kit takes the prop's whatever it
//! is. The token is the *fallback* in the other two kits and nothing at all
//! here; the port keeps this kit's rule and names it.
//!
//! `NavigationDrawerTokens::elevation` (4) is never read at all: every kit
//! takes the elevation from the props, whose default is also 4. The port
//! reports the prop's elevation, as v1 did, and does not pretend the token
//! is in play.
//!
//! ## The style fields v1 never used
//!
//! v1's Cupertino style reported `header_color` and `content_color`, and the
//! view used neither: the header is the caller's own widget, and the panel's
//! ink was `on_surface`, not `on_surface_variant`. The port keeps both
//! fields -- a style that omits a value the kit had is indistinguishable from
//! one that chose to leave it at a default -- and names them here.
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
use floom_types::{Edges, Insets, Shadow};
use floom_ui_composites::Overlay;
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

/// The scrim's alpha. A v1 literal.
const SCRIM_ALPHA: u8 = 150;
/// The panel's alpha. A v1 literal.
const PANEL_ALPHA: u8 = 235;
/// The hairline's alpha. A v1 literal.
const BORDER_ALPHA: u8 = 30;
/// The hairline's width. A v1 literal.
const BORDER_WIDTH: f32 = 0.5;
/// The shadow's alpha. A v1 literal.
const SHADOW_ALPHA: u8 = 50;
/// The motion's duration, in seconds. v1 wrote 260 milliseconds.
const MOTION_SECS: f64 = 0.26;

/// The navigation drawer's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoNavigationDrawerStyle {
    pub design_system: &'static str,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
    /// The panel's fill, before the alpha below.
    pub background: Color,
    /// The ink v1 gave the header. The header is the caller's own widget, so
    /// nothing uses it -- see the module note.
    pub header_color: Color,
    /// The ink v1 named for the content. The view never used it -- see the
    /// module note.
    pub content_color: Color,
    /// The scrim's colour, at the alpha v1 painted it.
    pub scrim_color: Color,
    /// The alpha the panel is painted at. v1's style did not report it.
    pub panel_alpha: u8,
    /// The hairline's width. A v1 literal.
    pub border_width: f32,
    /// The hairline's colour. A v1 literal.
    pub border_color: Color,
    /// The shadow's blur: v1 wrote `elevation * 4`.
    pub shadow_blur: f32,
    /// The shadow's spread. A v1 literal.
    pub shadow_spread: f32,
    /// The shadow's colour. A v1 literal.
    pub shadow_color: Color,
}

/// Cupertino's navigation drawer.
pub struct CupertinoNavigationDrawer {
    props: NavigationDrawerProps,
    /// Whether the drawer is open.
    open: SharedFlag,
    header: Option<Child>,
    content: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: shared_flag(props.open),
            props,
            header: None,
            content: None,
            // v1's Cupertino kit took a snapshot of its own design system.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        CupertinoNavigationDrawerStyle {
            design_system: "cupertino",
            // v1 took the prop's width and clamped it at zero: no token
            // fallback at all.
            width: self.props.width.max(0.0),
            elevation: self.props.elevation,
            padding: tokens.padding,
            background: theme.colors().surface,
            header_color: theme.colors().on_surface,
            content_color: theme.colors().on_surface_variant,
            scrim_color: with_alpha(theme.colors().scrim, SCRIM_ALPHA),
            panel_alpha: PANEL_ALPHA,
            border_width: BORDER_WIDTH,
            border_color: with_alpha(theme.colors().on_surface, BORDER_ALPHA),
            shadow_blur: f32::from(self.props.elevation) * 4.0,
            shadow_spread: 0.0,
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
        let scrim_color = to_paint(style.scrim_color);
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
        let panel_color = to_paint(with_alpha(style.background, style.panel_alpha));
        let border_color = to_paint(style.border_color);
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
        let mut panel = Surface::new()
            .request(panel_request(side, style.width))
            .padding(Insets::uniform(f64::from(padding)))
            .background(panel_color)
            // v1's hairline: on the edge the drawer is *not* on.
            .border_edges(
                style.border_width,
                border_color,
                match side {
                    NavigationDrawerSide::Left => Edges::RIGHT,
                    NavigationDrawerSide::Right => Edges::LEFT,
                },
            );
        if style.elevation > 0 {
            panel = panel.shadow(shadow);
        }
        let panel = panel.push(move || {
            // The slots are replayed `Fn`s, and the two inner slots want the
            // header and the content, so each takes a copy.
            let header = header.clone();
            let content = content.clone();
            // v1's `Stack::vertical((header, content))`: the header over the
            // content, eight points apart.
            // The two inner slots are replayed `Fn`s of their own, so the
            // header and the content are copied before they are handed over.
            let header = header.clone();
            let content = content.clone();
            let mut column = Stack::new(Axis::Vertical)
                .gap(8.0)
                // v1's `items_start()`. The stack's alignment is
                // `align(main, cross)`, and both axes start at the beginning.
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

impl Default for CupertinoNavigationDrawer {
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
    let mut request = LayoutRequest::new().width(width);
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
        // v1's Cupertino panel was inset top *and* bottom, rather than given
        // a height.
        bottom: Some(0.0),
    });
    request
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
