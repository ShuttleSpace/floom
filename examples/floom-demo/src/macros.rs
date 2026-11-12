//! The compare row puts one component in three design systems side by side.
//!
//! Every component exists in Material, Cupertino and Fluent as three types with
//! the same builder methods. `compare_children!` writes the sample once and
//! produces its three columns; `compare_row` lays them out with a caption each.

use floom::floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::MultiTheme;

/// Build one component in three design systems, each under its own theme.
///
/// The body is written once. Inside it, `$b` is the component *type* for that
/// design system and `$theme` is a `MultiTheme` pinned to it, so a sample reads
/// `$b::new(..).with_theme(theme).<methods>.view()`. Because the three design
/// systems spell their builders identically, the same body compiles for all
/// three; the macro instantiates it three times, each with a pinned theme, so
/// each column carries that design system's real tokens.
///
/// The widget is built *inside* the child closure: the engine widgets are not
/// `Send + Sync`, so they are never captured across the closure boundary.
#[macro_export]
macro_rules! compare_children {
    ($m:ty, $c:ty, $f:ty, |$b:ident, $theme:ident| $body:expr $(,)?) => {{
        let m = $crate::macros::one_column($crate::macros::Ds::Material, |$theme| {
            #[allow(non_camel_case_types)]
            type $b = $m;
            $body
        });
        let c = $crate::macros::one_column($crate::macros::Ds::Cupertino, |$theme| {
            #[allow(non_camel_case_types)]
            type $b = $c;
            $body
        });
        let f = $crate::macros::one_column($crate::macros::Ds::Fluent, |$theme| {
            #[allow(non_camel_case_types)]
            type $b = $f;
            $body
        });
        (m, c, f)
    }};
}

pub use compare_children;

/// Build one component in three design systems, with a second one beside it.
///
/// Some components take another component as their content -- a floating
/// action button takes its glyph, a button group takes its buttons -- and that
/// content has to be the design system's *own*, or the three columns would
/// not be showing the same thing. This macro binds two types per design
/// system: `$b`, the component on show, and `$x`, its companion.
#[macro_export]
macro_rules! compare_with {
    ($m:ty, $c:ty, $f:ty, $xm:ty, $xc:ty, $xf:ty, |$b:ident, $x:ident, $theme:ident| $body:expr $(,)?) => {{
        let m = $crate::macros::one_column($crate::macros::Ds::Material, |$theme| {
            #[allow(non_camel_case_types)]
            type $b = $m;
            #[allow(non_camel_case_types)]
            type $x = $xm;
            $body
        });
        let c = $crate::macros::one_column($crate::macros::Ds::Cupertino, |$theme| {
            #[allow(non_camel_case_types)]
            type $b = $c;
            #[allow(non_camel_case_types)]
            type $x = $xc;
            $body
        });
        let f = $crate::macros::one_column($crate::macros::Ds::Fluent, |$theme| {
            #[allow(non_camel_case_types)]
            type $b = $f;
            #[allow(non_camel_case_types)]
            type $x = $xf;
            $body
        });
        (m, c, f)
    }};
}

pub use compare_with;

/// A widget builder that rebuilds the component each time the host asks for it.
///
/// A component holds its theme in an `Rc`-backed signal, so a component cannot
/// cross the `Send + Sync` boundary a child closure must satisfy. What *can*
/// cross it is the theme -- `MultiTheme` is `Send + Sync` -- so the closure
/// captures the theme and rebuilds the component from it, applying `$chain` to
/// the themed component bound to `$c`.
///
/// Inside [`compare_children!`] the body reads
/// `themed_widget!(b, theme, |c| c.label("Save").view())`, where `b` is the
/// component type for the design system being built and `theme` is the theme
/// pinned to it.
#[macro_export]
macro_rules! themed_widget {
    ($b:ident, $theme:ident, |$c:ident| $chain:expr) => {{
        let $theme = $theme.clone();
        move || -> Box<dyn $crate::WidgetAny> {
            let $c = $b::new().with_theme($theme.clone());
            Box::new($chain)
        }
    }};

    // The same, for a component whose constructor takes an argument -- an
    // icon button's glyph, a radio's label. The argument is copied into each
    // build, so the closure stays `Fn`.
    ($b:ident, $theme:ident, $arg:expr, |$c:ident| $chain:expr) => {{
        let $theme = $theme.clone();
        let __arg = $arg;
        move || -> Box<dyn $crate::WidgetAny> {
            let $c = $b::new(__arg).with_theme($theme.clone());
            Box::new($chain)
        }
    }};
}

pub use themed_widget;

/// The design system's own icon for `name`, under `theme`.
///
/// The icon is rebuilt each time the host asks for it: an icon component
/// holds its theme in an `Rc`-backed signal, so it cannot be captured across
/// the `Send + Sync` boundary a child closure must satisfy. Only the theme
/// crosses it.
#[macro_export]
macro_rules! ds_icon {
    ($i:ident, $theme:ident, $name:literal) => {{
        let $theme = $theme.clone();
        move || -> Box<dyn $crate::WidgetAny> {
            Box::new($i::new($name).with_theme($theme.clone()).resolve())
        }
    }};
}

pub use ds_icon;

/// The three design systems, so the macro does not reach into the satellite.
pub enum Ds {
    Material,
    Cupertino,
    Fluent,
}

/// One column: a child built under a theme pinned to `ds`.
///
/// The theme is re-created on every build call (a `Child`'s closure may run
/// more than once, and `with_theme` takes the theme by value), so each rebuild
/// carries a fresh, fully pinned `MultiTheme`.
pub fn one_column<F, R>(ds: Ds, body: F) -> Child
where
    F: Fn(MultiTheme) -> R + Send + Sync + 'static,
    R: WidgetAny + 'static,
{
    Child::new(ChildKey::index(0), LayoutRequest::new().fill(), move || {
        let theme = match ds {
            Ds::Material => MultiTheme::pinned(DesignSystem::Material),
            Ds::Cupertino => MultiTheme::pinned(DesignSystem::Cupertino),
            Ds::Fluent => MultiTheme::pinned(DesignSystem::Fluent),
        };
        Box::new(body(theme))
    })
}
