use std::time::Duration;

use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::fade_transition::FadeTransitionProps;
use floem_core_theme::theme::{MultiTheme, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFadeTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
}

pub struct FluentFadeTransition {
    props: FadeTransitionProps,
    child: Option<Box<dyn Fn() -> AnyView>>,
    theme: RwSignal<MultiTheme>,
    show: RwSignal<bool>,
}

impl FluentFadeTransition {
    pub fn new() -> Self {
        Self {
            props: FadeTransitionProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            show: RwSignal::new(false),
        }
    }

    /// Set the element that fades in / out. Takes a factory closure because the child is
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

    pub fn resolved_style(&self) -> FluentFadeTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::FadeTransition(tokens) =
            theme.component(ComponentId::FadeTransition)
        else {
            unreachable!("ComponentId::FadeTransition must resolve to FadeTransitionTokens")
        };
        FluentFadeTransitionStyle {
            design_system: "fluent",
            duration_ms: tokens.duration_ms,
        }
    }

    pub fn view(self) -> impl View {
        let FluentFadeTransitionStyle { duration_ms, .. } = self.resolved_style();
        let show = self.show;
        let child_factory = self.child;

        dyn_view(move || -> AnyView {
            if show.get() {
                let child = match &child_factory {
                    Some(f) => f(),
                    None => Empty::new().into_any(),
                };
                // `move` + a Copy `duration_ms` so the stored animation closure is 'static
                // (otherwise it would borrow `style` from this outer closure).
                Container::new(child)
                    .animation(move |a| {
                        a.run_on_create(true)
                            .run_on_remove(true)
                            .duration(Duration::from_millis(duration_ms as u64))
                            .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                            .keyframe(100, |f| f.style(|s| s.opacity(1.0)))
                    })
                    .into_any()
            } else {
                Empty::new().into_any()
            }
        })
    }
}

impl Default for FluentFadeTransition {
    fn default() -> Self {
        Self::new()
    }
}
