//! Cross-design-system component gallery built on floom-elements.
//!
//! The same neutral-API source renders every component for Material, Cupertino,
//! or Fluent depending on the active feature:
//!   cargo run -p floom-demo                      # Material (default)
//!   cargo run -p floom-demo --no-default-features --features cupertino
//!   cargo run -p floom-demo --no-default-features --features fluent
//!
//! The window is a two-pane master/detail layout:
//!   * LEFT  — a clickable list of every component (one entry per component).
//!             The active row is highlighted reactively from the `selected`
//!             signal.
//!   * RIGHT — the demo for the selected component, inside a scroll container so
//!             tall pages scroll.
//!
//! Floating surfaces (Dialog, Menu, Snackbar, NavigationDrawer, Sheet, Overlay)
//! are returned separately by `gallery::build()` and rendered at the WINDOW ROOT
//! (absolutely positioned, full-window) rather than inside a scrollable list —
//! otherwise their `width_pct(100%).height_pct(100%)` scrim collapses to zero
//! height and they are invisible / unclickable.

mod gallery;
use gallery::{build, Gallery};
mod showcase;
mod showcase_pilots;
mod showcase_rollout;

#[cfg(all(feature = "material", not(feature = "multi")))]
const DS_NAME: &str = "Material 3";
#[cfg(feature = "cupertino")]
const DS_NAME: &str = "Cupertino";
#[cfg(feature = "fluent")]
const DS_NAME: &str = "Fluent";
#[cfg(feature = "multi")]
const DS_NAME: &str = "Multi-DS";

use floom::IntoView;
use floom::View;
use floom::Application;
use std::rc::Rc;
use floom::peniko::kurbo::Size;
use floom::reactive::{RwSignal, SignalGet, SignalUpdate};
use floom::views::{Decorators, Scroll, Stack, dyn_view};
use floom::window::WindowConfig;
use floom_core_theme::theme::Theme;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_elements::button::ButtonVariant;
use floom_elements::prelude::*;
use floom_widget_runtime::{dark_signal, design_system_signal, is_dark, set_dark, set_design_system, theme_signal, DesignSystem};

fn main() {
    Application::new()
        .window(
            move |_| app_view(),
            Some(
                WindowConfig::default()
                    .size(Size::new(1040.0, 900.0))
                    .show_titlebar(true)
                    .title(format!("floom-ui · {} gallery", DS_NAME)),
            ),
        )
        .run();
}

/// Builds the gallery root view.
///
/// Extracted from `main` so the exact same tree can be exercised headlessly in
/// tests (via `floom::headless`) without opening a real window — that lets us
/// catch reactive/runtime panics that only surface during view construction
/// and the initial style/layout passes.
pub fn app_view() -> impl IntoView {
        // `build()` owns the persistent overlay signals, the two
        // `create_effect`s (Form / PullToRefresh), and returns the master list
        // plus the root overlays. It must run here, inside the window view
        // factory, so the reactive scope is active when effects are created.
        let Gallery { pages, overlays } = build();
        // Share the immutable page list across the two `dyn_view`s below (left
        // list + right detail) without cloning the non-`Clone` `Page` factories.
        let pages = Rc::new(pages);

        let selected = RwSignal::new(0usize);
        let theme = theme_signal();

        // ---- LEFT pane: clickable component list ----
        // Wrapped in `dyn_view` reading `design_system_signal()` so the nav
        // buttons (which are dispatched `Button`s) re-skin their *shape* when the
        // active Design System changes — not just their highlight color.
        let pages_left = Rc::clone(&pages);
        let left = Scroll::new(
            dyn_view(move || {
                let _ds = design_system_signal().get();
                Stack::vertical_from_iter(pages_left.iter().enumerate().map(|(i, (name, _))| {
                    let name = name.clone();
                    Button::new(name)
                        .variant(ButtonVariant::Text)
                        .on_click(move || selected.set(i))
                        .view()
                        .style(move |s| {
                            let active = selected.get() == i;
                            s.width_full()
                                .padding(8.0)
                                .justify_start()
                                .border_radius(8.0)
                                .background(if active {
                                    theme.get().colors().primary_container.to_peniko()
                                } else {
                                    with_alpha(theme.get().colors().on_surface, 0).to_peniko()
                                })
                        })
                        .into_any()
                }))
                .style(|s| s.gap(2.0).padding(8.0))
            }),
        )
        .style(|s| s.width(240.0).height_pct(100.0));

        // ---- RIGHT pane: selected component demo (scrollable) ----
        // Reading `design_system_signal()` here makes the closure re-run (and
        // rebuild the selected page with the new DS components) whenever the
        // active Design System changes at runtime.
        let pages_right = Rc::clone(&pages);
        let right = Scroll::new(dyn_view(move || {
            let _ds = design_system_signal().get();
            let idx = selected.get();
            pages_right[idx].1()
        }))
        .style(|s| s.flex_grow(1.0).height_pct(100.0).padding(20.0));

        let split = Stack::horizontal((left, right)).style(move |s| {
            s.width_pct(100.0)
                .height_pct(100.0)
                .background(theme.get().colors().background.to_peniko())
        });

        // ---- Top-right Design System switcher (absolutely positioned overlay) ----
        // Wrapped in `dyn_view` so the three switch buttons (dispatched
        // `Button`s) re-skin their *shape* when the active DS changes, instead of
        // staying Material-shaped regardless of selection. Clicking one calls
        // `set_design_system`, which re-skins the whole demo (LEFT list, RIGHT
        // detail pane and the toggle itself) in a single click.
        let ds_now = design_system_signal();
        let ds_segment = dyn_view(move || {
            let _ds = design_system_signal().get();
            let _dark = dark_signal().get();
            Stack::horizontal((
                Stack::horizontal((
                    ds_switch_button(DesignSystem::Material, ds_now, theme),
                    ds_switch_button(DesignSystem::Cupertino, ds_now, theme),
                    ds_switch_button(DesignSystem::Fluent, ds_now, theme),
                ))
                .style(move |s| {
                    s.gap(2.0)
                        .padding(3.0)
                        .border_radius(10.0)
                        .background(theme.get().colors().surface_variant.to_peniko())
                })
                .into_any(),
                // Dark-mode toggle: flips the process-wide `dark_signal`, which
                // `Theme::colors` reads, so the whole app re-themes.
                Button::new(if is_dark() { "Light" } else { "Dark" })
                    .variant(ButtonVariant::Secondary)
                    .on_click(move || set_dark(!is_dark()))
                    .view()
                    .style(move |s| {
                        s.padding(8.0).border_radius(8.0).justify_center()
                    })
                    .into_any(),
            ))
            .style(move |s| {
                s.absolute()
                    .inset_top(12.0)
                    .inset_right(12.0)
                    .gap(8.0)
                    .items_center()
            })
            .into_any()
        });

        // ---- Root overlays ----
        // Each overlay is a Floom `Overlay` widget, which reparents its content
        // to the window root (z-index 1) and hides it via `.hide()` when closed,
        // so it captures pointer events ONLY while open. We must therefore NOT
        // wrap them in a persistent full-window container — that wrapper sat on
        // top of the interactive panes and swallowed every click (the "demo
        // doesn't respond to clicks" bug). Placing the overlays directly lets
        // Floom handle their window-level positioning and open/closed capture.
        let overlays_view = Stack::vertical_from_iter(overlays).into_any();

        let inner = Stack::vertical((split, overlays_view, ds_segment))
            .style(|s| s.width_pct(100.0).height_pct(100.0));

        // Wrap the whole gallery in a `LocaleProvider` seeded with `zh-CN` so
        // any locale-aware content defaults to Simplified Chinese. The provider
        // is a transparent `Container` wrapper (no visual change) that shares
        // the BCP-47 tag with its subtree via `locale_signal()`.
        LocaleProvider::new()
            .locale("zh-CN")
            .child(inner)
            .view()
            .style(|s| s.width_pct(100.0).height_pct(100.0))
}

/// Builds a single button in the top-right Design System switcher. The button
/// is highlighted when its `DesignSystem` matches the active one, and clicking
/// it calls `set_design_system` (which bumps the global theme signal so every
/// component re-skins).
fn ds_switch_button(
    ds: DesignSystem,
    ds_now: RwSignal<DesignSystem>,
    theme: RwSignal<floom_core_theme::theme::MultiTheme>,
) -> impl View {
    Button::new(ds.label())
        .variant(ButtonVariant::Text)
        .on_click(move || set_design_system(ds))
        .view()
        .style(move |s| {
            let active = ds_now.get() == ds;
            let t = theme.get();
            s.padding(8.0)
                .border_radius(6.0)
                .justify_center()
                .background(if active {
                    t.colors().primary_container.to_peniko()
                } else {
                    with_alpha(t.colors().on_surface, 0).to_peniko()
                })
                .color(if active {
                    t.colors().on_primary_container.to_peniko()
                } else {
                    t.colors().on_surface.to_peniko()
                })
        })
}

#[cfg(test)]
mod tests {
    use floom::IntoView;
    use floom::headless::{HeadlessHarness, TestRoot};
    use floom::reactive::SignalGet;
    use floom::views::{Decorators, Label, Stack, dyn_view};
    use floom_widget_runtime::{design_system_signal, set_design_system, set_dark, theme_signal, DesignSystem};
    use floom_core_theme::theme::Theme;

    /// Reproduces the startup panic (floom `reactive/src/read.rs` `Option::unwrap`
    /// on a `None` signal) without opening a real window.
    ///
    /// Plain construction + a single `rebuild()` is not enough: the real
    /// `Application::run` event loop runs many update passes and reactive
    /// effects, which cause the `dyn_view`s to dispose and rebuild their child
    /// scopes. A signal created in one of those child scopes and read after the
    /// scope is disposed triggers the panic. Cycling the active Design System
    /// forces the `dyn_view` below to re-run (dispose + recreate), which is what
    /// surfaces the bug headlessly.
    ///
    /// NOTE: this builds a *minimal* subtree, not the full `app_view()`. The
    /// full gallery renders its floating surfaces (Dialog/Menu/Snackbar/
    /// NavigationDrawer/Sheet/Overlay) as window-root `Overlay`s, which reparent
    /// their view ids to a window root that floom's headless harness does not
    /// register during `commit_box_tree` — so the full gallery cannot be
    /// exercised headlessly. The real `Application::run` uses a proper window
    /// root and is unaffected. This test still guards the critical reactive
    /// global-signal regression.
    #[test]
    fn headless_build_does_not_panic() {
        let root = TestRoot::new();
        let theme = theme_signal();
        let view = Stack::vertical((
            dyn_view(move || {
                let _ds = design_system_signal().get();
                Label::new("smoke")
                    .style(move |s| {
                        s.color(theme.get().colors().on_surface.to_peniko())
                    })
                    .into_any()
            }),
        ))
        .style(|s| s.width_pct(100.0).height_pct(100.0))
        .into_any();
        let mut harness = HeadlessHarness::new(root, view);
        harness.rebuild();
        harness.rebuild();

        for ds in [
            DesignSystem::Cupertino,
            DesignSystem::Fluent,
            DesignSystem::Material,
        ] {
            set_design_system(ds);
            harness.rebuild();
            harness.rebuild();
        }

        // Also exercise the dark-mode global signal path.
        set_dark(true);
        harness.rebuild();
        set_dark(false);
        harness.rebuild();
    }
}
