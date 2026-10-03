pub use floom_core_theme::component_tokens::ValidationMessagesTokens;
pub use floom_core_theme::props::validation_messages::ValidationMessagesProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_validation_messages::CupertinoValidationMessages as ValidationMessages;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_validation_messages::CupertinoValidationMessagesStyle as ValidationMessagesStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_validation_messages::FluentValidationMessages as ValidationMessages;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_validation_messages::FluentValidationMessagesStyle as ValidationMessagesStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_validation_messages::MdValidationMessages as ValidationMessages;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_validation_messages::MdValidationMessagesStyle as ValidationMessagesStyle;

/// Runtime-dispatch `ValidationMessages` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::RwSignal;
    use floom::reactive::{SignalGet, SignalUpdate};
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::validation_messages::ValidationMessagesProps;

    use floom_widget_md_validation_messages::MdValidationMessages;
    use floom_widget_cupertino_validation_messages::CupertinoValidationMessages;
    use floom_widget_fluent_validation_messages::FluentValidationMessages;

    pub struct ValidationMessages {
        messages: RwSignal<Vec<String>>,
        gap: f32,
        font_size: f32,
    }

    impl ValidationMessages {
        pub fn new() -> Self {
            let props = ValidationMessagesProps::default();
            Self {
                messages: RwSignal::new(Vec::new()),
                gap: props.gap,
                font_size: props.font_size,
            }
        }

        /// Convenience: set a static list of messages immediately.
        pub fn messages(mut self, messages: Vec<String>) -> Self {
            self.messages.set(messages);
            self
        }

        pub fn gap(mut self, gap: f32) -> Self {
            self.gap = gap;
            self
        }

        pub fn font_size(mut self, font_size: f32) -> Self {
            self.font_size = font_size;
            self
        }

        /// Expose the internal messages signal so the consumer can push/pull
        /// errors reactively (mirrors the `open_signal()` pattern).
        pub fn messages_signal(&self) -> RwSignal<Vec<String>> {
            self.messages
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdValidationMessages::new()
                    .messages(self.messages.get())
                    .gap(self.gap)
                    .font_size(self.font_size)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoValidationMessages::new()
                    .messages(self.messages.get())
                    .gap(self.gap)
                    .font_size(self.font_size)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentValidationMessages::new()
                    .messages(self.messages.get())
                    .gap(self.gap)
                    .font_size(self.font_size)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for ValidationMessages {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ValidationMessages;
#[cfg(feature = "multi")]
pub use floom_widget_md_validation_messages::MdValidationMessagesStyle as ValidationMessagesStyle;
