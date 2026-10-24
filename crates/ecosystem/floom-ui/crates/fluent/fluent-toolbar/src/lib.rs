//! Fluent toolbar, ported to the Floom 2.0 widget API.
//!
//! # What changed from v1
//!
//! v1 built a view by composing and styling:
//!
//! ```ignore
//! Container::new(Stack::horizontal((left, actions)).style(|s| s.items_center().justify_between()))
//!     .style(move |s| s.width_pct(100.0).height(h).padding(pad).background(..))
//! ```
//!
//! 2.0 has no style closure and no `IntoView`, so the same three ideas become
//! three different things:
//!
//! * the bar is a [`Surface`] — a filled box that holds children and pads them,
//!   which v1's `Container` only did because it was both;
//! * the two groups are a [`Stack`] with `Align::SpaceBetween` on the main axis
//!   (v1's `justify_between`) and `Align::Center` across it (v1's `items_center`);
//! * the trailing actions are a **builder closure** rather than an
//!   `impl IntoView`, because 2.0 slots are replayable `Fn`s, not values.
//!
//! `FluentToolbar::view` still returns the whole bar, content included. That is a
//! deliberate exception to "composition is declared by the parent": *where* the
//! leading icon sits relative to the trailing actions is what makes a toolbar a
//! toolbar, so it is the component's business rather than the caller's.
//!
//! Two capabilities this component was the first to need are now in the engine:
//! `FontWeight` (the `Medium` title) and `Surface::request` (a 64px bar that is
//! also the full width of its parent).
//!
//! The theme is a `MultiTheme` snapshot rather than the app-wide signal that
//! Material's kit uses. That is v1's difference, kept: v1 held a snapshot here
//! and the live signal there.
//!
//! # What was kept deliberately
//!
//! * The title's `Medium` weight and the leading icon's hardcoded 22px are v1
//!   literals, not tokens, and they are still literals here. A port that quietly
//!   promoted them to tokens would change what the component draws.
//! * The bar's height comes from `ToolbarProps::height`, and `ToolbarTokens`
//!   also has a `height` field — which v1 never read. Reading the token instead
//!   would be a behaviour change, so the token stays unread and says so in
//!   [`ToolbarTokensHeightNote`] below rather than being silently preferred.

use floom_core::{FontWeight as EngineFontWeight, LayoutRequest, WidgetAny};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::toolbar::ToolbarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Insets, Shadow};
use floom_widgets::{Align, Axis, Icon, Stack, Surface, Text};

/// Opaque black, the base the toolbar's elevation shadow tints.
const BLACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};

/// The gap between the leading icon and the title, in logical pixels. A v1
/// `.style(|s| s.gap(4.0))` literal.
const LEADING_GAP: f32 = 4.0;

/// The leading icon's font size, in logical pixels. A v1 literal, not a token.
const LEADING_ICON_SIZE: f32 = 22.0;

/// The blur of the elevation shadow, in logical pixels. v1 wrote
/// `box_shadow_blur(6.0).box_shadow_spread(0.0).box_shadow_color(black @ 30%)`
/// — note there is no offset, so the shadow is a symmetric halo rather than a
/// drop below the bar.
const ELEVATION_BLUR: f32 = 6.0;

/// The alpha of that shadow, matching v1's `with_alpha(BLACK, 30)`.
const ELEVATION_ALPHA: u8 = 30;

/// Marks [`ToolbarTokens::height`] as deliberately unread. See the module note.
///
/// [`ToolbarTokens::height`]: floom_core_theme::component_tokens::toolbar::ToolbarTokens::height
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolbarTokensHeightNote;

impl ToolbarTokensHeightNote {
    /// The token's value, for a caller that wants it. Nothing in this component
    /// uses it, because nothing in v1 did.
    pub fn token_height(&self, theme: &MultiTheme) -> f32 {
        match theme.component(ComponentId::Toolbar) {
            ComponentTokens::Toolbar(t) => t.height,
            _ => 0.0,
        }
    }
}

/// A builder for the trailing actions, replayed each time the bar is built.
///
/// 2.0 slots are `Fn`s, not stored values: a widget tree is rebuilt far more often
/// than it is created, so a child held as a value would have to be a pre-built
/// `Box<dyn Widget>` that could only ever be mounted once.
///
/// An [`Arc`] rather than a `Box`, for a reason the compiler states: the bar's own
/// slot is itself a replayed `Fn`, so every rebuild needs its own copy of the
/// actions closure to move into the inner slot. A `Box` cannot be cloned and a
/// borrowed one would not be `'static`.
pub type ActionsBuilder = std::sync::Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// The theme's weight -> the engine's.
///
/// The theme's own `FontWeight::to_floom` returns *parley's* weight, which a kit
/// crate has no business naming: the engine's [`floom_core::FontWeight`] is what
/// `Text` takes, and it is renderer-neutral.
///
/// Worth recording: `to_floom` maps `Black` to `EXTRA_BOLD` (800) rather than
/// `BLACK` (900), and maps `ExtraBold` to `EXTRA_BOLD` too — so the two are
/// indistinguishable there. Neither is reachable from any of the three kits
/// (`FontWeight::Black` has no call site in material, cupertino or fluent), so
/// the mapping is left alone rather than "fixed" where nothing could observe the
/// change. This conversion does not repeat it.
fn to_engine_weight(w: floom_core_theme::tokens::typography::FontWeight) -> EngineFontWeight {
    use floom_core::FontWeight as W;
    use floom_core_theme::tokens::typography::FontWeight as T;
    match w {
        T::Thin => W::Thin,
        T::ExtraLight => W::ExtraLight,
        T::Light => W::Light,
        T::Regular => W::Normal,
        T::Medium => W::Medium,
        T::SemiBold => W::SemiBold,
        T::Bold => W::Bold,
        T::ExtraBold => W::ExtraBold,
        T::Black => W::Black,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentToolbarStyle {
    pub surface: Color,
    pub content: Color,
    pub height: f32,
    pub padding: f32,
    pub title_font_size: f32,
    /// v1 wrote `FontWeight::Medium` here as a literal; it is data now, so the
    /// resolved style states the weight rather than the widget builder knowing it.
    pub title_weight: EngineFontWeight,
    pub elevated: bool,
}

pub struct FluentToolbar {
    props: ToolbarProps,
    theme: RwSignal<MultiTheme>,
    actions: Option<ActionsBuilder>,
}

impl FluentToolbar {
    pub fn new() -> Self {
        Self {
            props: ToolbarProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            actions: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn leading(mut self, icon: impl Into<String>) -> Self {
        self.props.leading = Some(icon.into());
        self
    }

    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    /// Supply the trailing actions slot (icon buttons, menus, and so on).
    ///
    /// v1 took `impl IntoView`, which is a *value*. 2.0 takes a builder, because a
    /// slot is replayed on every rebuild and a stored view could not be mounted
    /// twice. Callers that already have a widget write `move || Box::new(..)`.
    pub fn actions(
        mut self,
        build: impl Fn() -> Box<dyn floom_core::WidgetAny> + Send + Sync + 'static,
    ) -> Self {
        self.actions = Some(std::sync::Arc::new(build));
        self
    }

    pub fn props(&self) -> &ToolbarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentToolbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Toolbar(tokens) = theme.component(ComponentId::Toolbar) else {
            unreachable!("ComponentId::Toolbar must resolve to ToolbarTokens")
        };
        FluentToolbarStyle {
            surface: theme.colors().surface,
            content: theme.colors().on_surface,
            height: self.props.height,
            padding: tokens.padding,
            title_font_size: tokens.title_font_size,
            title_weight: to_engine_weight(
                floom_core_theme::tokens::typography::FontWeight::Medium,
            ),
            elevated: self.props.elevation > 0,
        }
    }

    /// The bar's own extent: its height, and the full width of its parent.
    ///
    /// v1 wrote `.width_pct(100.0).height(h)` on the bar's own style. In 2.0 a
    /// decorated box declares its own extent through
    /// [`Surface::request`](floom_widgets::Surface::request), so this stays on the
    /// surface rather than being pushed onto the caller's slot for it — one box's
    /// styling on one widget.
    pub fn request(&self) -> LayoutRequest {
        let style = self.resolved_style();
        LayoutRequest::new().height(style.height).width_pct(100.0)
    }

    pub fn padding(&self) -> Insets {
        Insets::uniform(f64::from(self.resolved_style().padding))
    }

    /// The elevation shadow, or `None` for an unelevated bar.
    ///
    /// `None` rather than a zero-blur transparent shadow, because a bar with no
    /// elevation should not carry a shadow op at all.
    pub fn shadow(&self) -> Option<Shadow> {
        self.resolved_style().elevated.then(|| Shadow {
            color: to_paint(with_alpha(BLACK, ELEVATION_ALPHA)),
            blur: f64::from(ELEVATION_BLUR),
            offset_x: 0.0,
            offset_y: 0.0,
            spread: 0.0,
        })
    }

    /// The leading icon, if this toolbar has one.
    ///
    /// v1 rendered the glyph through `Label::new(ic)`; 2.0's [`Icon`] is the same
    /// thing with the accessibility default spelled out (decorative unless
    /// labelled).
    ///
    /// Takes the glyph as an argument rather than reading `self`, because a 2.0
    /// slot is a **replayed** `Fn`: it runs again on every rebuild, so anything it
    /// captures has to be cloneable data rather than a widget built once. See
    /// [`Self::leading_group`].
    pub fn leading_widget(&self, glyph: char) -> Icon {
        let content = to_paint(self.resolved_style().content);
        Icon::new(glyph).size(LEADING_ICON_SIZE).color(content)
    }

    /// The title label: the resolved size, the `Medium` weight, the content colour.
    pub fn title_widget(&self, title: &str) -> Text {
        let style = self.resolved_style();
        Text::new(title)
            .size(style.title_font_size)
            .weight(style.title_weight)
            .color(to_paint(style.content))
    }

    /// The leading group — icon and title, side by side, vertically centred.
    ///
    /// Built from plain data rather than from `self`, and that is not a stylistic
    /// choice: 2.0's `push` takes `impl Fn() -> Box<dyn WidgetAny> + Send + Sync`,
    /// and `FluentToolbar` holds an `RwSignal`, which is `Rc`-based and therefore
    /// neither. A slot that closed over `self` could not be built at all. So the
    /// theme is resolved first and the closure captures `String`s, `f32`s and
    /// colours — all of which are `Send + Sync` and cloneable.
    ///
    /// Exposed because a caller composing the bar by hand should not have to
    /// restate the gap or the alignment to get the same arrangement.
    pub fn leading_group(&self) -> Stack {
        let style = self.resolved_style();
        leading_group_of(
            self.props.leading.as_deref().and_then(|s| s.chars().next()),
            &self.props.title,
            style.title_font_size,
            style.title_weight,
            to_paint(style.content),
        )
    }

    /// The whole bar: the filled box, its padding, its extent, and its two groups.
    ///
    /// The trailing actions are pushed only when the caller supplied them. v1
    /// substituted an empty view, which with `justify_between` put an invisible
    /// box at the far end — and, being a *child*, it also meant the title was no
    /// longer the only thing deciding the bar's contents. With one child
    /// `SpaceBetween` places it at the start edge, which is the right answer.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let request = self.request();
        let padding = self.padding();
        let shadow = self.shadow();
        let surface_colour = to_paint(style.surface);
        let content = to_paint(style.content);
        let title = self.props.title.clone();
        let leading = self.props.leading.as_deref().and_then(|s| s.chars().next());
        let title_size = style.title_font_size;
        let title_weight = style.title_weight;
        // Last, because it moves: everything above borrows `self`.
        let actions = self.actions;

        let mut surface = Surface::new()
            .request(request)
            .padding(padding)
            .background(surface_colour)
            .push(move || {
                Box::new(bar_of(
                    leading,
                    &title,
                    title_size,
                    title_weight,
                    content,
                    actions.clone(),
                ))
            });
        if let Some(shadow) = shadow {
            surface = surface.shadow(shadow);
        }
        surface
    }
}

/// The whole bar as a function over plain data.
///
/// Free-standing because both the surface's slot and any caller composing by hand
/// need it, and neither may capture a built widget.
fn bar_of(
    leading: Option<char>,
    title: &str,
    title_size: f32,
    title_weight: EngineFontWeight,
    content: PaintColor,
    actions: Option<ActionsBuilder>,
) -> Stack {
    let title = title.to_string();
    let mut bar = Stack::new(Axis::Horizontal)
        .align(Align::SpaceBetween, Align::Center)
        .push(move || {
            Box::new(leading_group_of(
                leading,
                &title,
                title_size,
                title_weight,
                content,
            ))
        });
    if let Some(actions) = actions {
        bar = bar.push(move || actions());
    }
    bar
}

/// The leading group as a free function over plain data.
///
/// Separate from [`FluentToolbar::leading_group`] because both callers need it and
/// neither can capture a widget: a slot is rebuilt, not stored.
fn leading_group_of(
    leading: Option<char>,
    title: &str,
    title_size: f32,
    title_weight: EngineFontWeight,
    content: PaintColor,
) -> Stack {
    let mut group = Stack::new(Axis::Horizontal)
        .gap(LEADING_GAP)
        .align(Align::Start, Align::Center);
    if let Some(glyph) = leading {
        group =
            group.push(move || Box::new(Icon::new(glyph).size(LEADING_ICON_SIZE).color(content)));
    }
    let title = title.to_string();
    group.push(move || {
        Box::new(
            Text::new(title.clone())
                .size(title_size)
                .weight(title_weight)
                .color(content),
        )
    })
}

impl Default for FluentToolbar {
    fn default() -> Self {
        Self::new()
    }
}
