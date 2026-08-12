use std::time::Duration;

use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::slide_transition::{SlideDirection, SlideTransitionProps};
use floem_core_theme::theme::{MultiTheme, Theme};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSlideTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
    pub offset: f32,
    pub direction: SlideDirection,
}

pub struct FluentSlideTransition {
    props: SlideTransitionProps,
    child: Option<Box<dyn Fn() -> AnyView>>,
    theme: RwSignal<MultiTheme>,
    show: RwSignal<bool>,
}

impl FluentSlideTransition {
    pub fn new() -> Self {
        Self {
            props: SlideTransitionProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
            show: RwSignal::new(false),
        }
    }

    /// Set the element that slides in / out. Takes a factory closure because the child is
    /// mounted and unmounted (re-created) on every enter/leave, so it must be rebuildable.
    pub fn child<F, IV>(mut self, child: F) -> Self
    where
        F: Fn() -> IV + 'static,
        IV: IntoView + 'static,
    {
        self.child = Some(Box::new(move || child().into_any()));
        self
    }

    /// Choose the travel axis (X = horizontal, Y = vertical). Defaults to X.
    pub fn direction(mut self, direction: SlideDirection) -> Self {
        self.props.direction = direction;
        self
    }

    /// Expose the internal show signal so an external trigger (e.g. a button) can enter or
    /// leave the transition. Mirrors `Dialog::open_signal()`.
    pub fn show_signal(&self) -> RwSignal<bool> {
        self.show
    }

    pub fn resolved_style(&self) -> FluentSlideTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::SlideTransition(tokens) =
            theme.component(ComponentId::SlideTransition)
        else {
            unreachable!("ComponentId::SlideTransition must resolve to SlideTransitionTokens")
        };
        FluentSlideTransitionStyle {
            design_system: "fluent",
            duration_ms: tokens.duration_ms,
            offset: tokens.offset,
            direction: self.props.direction,
        }
    }

    pub fn view(self) -> impl View {
        let style = self.resolved_style();
        let show = self.show;
        let child_factory = self.child;
        // Pre-extract the Copy fields so the animation closure owns copies ('static)
        // instead of borrowing `style` from the outer closure.
        let direction = style.direction;
        let offset = style.offset;

        dyn_view(move || -> AnyView {
            if show.get() {
                let child = match &child_factory {
                    Some(f) => f(),
                    None => Empty::new().into_any(),
                };
                // Start off-screen by `offset` px on the chosen axis, settle at 0.
                Container::new(child)
                    .animation(move |a| {
                        a.run_on_create(true)
                            .run_on_remove(true)
                            .duration(Duration::from_millis(style.duration_ms as u64))
                            .keyframe(0, |f| {
                                f.style(|s| match direction {
                                    SlideDirection::X => s.translate_x(offset),
                                    SlideDirection::Y => s.translate_y(offset),
                                })
                            })
                            .keyframe(100, |f| f.computed_style())
                    })
                    .into_any()
            } else {
                Empty::new().into_any()
            }
        })
    }
}

impl Default for FluentSlideTransition {
    fn default() -> Self {
        Self::new()
    }
}
