pub use floem_core_theme::component_tokens::{ButtonTokens, ComponentId};
pub use floem_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Button` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_button::CupertinoButton as Button;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_button::CupertinoButtonStyle as ButtonStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_button::FluentButton as Button;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_button::FluentButtonStyle as ButtonStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_button::MdButton as Button;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_button::MdButtonStyle as ButtonStyle;

/// Runtime-dispatch `Button` for the `multi` feature.
///
/// It stores the neutral builder state and, at `view()` time, reads the active
/// Design System to construct the matching concrete button (Material /
/// Cupertino / Fluent). The selected page in the demo is rebuilt whenever the
/// Design System signal changes, so the `view()` call picks up the new DS.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::button::{ButtonSize, ButtonVariant};

    use floem_widget_md_button::MdButton;
    use floem_widget_cupertino_button::CupertinoButton;
    use floem_widget_fluent_button::FluentButton;

    pub struct Button {
        label: String,
        variant: ButtonVariant,
        size: ButtonSize,
        disabled: bool,
        loading: bool,
        on_click: Option<Box<dyn Fn()>>,
    }

    impl Button {
        pub fn new(label: impl Into<String>) -> Self {
            Self {
                label: label.into(),
                variant: ButtonVariant::Primary,
                size: ButtonSize::Medium,
                disabled: false,
                loading: false,
                on_click: None,
            }
        }

        pub fn variant(mut self, variant: ButtonVariant) -> Self {
            self.variant = variant;
            self
        }

        pub fn size(mut self, size: ButtonSize) -> Self {
            self.size = size;
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }

        pub fn loading(mut self, loading: bool) -> Self {
            self.loading = loading;
            self
        }

        pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_click = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_click = self
                .on_click
                .unwrap_or_else(|| Box::new(|| {}));
            match ds {
                DesignSystem::Material => MdButton::new(self.label)
                    .variant(self.variant)
                    .size(self.size)
                    .disabled(self.disabled)
                    .loading(self.loading)
                    .on_click(on_click)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoButton::new(self.label)
                    .variant(self.variant)
                    .size(self.size)
                    .disabled(self.disabled)
                    .loading(self.loading)
                    .on_click(on_click)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentButton::new(self.label)
                    .variant(self.variant)
                    .size(self.size)
                    .disabled(self.disabled)
                    .loading(self.loading)
                    .on_click(on_click)
                    .view()
                    .into_any(),
            }
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Button;
