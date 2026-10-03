pub use floom_core_theme::component_tokens::RatingTokens;
pub use floom_core_theme::props::rating::RatingProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Rating` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_rating::CupertinoRating as Rating;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_rating::CupertinoRatingStyle as RatingStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_rating::FluentRating as Rating;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_rating::FluentRatingStyle as RatingStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_rating::MdRating as Rating;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_rating::MdRatingStyle as RatingStyle;

/// Runtime-dispatch `Rating` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::rating::RatingProps;

    use floom_widget_md_rating::MdRating;
    use floom_widget_cupertino_rating::CupertinoRating;
    use floom_widget_fluent_rating::FluentRating;

    pub struct Rating {
        props: RatingProps,
        on_change: Option<Box<dyn Fn(f32)>>,
    }

    impl Rating {
        pub fn new() -> Self {
            Self {
                props: RatingProps::default(),
                on_change: None,
            }
        }

        pub fn value(mut self, value: f32) -> Self {
            self.props.value = value;
            self
        }

        pub fn max(mut self, max: usize) -> Self {
            self.props.max = max;
            self
        }

        pub fn readonly(mut self, readonly: bool) -> Self {
            self.props.readonly = readonly;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdRating::new()
                    .value(self.props.value)
                    .max(self.props.max)
                    .readonly(self.props.readonly)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoRating::new()
                    .value(self.props.value)
                    .max(self.props.max)
                    .readonly(self.props.readonly)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentRating::new()
                    .value(self.props.value)
                    .max(self.props.max)
                    .readonly(self.props.readonly)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Rating {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Rating;
#[cfg(feature = "multi")]
pub use floom_widget_md_rating::MdRatingStyle as RatingStyle;
