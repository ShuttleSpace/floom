//! Cross-design-system component gallery built on floem-elements.
//!
//! The same neutral-API source renders every component for Material, Cupertino,
//! or Fluent depending on the active feature:
//!   cargo run -p floem-demo                      # Material (default)
//!   cargo run -p floem-demo --no-default-features --features cupertino
//!   cargo run -p floem-demo --no-default-features --features fluent
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

#[cfg(feature = "material")]
const DS_NAME: &str = "Material 3";
#[cfg(feature = "cupertino")]
const DS_NAME: &str = "Cupertino";
#[cfg(feature = "fluent")]
const DS_NAME: &str = "Fluent";
#[cfg(feature = "multi")]
const DS_NAME: &str = "Multi-DS";

use floem::IntoView;
use floem::View;
use floem::Application;
use floem::peniko::kurbo::Size;
use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem::views::{Decorators, Scroll, dyn_view, h_stack, v_stack, v_stack_from_iter};
use floem::window::WindowConfig;
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color_ext::with_alpha;
use floem_elements::button::ButtonVariant;
use floem_elements::prelude::*;
use floem_widget_runtime::{design_system_signal, set_design_system, theme_signal, DesignSystem};

fn main() {
    let view = move |_| {
        // `build()` owns the persistent overlay signals, the two
        // `create_effect`s (Form / PullToRefresh), and returns the master list
        // plus the root overlays. It must run here, inside the window view
        // factory, so the reactive scope is active when effects are created.
        let Gallery { pages, overlays } = build();

        let selected = RwSignal::new(0usize);
        let theme = theme_signal();

        // ---- LEFT pane: clickable component list ----
        let left = Scroll::new(
            v_stack_from_iter(pages.iter().enumerate().map(|(i, (name, _))| {
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
            .style(|s| s.gap(2.0).padding(8.0)),
        )
        .style(|s| s.width(240.0).height_pct(100.0));

        // ---- RIGHT pane: selected component demo (scrollable) ----
        // Reading `design_system_signal()` here makes the closure re-run (and
        // rebuild the selected page with the new DS components) whenever the
        // active Design System changes at runtime.
        let right = Scroll::new(dyn_view(move || {
            let _ds = design_system_signal().get();
            let idx = selected.get();
            pages[idx].1()
        }))
        .style(|s| s.flex_grow(1.0).height_pct(100.0).padding(20.0));

        let split = h_stack((left, right)).style(|s| s.width_pct(100.0).height_pct(100.0));

        // ---- Top-right Design System switcher (absolutely positioned overlay) ----
        let ds_now = design_system_signal();
        let ds_segment = h_stack((
            ds_switch_button(DesignSystem::Material, ds_now, theme),
            ds_switch_button(DesignSystem::Cupertino, ds_now, theme),
            ds_switch_button(DesignSystem::Fluent, ds_now, theme),
        ))
        .style(move |s| {
            s.absolute()
                .inset_top(12.0)
                .inset_right(12.0)
                .gap(4.0)
                .padding(4.0)
                .border_radius(10.0)
                .background(theme.get().colors().surface_variant.to_peniko())
        });

        // ---- Root overlays: absolutely positioned, full-window floating surfaces ----
        let overlays_view = v_stack_from_iter(overlays.into_iter().map(|ov| {
            floem::views::Container::new(ov)
                .style(|s| s.absolute().inset_top(0.0).inset_left(0.0).width_full().height_full())
                .into_any()
        }))
        .style(|s| s.absolute().inset_top(0.0).inset_left(0.0).width_full().height_full());

        let inner = v_stack((split, overlays_view, ds_segment))
            .style(|s| s.width_pct(100.0).height_pct(100.0));
        floem::views::Container::new(inner).style(|s| s.width_pct(100.0).height_pct(100.0))
    };

    Application::new()
        .window(
            view,
            Some(
                WindowConfig::default()
                    .size(Size::new(1040.0, 900.0))
                    .title(format!("floem-ui · {} gallery", DS_NAME)),
            ),
        )
        .run();
}

/// Builds a single button in the top-right Design System switcher. The button
/// is highlighted when its `DesignSystem` matches the active one, and clicking
/// it calls `set_design_system` (which bumps the global theme signal so every
/// component re-skins).
fn ds_switch_button(
    ds: DesignSystem,
    ds_now: RwSignal<DesignSystem>,
    theme: RwSignal<floem_core_theme::theme::MultiTheme>,
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
