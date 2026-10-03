pub use floem_core_theme::component_tokens::SnackbarTokens;
pub use floem_core_theme::props::snackbar::SnackbarProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Snackbar` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_snackbar::CupertinoSnackbar as Snackbar;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_snackbar::CupertinoSnackbarStyle as SnackbarStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_snackbar::FluentSnackbar as Snackbar;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_snackbar::FluentSnackbarStyle as SnackbarStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_snackbar::MdSnackbar as Snackbar;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_snackbar::MdSnackbarStyle as SnackbarStyle;

/// Runtime-dispatch `Snackbar` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet};
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::snackbar::SnackbarProps;

    use floem_widget_md_snackbar::MdSnackbar;
    use floem_widget_cupertino_snackbar::CupertinoSnackbar;
    use floem_widget_fluent_snackbar::FluentSnackbar;

    pub struct Snackbar {
        props: SnackbarProps,
        on_action: Option<Box<dyn Fn()>>,
        open: RwSignal<bool>,
    }

    impl Snackbar {
        pub fn new() -> Self {
            Self {
                props: SnackbarProps::default(),
                on_action: None,
                open: RwSignal::new(false),
            }
        }

        pub fn message(mut self, message: impl Into<String>) -> Self {
            self.props.message = message.into();
            self
        }

        pub fn action_label(mut self, action_label: Option<String>) -> Self {
            self.props.action_label = action_label;
            self
        }

        pub fn on_action(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_action = Some(Box::new(callback));
            self
        }

        pub fn duration_ms(mut self, duration_ms: u32) -> Self {
            self.props.duration_ms = duration_ms;
            self
        }

        /// Expose the internal open signal so an external trigger can show/hide.
        pub fn open_signal(&self) -> RwSignal<bool> {
            self.open
        }

        /// Replace the internal open signal with an external one so a trigger
        /// (built separately, e.g. on the same page) can show/hide the snackbar.
        pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
            self.open = open;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_action = self.on_action.unwrap_or_else(|| Box::new(|| {}));
            match ds {
                DesignSystem::Material => MdSnackbar::new()
                    .message(self.props.message.clone())
                    .action_label(self.props.action_label.clone())
                    .on_action(on_action)
                    .duration_ms(self.props.duration_ms)
                    .with_open_signal(self.open)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSnackbar::new()
                    .message(self.props.message.clone())
                    .action_label(self.props.action_label.clone())
                    .on_action(on_action)
                    .duration_ms(self.props.duration_ms)
                    .with_open_signal(self.open)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSnackbar::new()
                    .message(self.props.message.clone())
                    .action_label(self.props.action_label.clone())
                    .on_action(on_action)
                    .duration_ms(self.props.duration_ms)
                    .with_open_signal(self.open)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Snackbar {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Snackbar;
#[cfg(feature = "multi")]
pub use floem_widget_md_snackbar::MdSnackbarStyle as SnackbarStyle;
