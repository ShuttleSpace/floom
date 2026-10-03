pub use floem_core_theme::component_tokens::TextTokens;
pub use floem_core_theme::props::text::TextProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `Text` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_text::CupertinoText as Text;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_text::CupertinoTextStyle as TextStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_text::FluentText as Text;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_text::FluentTextStyle as TextStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_text::MdText as Text;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_text::MdTextStyle as TextStyle;

/// Runtime-dispatch `Text` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_text::MdText;
    use floem_widget_cupertino_text::CupertinoText;
    use floem_widget_fluent_text::FluentText;

    pub struct Text {
        content: String,
    }

    impl Text {
        pub fn new(content: impl Into<String>) -> Self {
            Self {
                content: content.into(),
            }
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdText::new(self.content).view().into_any(),
                DesignSystem::Cupertino => CupertinoText::new(self.content).view().into_any(),
                DesignSystem::Fluent => FluentText::new(self.content).view().into_any(),
            }
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Text;
