//! Cupertino menu, ported to the Floom 2.0 widget API.
//!
//! A menu is a list of choices in a floating panel: the panel appears over the
//! page, a row is filled while the pointer is over it, and choosing a row runs
//! its callback and closes the menu.
//!
//! ## What the engine grew
//!
//! **A `Menu` composite.** v1's menu was an overlay holding a container of
//! labels, hidden by a signal and shown by the same signal. 2.0's host owns the
//! overlay layers, so the panel is asked of the host and sized by the engine
//! (the widest row, clamped to the tokens' bounds) -- and the row that is
//! filled, the row that is clicked and the menu closing are all one widget's
//! business now, rather than three kits' worth of event handlers.
//!
//! ## What this kit answers
//!
//! * the panel's fill: `surface`; the rows' ink: `on_surface`;
//! * a hovered row: `primary` at 16 -- this kit's alone, where the other two
//!   use `on_surface` at 8;
//! * the panel's width, the row's height and the panel's padding: the tokens;
//! * the row's padding (12 at the sides, 8 above and below): v1 literals;
//! * the panel's radius: the theme's `md` radius, where Material wrote a
//!   literal 4 and Fluent the theme's `sm`.
//!
//! ## What the port's style carries that v1's did not
//!
//! v1's `CupertinoMenuStyle` reported six numbers, and its paint used more than
//! that: the maximum width, the radius, the shadow, the hovered row's colour,
//! the row's padding and the row's text size were all in the markup and none
//! of them in the style. The port's style reports all of them, because a style
//! that does not say what the widget draws is not the widget's output.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`open_signal()` was an `RwSignal<bool>`.** 2.0's shared state is a
//!   `SharedFlag`, and the menu and the thing that opens it share one.
//! * **`item(..)` took a `Box<dyn Fn()>`.** The engine's slots are
//!   `Send + Sync`, so the callback is `Fn() + Send + Sync`.
//! * **The menu's own position.** v1's panel went into an overlay at the
//!   overlay's origin, and the caller chose where that was by choosing the
//!   slot the menu was declared in. 2.0 keeps that: the caller still chooses
//!   the slot, and the host does the arithmetic.
//! * **The row's text is not centred.** v1 put it `padding_vert` below the
//!   row's top edge, in a row twice as tall as the text; the port reproduces
//!   that, because the padding is in v1's markup and a row whose label sits
//!   above its middle is v1's doing.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::Shadow;
use floom_ui_composites::{Menu, MenuItem, MenuStyle};
use floom_widgets::SharedFlag;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::menu::{MenuItem as MenuItemProps, MenuProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;

const DESIGN_SYSTEM: &str = "cupertino";

/// The menu's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoMenuStyle {
    pub design_system: &'static str,
    /// The panel's fill.
    pub container_color: Color,
    /// The rows' ink.
    pub text_color: Color,
    /// The fill behind the row the pointer is over.
    pub hover_color: Color,
    pub min_width: f32,
    pub max_width: f32,
    pub item_height: f32,
    pub item_padding_horizontal: f32,
    pub item_padding_vertical: f32,
    pub item_font_size: f32,
    pub padding: f32,
    pub radius: f32,
    pub shadow: Shadow,
}

/// Cupertino's menu.
pub struct CupertinoMenu {
    props: MenuProps,
    /// Whether the menu is up, shared with whoever opens it.
    open: SharedFlag,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoMenu {
    pub fn new() -> Self {
        Self {
            props: MenuProps::default(),
            open: floom_widgets::shared_flag(false),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
        }
    }

    /// Append a row.
    pub fn item(
        mut self,
        label: impl Into<String>,
        callback: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        self.props.items.push(MenuItemProps {
            label: label.into(),
            on_select: Some(Box::new(callback)),
        });
        self
    }

    /// Whether the menu starts open.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        floom_widgets::write_flag(&self.open, open);
        self
    }

    pub fn props(&self) -> &MenuProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The flag that says whether the menu is up.
    ///
    /// v1's `open_signal()` handed out an `RwSignal<bool>`. 2.0's shared state
    /// is a `SharedFlag` -- a bool behind an `Arc<Mutex<bool>>` -- because a
    /// value that changes after mount cannot be held in a signal and read
    /// during render.
    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Use a flag the caller already has, so the thing that opens the menu and
    /// the menu itself share one truth.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self
    }

    /// Choose the row at `index`, as if it had been clicked.
    ///
    /// v1's `select(..)` did the same: it ran the row's callback and nothing
    /// else -- it did not close the menu.
    pub fn select(&self, index: usize) {
        if let Some(item) = self.props.items.get(index) {
            if let Some(callback) = &item.on_select {
                callback();
            }
        }
    }

    /// The menu's appearance, in the terms v1 used.
    pub fn resolved_style(&self) -> CupertinoMenuStyle {
        let theme = self.theme.get();
        let ComponentTokens::Menu(tokens) = theme.component(ComponentId::Menu) else {
            unreachable!("ComponentId::Menu must resolve to MenuTokens")
        };
        CupertinoMenuStyle {
            design_system: DESIGN_SYSTEM,
            container_color: theme.colors().surface,
            text_color: theme.colors().on_surface,
            // v1's hovered row: `with_alpha(primary, 16)` -- this kit's own,
            // where the other two use `on_surface` at 8.
            hover_color: with_alpha(theme.colors().primary, 16),
            min_width: tokens.min_width,
            max_width: tokens.max_width,
            item_height: tokens.item_height,
            // v1's row: `padding_horiz(12.0).padding_vert(8.0)`.
            item_padding_horizontal: 12.0,
            item_padding_vertical: 8.0,
            // v1's row: `font_size(t.typography().label.large.font_size)`.
            item_font_size: theme.typography().body.medium.font_size,
            padding: tokens.padding,
            // v1's panel: the theme's `md` radius.
            radius: theme.radius().md,
            // v1's panel: `box_shadow_blur(12.0).box_shadow_spread(1.0)
            // .box_shadow_color(with_alpha(black, 24))`.
            shadow: Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 24)),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
        }
    }

    /// The menu: the panel, its rows, and the flag that says whether it is up.
    pub fn view(self) -> Menu {
        let style = self.resolved_style();
        let mut menu = Menu::new()
            .style(MenuStyle {
                background: to_paint(style.container_color),
                text_color: to_paint(style.text_color),
                hover_color: to_paint(style.hover_color),
                min_width: f64::from(style.min_width),
                max_width: f64::from(style.max_width),
                item_height: f64::from(style.item_height),
                item_padding_horizontal: f64::from(style.item_padding_horizontal),
                item_padding_vertical: f64::from(style.item_padding_vertical),
                item_font_size: style.item_font_size,
                padding: f64::from(style.padding),
                radius: style.radius,
                shadow: style.shadow,
            })
            .with_open_flag(self.open.clone());
        for item in self.props.items {
            menu = menu.item(MenuItem {
                label: item.label,
                on_select: item.on_select,
            });
        }
        menu
    }
}

impl Default for CupertinoMenu {
    fn default() -> Self {
        Self::new()
    }
}
