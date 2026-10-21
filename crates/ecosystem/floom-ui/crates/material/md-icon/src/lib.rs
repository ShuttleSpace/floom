//! Material 3 icon, ported to the Floom 2.0 widget API.
//!
//! # What changed from v1
//!
//! v1 rendered an icon as SVG markup fed to the renderer:
//!
//! ```ignore
//! svg(move || icons::md_icon_svg(&name)).style(move |s| {
//!     s.size(sz, sz).color(t.colors().on_surface_variant.to_peniko())
//! })
//! ```
//!
//! That whole pipeline is gone in 2.0: there is no `svg` view, no `usvg` in the
//! render path, and no style closure. The icon's *shape* — one filled outline —
//! is expressed as a [`floom_widgets::VectorIcon`], which parses the `d`
//! attribute once at construction and fills a `Path` at paint.
//!
//! Two things worth naming:
//!
//! * **`md_icon_svg` still exists, for the v1 callers.** Six material crates
//!   (`md-list_group`, `md-stepper`, `md-confirm_edit`, `md-checkbox_btn`,
//!   `md-checkbox`, `md-rating`) are still on v1 and call it to hand markup to
//!   their own `svg()` view. Deleting it would break them; porting them is
//!   separate work. New code uses [`MdIcon::resolve`].
//! * **The size and colour are resolved up front.** v1's style closure re-read
//!   the theme on every use, which 2.0's mount-once tree cannot do, so the kit
//!   resolves them and pushes the result in — the same pattern `md-button` uses.
//!
//! See `docs/porting-floem-ui-to-2.0.md`.

use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, Rect};
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon::IconProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_widget_runtime::theme_signal;

mod icons;

// v1 markup, for the six material crates still on v1 that feed it to their own
// `svg()` view. 2.0 callers use `MdIcon::resolve` instead.
pub use icons::md_icon_svg;

/// The Material Symbols view box every glyph in [`icons`] is authored against.
pub const MD_ICON_VIEW_BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdIconStyle {
    pub color: Color,
    pub size: f32,
}

pub struct MdIcon {
    props: IconProps,
    theme: RwSignal<MultiTheme>,
}

impl MdIcon {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            props: IconProps {
                name: name.into(),
                ..Default::default()
            },
            theme: theme_signal(),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = Some(size);
        self
    }

    /// Names the icon for assistive technology. Implies `decorative(false)`:
    /// an icon with a label is no longer decoration.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self.props.decorative = false;
        self
    }

    pub fn decorative(mut self, decorative: bool) -> Self {
        self.props.decorative = decorative;
        self
    }

    pub fn props(&self) -> &IconProps {
        &self.props
    }

    /// The glyph's name, as given.
    pub fn name(&self) -> &str {
        &self.props.name
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdIconStyle {
        let theme = self.theme.get();
        let ComponentTokens::Icon(tokens) = theme.component(ComponentId::Icon) else {
            unreachable!("ComponentId::Icon must resolve to IconTokens")
        };
        MdIconStyle {
            color: theme.colors().on_surface_variant,
            size: self.props.size.unwrap_or(tokens.size),
        }
    }

    /// The path data for this icon's glyph, or the fallback dot for a name the
    /// set does not carry.
    pub fn path_data(&self) -> &'static str {
        icons::md_icon_path_data(&self.props.name)
    }

    /// The icon as a 2.0 widget, ready to mount with `Child::new(..)`.
    ///
    /// This is the whole port: resolve the theme (the kit's job), parse the
    /// outline once, hand back a widget. There is no `view()` terminal call
    /// because 2.0 widgets are not composed — the *parent* declares them.
    pub fn resolve(&self) -> VectorIcon {
        let style = self.resolved_style();
        let mut icon = VectorIcon::from_path_data(self.path_data(), MD_ICON_VIEW_BOX)
            .size(style.size)
            .color(to_paint(style.color));
        // v1's `svg` view had no accessibility story at all; 2.0's does, and
        // the props already carried the intent, so this is a fix rather than a
        // translation. An icon with no label stays decorative.
        if let Some(label) = self.props.label.as_deref() {
            icon = icon.label(label);
        }
        icon
    }
}

impl Default for MdIcon {
    fn default() -> Self {
        Self::new("")
    }
}
