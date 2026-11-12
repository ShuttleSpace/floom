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
/// The body is written once. Inside it, `B` is the component *type* for that
/// design system and `theme` is a `MultiTheme` pinned to it, so a sample reads
/// `B::new(..).with_theme(theme).<methods>.view()`. Because the three design
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
