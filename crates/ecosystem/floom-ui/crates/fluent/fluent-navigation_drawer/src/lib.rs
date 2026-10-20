//! Fluent navigation drawer, ported to the Floom 2.0 widget API.
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
//! * [`Conditional`] for the drawer's *presence*, which is what carries the
//!   motion.
//!
//! ## This kit's shape is not the other two kits' shape
//!
//! Material and Cupertino put the panel *absolutely* over a full-size scrim,
//! and slid the panel in while the scrim faded. Fluent did neither: the
//! panel and the scrim are two children of a **row**, the panel takes its
//! width and the scrim takes the rest, and the whole row sits on a scrim
//! coloured background. There is no slide in v1's Fluent drawer -- only the
//! row's fade -- and the port keeps that.
//!
//! ## What this kit answers
//!
//! * the panel's fill: `surface`;
//! * the accent: `primary`, on the edge the drawer is *not* on;
//! * the panel's width: the *prop's*, and only the prop's: v1 took
//!   `props.width` with no token fallback;
//! * the panel's padding: the token's;
//! * the elevation shadow: v1's numbers, which differ per kit.
//!
//! ## The tokens that never showed
//!
//! `NavigationDrawerTokens::width` (300) is read by nobody in practice: the
//! prop's width defaults to 360, and this kit takes the prop's whatever it
//! is. The token is the *fallback* in Material and nothing at all here; the
//! port keeps this kit's rule and names it.
//!
//! `NavigationDrawerTokens::elevation` (4) is never read at all: every kit
//! takes the elevation from the props, whose default is also 4. The port
//! reports the prop's elevation, as v1 did, and does not pretend the token
//! is in play.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The panel's inherited ink.** v1 did not set one, and 2.0's `Text`
//!   carries its own colour anyway, so a caller's child keeps the colour it
//!   was given.
//! * **The width was published reactively.** v1 registered the drawer's
//!   width with the theme's layout slots from an effect over the open
//!   signal, so the content area re-inset itself whenever the drawer opened
//!   or closed. 2.0's `open` is a `SharedFlag`, which no effect can track,
//!   so the width is published once, when the view is built -- the same
//!   trade the ported footer makes.
//! * **The child is a slot, not a view.** v1's `header(..)` and `child(..)`
//!   took `impl IntoView`; 2.0's take a [`Child`], because a 2.0 widget is
//!   mounted by its parent rather than returned.

use floom_core::{Child, ChildKey, LayoutRequest, PresenceTransition, WidgetAny};
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
const SCRIM_ALPHA: u8 = 160;
/// The accent's width. A v1 literal.
const ACCENT_WIDTH: f32 = 3.0;
/// The shadow's alpha. A v1 literal.
const SHADOW_ALPHA: u8 = 30;
/// The motion's duration, in seconds. v1 wrote 260 milliseconds.
const MOTION_SECS: f64 = 0.26;

/// The navigation drawer's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentNavigationDrawerStyle {
    pub design_system: &'static str,
    /// The panel's fill.
    pub background: Color,
    /// The scrim's colour, before the alpha below.
    pub scrim: Color,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
    /// The alpha the scrim is painted at. v1's style did not report it.
    pub scrim_alpha: u8,
    /// The accent's width. A v1 literal.
    pub accent_width: f32,
    /// The accent's colour: `primary`.
    pub accent_color: Color,
    /// The shadow's blur: v1 wrote `elevation * 3`.
    pub shadow_blur: f32,
    /// The shadow's spread. A v1 literal.
    pub shadow_spread: f32,
    /// The shadow's colour. A v1 literal.
    pub shadow_color: Color,
}

/// Fluent's navigation drawer.
pub struct FluentNavigationDrawer {
    props: NavigationDrawerProps,
    /// Whether the drawer is open.
    open: SharedFlag,
    header: Option<Child>,
    content: Option<Child>,
    theme: RwSignal<MultiTheme>,
}

impl FluentNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: shared_flag(props.open),
            props,
            header: None,
            content: None,
            // v1's Fluent kit took a snapshot of its own design system.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
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

    pub fn resolved_style(&self) -> FluentNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        FluentNavigationDrawerStyle {
            design_system: "fluent",
            background: theme.colors().surface,
            scrim: theme.colors().scrim,
            // v1 took the prop's width and nothing else: no token fallback.
            width: self.props.width,
            elevation: self.props.elevation,
            padding: tokens.padding,
            scrim_alpha: SCRIM_ALPHA,
            accent_width: ACCENT_WIDTH,
            accent_color: theme.colors().primary,
            shadow_blur: f32::from(self.props.elevation) * 3.0,
            shadow_spread: 1.0,
            shadow_color: with_alpha(Color::rgba(0, 0, 0, 255), SHADOW_ALPHA),
        }
    }

    /// The drawer: the layer, the row, the panel and the scrim.
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

        // The scrim area: the rest of the row, and the thing that dismisses
        // the drawer when it is clicked.
        //
        // `Pressable` is a builder rather than a `Clone` value, so the scrim
        // is built inside the closure that mounts it.
        let scrim_slot = Child::new(
            ChildKey::index(1),
            LayoutRequest::new().grow(1.0),
            {
                let dismiss = open.clone();
                move || {
                    let dismiss = dismiss.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().absolute_fill(),
                            || Box::new(Surface::new()),
                        ))
                        .on_click(move || {
                            write_flag(&dismiss, false);
                        }),
                    ) as Box<dyn WidgetAny>
                }
            },
        );

        // The panel: the header over the content, on the panel's own fill.
        let panel_color = to_paint(style.background);
        let accent_color = to_paint(style.accent_color);
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
            .request(LayoutRequest::new().width(style.width).height_pct(100.0))
            .padding(Insets::uniform(f64::from(padding)))
            .background(panel_color)
            // v1's accent: on the edge the drawer is *not* on.
            .border_edges(
                style.accent_width,
                accent_color,
                match side {
                    NavigationDrawerSide::Left => Edges::LEFT,
                    NavigationDrawerSide::Right => Edges::RIGHT,
                },
            );
        if style.elevation > 0 {
            panel = panel.shadow(shadow);
        }
        let panel = panel.push(move || {
            // v1's `list(panel_children)`: the header over the content, in a
            // column.
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

        // The row: the panel on the chosen side, the scrim taking the rest.
        let panel_slot = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().width(style.width).height_pct(100.0),
            move || Box::new(panel.clone()),
        );
        let mut row = Stack::new(Axis::Horizontal);
        match side {
            NavigationDrawerSide::Left => {
                row = row.child(panel_slot);
                row = row.child(scrim_slot);
            }
            NavigationDrawerSide::Right => {
                row = row.child(scrim_slot);
                row = row.child(panel_slot);
            }
        }

        // The layer: v1's Fluent drawer put the scrim colour on the whole
        // overlay, and the row inside it.
        let scrim_color = to_paint(with_alpha(style.scrim, style.scrim_alpha));
        let layer = Surface::new()
            .request(LayoutRequest::new().absolute_fill())
            .background(scrim_color)
            .child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                move || Box::new(row.clone()),
            ));

        // v1's Fluent drawer faded the whole overlay; it did not slide.
        let content = Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || Box::new(layer.clone()),
        );

        Overlay::new().content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().absolute_fill(),
            move || {
                Box::new(presence(
                    &open,
                    PresenceTransition::fade(MOTION_SECS),
                    content.clone(),
                )) as Box<dyn WidgetAny>
            },
        ))
    }
}

impl Default for FluentNavigationDrawer {
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
/// knows the answer, and a drawer that starts open still plays its fade,
/// which is v1's `run_on_create(true)`.
fn presence(open: &SharedFlag, transition: PresenceTransition, child: Child) -> Conditional {
    let mut conditional = Conditional::new(false).when_flag(open.clone());
    if !read_flag(open) {
        conditional.sync_when(false);
    }
    conditional.presence(transition).show(child)
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
