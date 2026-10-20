//! Fluent main content region, ported to the Floom 2.0 widget API.
//!
//! The main region is the part of the screen the application's content lives
//! in: the area between the app bar above it, the footer or the bottom
//! navigation below it, and the navigation drawers at its sides. It reads the
//! app-wide layout signal and offsets its content by the sizes those shell
//! pieces registered, so the content never hides under them.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a vertical [`Stack`] of a top spacer, the middle
//! row and a bottom spacer; the middle row a horizontal [`Stack`] of a left
//! spacer, the content and a right spacer; and a [`Surface`] for the content's
//! own fill. The spacers are `Surface`s with a size and no fill -- 2.0's
//! spelling of v1's `Empty`.
//!
//! ## What this kit answers
//!
//! * the fill behind the content: `background`;
//! * the four insets: the app-wide layout signal's own numbers, so the kit
//!   answers no number of its own.
//!
//! ## The theme it reads
//!
//! Fluent's theme is a *snapshot*: the kit takes a `MultiTheme` for
//! Fluent when it is built and reads its tokens and colours from it.
//! The Material kit reads the app-wide signal instead, so a change of theme
//! re-themes the main region with everything else.
//!
//! ## What the three kits disagree about
//!
//! Only the theme they read: the four insets are the same numbers in all
//! three, and the fill is each theme's own background.
//!
//! ## The style v1 did not have
//!
//! v1's main had no style struct at all -- it was a view and nothing else.
//! The port adds one, because a kit whose numbers cannot be read cannot be
//! tested: [`FluentMainStyle`] reports the fill and the four insets.
//!
//! ## The difference that is named rather than reproduced
//!
//! **v1 re-read the layout signal on every style pass; the port reads it once,
//! when the tree is built.** v1's spacers each carried a style closure that
//! read `layout_signal()` on every frame, so a bar that registered its height
//! after the main was built re-inset the main at once. 2.0 mounts a widget
//! once: the insets are read when `view()` runs and the spacers are sized from
//! them. The reason is the signal itself -- `RwSignal` is `Rc`-based and so
//! neither `Send` nor `Sync`, and 2.0's slot closures are shared between
//! threads, so a closure that read the signal could not be built at all.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The child is a slot, not a view.** v1's `child(..)` took an
//!   `impl IntoView`; 2.0's takes a [`Child`], because a 2.0 widget is
//!   mounted by its parent rather than returned.
//! * **The empty child.** v1 mounted an `Empty` view when there was no child,
//!   so the content box always had one child. Here the slot is absent when
//!   there is nothing to put in it. `Empty` had no size, so there is no pixel
//!   difference -- only a structural one.
//! * **The accessors.** v1's main had no `props()`, `theme()` or `with_theme()`.
//!   The port gives it `theme()` and `with_theme()`, because a kit that cannot
//!   be given a theme cannot be tested against one. There is no `props()`:
//!   `floom-core-theme` has no props struct for the main region, so there is
//!   none to report.

use floom_core::{Child, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Axis, Stack, Surface};

use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The main region's appearance, in the terms v1 used.
///
/// v1 had no style struct; this one is the port's own, and it reports the
/// numbers the widget is built from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentMainStyle {
    pub design_system: &'static str,
    /// The fill behind the content.
    pub background: Color,
    /// The registered top inset: the system bar plus the app bar.
    pub top: f32,
    /// The registered bottom inset: the footer plus the bottom navigation.
    pub bottom: f32,
    /// The open left navigation drawer's width.
    pub left: f32,
    /// The open right navigation drawer's width.
    pub right: f32,
}

/// Fluent's main content region.
pub struct FluentMain {
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
}

impl FluentMain {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    /// The scrollable/static content of the application.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentMainStyle {
        let theme = self.theme.get();
        // The insets are read here, once: see the module's note on the layout
        // signal.
        let layout = layout_signal().get();
        FluentMainStyle {
            design_system: "fluent",
            background: theme.colors().background,
            top: layout.top(),
            bottom: layout.bottom(),
            left: layout.left(),
            right: layout.right(),
        }
    }

    /// Build the main content region with automatic layout insets.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();

        // v1's spacers were `Empty` views: a box with a size and nothing in
        // it. A `Surface` with no fill is that in 2.0.
        let top = Surface::new().request(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(style.top),
        );
        let bottom = Surface::new().request(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(style.bottom),
        );
        let left = Surface::new().request(
            LayoutRequest::new()
                .height_pct(100.0)
                .width(style.left),
        );
        let right = Surface::new().request(
            LayoutRequest::new()
                .height_pct(100.0)
                .width(style.right),
        );

        // v1's content: a container that grew and carried the background.
        let mut content = Surface::new().background(to_paint(style.background));
        if let Some(child) = self.child {
            content = content.child(child);
        }

        // v1's middle row: the left spacer, the content and the right spacer.
        // The content's `flex_grow(1.0)` lives on the *slot*, because 2.0's
        // layout takes a grow factor from the slot's request and not from the
        // widget's own.
        let middle = Stack::new(Axis::Horizontal)
            .request(LayoutRequest::new().width_pct(100.0))
            .push(move || Box::new(left.clone()))
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(content.clone())
            })
            .push(move || Box::new(right.clone()));

        // v1's outer stack: the top spacer, the middle row and the bottom
        // spacer.
        Stack::new(Axis::Vertical)
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .height_pct(100.0),
            )
            .push(move || Box::new(top.clone()))
            .push_layout(LayoutRequest::new().grow(1.0), move || {
                Box::new(middle.clone())
            })
            .push(move || Box::new(bottom.clone()))
    }
}

impl Default for FluentMain {
    fn default() -> Self {
        Self::new()
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
