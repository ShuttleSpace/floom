pub use floom_core_theme::component_tokens::TimelineTokens;
pub use floom_core_theme::props::timeline::{TimelineItem, TimelineProps};

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Timeline` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_timeline::CupertinoTimeline as Timeline;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_timeline::CupertinoTimelineStyle as TimelineStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_timeline::FluentTimeline as Timeline;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_timeline::FluentTimelineStyle as TimelineStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_timeline::MdTimeline as Timeline;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_timeline::MdTimelineStyle as TimelineStyle;

/// Runtime-dispatch `Timeline` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::timeline::{TimelineItem, TimelineProps};

    use floom_widget_md_timeline::MdTimeline;
    use floom_widget_cupertino_timeline::CupertinoTimeline;
    use floom_widget_fluent_timeline::FluentTimeline;

    pub struct Timeline {
        props: TimelineProps,
    }

    impl Timeline {
        pub fn new() -> Self {
            Self {
                props: TimelineProps::default(),
            }
        }

        pub fn item(mut self, title: impl Into<String>, description: impl Into<String>) -> Self {
            self.props.items.push(TimelineItem {
                title: title.into(),
                description: description.into(),
            });
            self
        }

        pub fn items(mut self, items: Vec<TimelineItem>) -> Self {
            self.props.items = items;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdTimeline::new()
                    .items(self.props.items.clone())
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoTimeline::new()
                    .items(self.props.items.clone())
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentTimeline::new()
                    .items(self.props.items.clone())
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Timeline {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Timeline;
#[cfg(feature = "multi")]
pub use floom_widget_md_timeline::MdTimelineStyle as TimelineStyle;
