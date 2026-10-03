use std::time::Duration;

use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::scale_transition::ScaleTransitionProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdScaleTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
    pub min_scale: f32,
}

pub struct MdScaleTransition {
    props: ScaleTransitionProps,
    child: Option<Box<dyn Fn() -> AnyView>>,
    theme: RwSignal<MultiTheme>,
    show: RwSignal<bool>,
}

impl MdScaleTransition {
    pub fn new() -> Self {
        Self {
            props: ScaleTransitionProps::default(),
            child: None,
            theme: theme_signal(),
            show: RwSignal::new(false),
        }
    }

    /// Set the element that scales in / out. Takes a factory closure because the child is
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

    /// Set the internal `show` signal from an external signal.
    pub fn with_show_signal(mut self, show: RwSignal<bool>) -> Self {
        self.show = show;
        self
    }

    pub fn resolved_style(&self) -> MdScaleTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::ScaleTransition(tokens) =
            theme.component(ComponentId::ScaleTransition)
        else {
            unreachable!("ComponentId::ScaleTransition must resolve to ScaleTransitionTokens")
        };
        MdScaleTransitionStyle {
            design_system: "material",
            duration_ms: tokens.duration_ms,
            min_scale: tokens.min_scale,
        }
    }

    pub fn view(self) -> impl View {
        let MdScaleTransitionStyle { duration_ms, min_scale, .. } = self.resolved_style();
        let show = self.show;
        let child_factory = self.child;
        let start = (min_scale * 100.0).pct();

        dyn_view(move || -> AnyView {
            if show.get() {
                let child = match &child_factory {
                    Some(f) => f(),
                    None => Empty::new().into_any(),
                };
                // `move` + Copy `duration_ms`/`start` so the stored animation closure is 'static.
                Container::new(child)
                    .animation(move |a| {
                        a.run_on_create(true)
                            .run_on_remove(true)
                            .duration(Duration::from_millis(duration_ms as u64))
                            .keyframe(0, |f| f.style(|s| s.scale(start)))
                            .keyframe(100, |f| f.style(|s| s.scale(100.pct())))
                    })
                    .into_any()
            } else {
                Empty::new().into_any()
            }
        })
    }
}

impl Default for MdScaleTransition {
    fn default() -> Self {
        Self::new()
    }
}
