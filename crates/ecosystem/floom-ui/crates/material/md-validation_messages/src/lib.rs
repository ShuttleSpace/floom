use floom::prelude::*;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::validation_messages::ValidationMessagesProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdValidationMessagesStyle {
    pub error_color: Color,
    pub font_size: f32,
    pub gap: f32,
}

pub struct MdValidationMessages {
    props: ValidationMessagesProps,
    theme: RwSignal<MultiTheme>,
    /// Reactive list of error strings. Empty means "no messages".
    messages: RwSignal<Vec<String>>,
}

impl MdValidationMessages {
    pub fn new() -> Self {
        Self {
            props: ValidationMessagesProps::default(),
            theme: theme_signal(),
            messages: RwSignal::new(Vec::new()),
        }
    }

    /// Expose the internal messages signal so the consumer can push/pull errors
    /// reactively (mirrors the `open_signal()` pattern from `Dialog`/`Menu`).
    pub fn messages_signal(&self) -> RwSignal<Vec<String>> {
        self.messages
    }

    /// Convenience: set a static list of messages immediately.
    pub fn messages(self, messages: Vec<String>) -> Self {
        self.messages.set(messages);
        self
    }

    /// Override the gap between messages.
    pub fn gap(mut self, gap: f32) -> Self {
        self.props.gap = gap;
        self
    }

    /// Override the per-message font size.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.props.font_size = font_size;
        self
    }

    pub fn props(&self) -> &ValidationMessagesProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> MdValidationMessagesStyle {
        let theme = self.theme.get();
        let ComponentTokens::ValidationMessages(tokens) =
            theme.component(ComponentId::ValidationMessages)
        else {
            unreachable!("ComponentId::ValidationMessages must resolve to ValidationMessagesTokens")
        };
        MdValidationMessagesStyle {
            error_color: theme.colors().error,
            font_size: tokens.font_size,
            gap: tokens.gap,
        }
    }

    /// Render the reactive error list in the theme `error` colour. When the
    /// bound signal is empty, this collapses to a zero-height empty view.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let messages = self.messages;
        let gap = self.props.gap;
        let font_size = self.props.font_size;

        dyn_view(move || {
            let msgs = messages.get();
            let theme = theme;
            let items: Vec<AnyView> = msgs
                .iter()
                .map(|m| {
                    Label::new(m.clone())
                        .style(move |s| {
                            s.font_size(font_size)
                                .color(theme.get().colors().error.to_peniko())
                        })
                        .into_any()
                })
                .collect();
            list(items)
                .style(move |s| s.gap(gap))
                .into_any()
        })
    }
}

impl Default for MdValidationMessages {
    fn default() -> Self {
        Self::new()
    }
}
