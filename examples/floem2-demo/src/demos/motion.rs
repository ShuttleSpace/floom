//! Motion: the implicit (value-changed) and presence (existence-changed)
//! animation layer.
//!
//! Every widget here is mounted at rest; the host runs the enter transitions on
//! its first frames. Nothing in the gallery drives them — that is the point of
//! the implicit layer, and it is what makes these demos paint instead of
//! needing a script.

use floem_core::{
    Axis, Child, ChildKey, Easing, LayoutRequest, PresenceTransition, Transition, WidgetAny,
};
use floem_widgets::{
    AnimatedContent, Button, Crossfade, Opacity, SharedTransitionLayout, Stack, Text,
};

use super::{Demo, Section};

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "crossfade",
            title: "Crossfade",
            section: Section::Motion,
            build: build_crossfade,
        },
        Demo {
            id: "animated-content",
            title: "AnimatedContent",
            section: Section::Motion,
            build: build_animated_content,
        },
        Demo {
            id: "opacity",
            title: "Opacity",
            section: Section::Motion,
            build: build_opacity,
        },
        Demo {
            id: "implicit-transition",
            title: "Implicit transition",
            section: Section::Motion,
            build: build_implicit_transition,
        },
        Demo {
            id: "shared-transition",
            title: "SharedTransitionLayout",
            section: Section::Motion,
            build: build_shared_transition,
        },
    ]
}

fn label(key: usize, text: &'static str) -> Child {
    Child::new(ChildKey::index(key), LayoutRequest::new(), move || {
        Box::new(Text::new(text))
    })
}

fn build_crossfade() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(LayoutRequest::new().height(40.0), || {
            Box::new(
                Crossfade::new(true)
                    .a(label(0, "side A"))
                    .b(label(1, "side B"))
                    .presence(PresenceTransition::fade(0.2)),
            )
        }),
    )
}

fn build_animated_content() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(LayoutRequest::new().height(40.0), || {
            Box::new(
                AnimatedContent::new(1u32)
                    .content_with(|value: &u32| {
                        let text = format!("content v{value}");
                        Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
                            Box::new(Text::new(text.clone()))
                        })
                    })
                    .presence(PresenceTransition::fade(0.2)),
            )
        }),
    )
}

fn build_opacity() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .push_layout(LayoutRequest::new().height(24.0), || {
                Box::new(
                    Opacity::new(label(0, "opaque"))
                        .opacity(1.0)
                        .transition(Transition::new(0.2)),
                )
            })
            .push_layout(LayoutRequest::new().height(24.0), || {
                Box::new(
                    Opacity::new(label(0, "half"))
                        .opacity(0.5)
                        .transition(Transition::new(0.2)),
                )
            })
            .push_layout(LayoutRequest::new().height(24.0), || {
                Box::new(
                    Opacity::new(label(0, "faint"))
                        .opacity(0.25)
                        .transition(Transition::new(0.2)),
                )
            }),
    )
}

/// A control that animates its own value change: `Button` armed with a
/// `Transition`, so a hover or state change fades instead of snapping.
fn build_implicit_transition() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(
                    Button::new("Fades")
                        .transition(Transition::new(0.25).with_easing(Easing::EASE_OUT))
                        .on_click(|| {}),
                )
            })
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(Button::new("Snaps").on_click(|| {}))
            }),
    )
}

/// At rest a shared-element layout shows its source; arming the flight is the
/// host's job, so the gallery shows the resting half.
fn build_shared_transition() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .push_layout(LayoutRequest::new().width(200.0).height(90.0), || {
                Box::new(
                    SharedTransitionLayout::new(7, 40.0)
                        .duration(0.4)
                        .source(label(0, "source"))
                        .dest(label(1, "destination")),
                )
            }),
    )
}
