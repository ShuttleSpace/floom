use std::time::Duration;

use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::taffy::style::Overflow;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::expand_transition::ExpandTransitionProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdExpandTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
}

pub struct MdExpandTransition {
    props: ExpandTransitionProps,
    child: Option<Box<dyn Fn() -> AnyView>>,
    theme: RwSignal<MultiTheme>,
    show: RwSignal<bool>,
}

impl MdExpandTransition {
    pub fn new() -> Self {
        Self {
            props: ExpandTransitionProps::default(),
            child: None,
            theme: theme_signal(),
            show: RwSignal::new(false),
        }
    }

    /// Set the element that expands / collapses. Takes a factory closure because the child is
    /// mounted and unmounted (re-created) on every enter/leave, so it must be rebuildable.
    pub fn child<F, IV>(mut self, child: F) -> Self
    where
        F: Fn() -> IV + 'static,
        IV: IntoView + 'static,
    {
        self.child = Some(Box::new(move || child().into_any()));
        self
    }

    /// Expose the internal show signal so an external trigger (e.g. a button) can enter or
    /// leave the transition. Mirrors `Dialog::open_signal()`.
    pub fn show_signal(&self) -> RwSignal<bool> {
        self.show
    }

    pub fn resolved_style(&self) -> MdExpandTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::ExpandTransition(tokens) =
            theme.component(ComponentId::ExpandTransition)
        else {
            unreachable!("ComponentId::ExpandTransition must resolve to ExpandTransitionTokens")
        };
        MdExpandTransitionStyle {
            design_system: "material",
            duration_ms: tokens.duration_ms,
        }
    }

    pub fn view(self) -> impl View {
        let MdExpandTransitionStyle { duration_ms, .. } = self.resolved_style();
        let show = self.show;
        let child_factory = self.child;

        dyn_view(move || -> AnyView {
            if show.get() {
                let child = match &child_factory {
                    Some(f) => f(),
                    None => Empty::new().into_any(),
                };
                // Height animates 0 -> natural (computed). Overflow is clipped so the
                // content does not spill out of the collapsing box during the transition.
                // `move` + Copy `duration_ms` keeps the stored animation closure 'static.
                Container::new(child)
                    .style(|s| s.overflow_x(Overflow::Clip).overflow_y(Overflow::Clip))
                    .animation(move |a| {
                        a.run_on_create(true)
                            .run_on_remove(true)
                            .duration(Duration::from_millis(duration_ms as u64))
                            .keyframe(0, |f| f.style(|s| s.height(0.0)))
                            .keyframe(100, |f| f.computed_style())
                    })
                    .into_any()
            } else {
                Empty::new().into_any()
            }
        })
    }
}

impl Default for MdExpandTransition {
    fn default() -> Self {
        Self::new()
    }
}
