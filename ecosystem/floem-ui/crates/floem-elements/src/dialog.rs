pub use floem_core_theme::component_tokens::DialogTokens;
pub use floem_core_theme::props::dialog::DialogProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_dialog::CupertinoDialog as Dialog;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_dialog::CupertinoDialogStyle as DialogStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_dialog::FluentDialog as Dialog;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_dialog::FluentDialogStyle as DialogStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_dialog::MdDialog as Dialog;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_dialog::MdDialogStyle as DialogStyle;

/// Runtime-dispatch `Dialog` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_dialog::MdDialog;
    use floem_widget_cupertino_dialog::CupertinoDialog;
    use floem_widget_fluent_dialog::FluentDialog;

    pub struct Dialog {
        title: String,
        content: String,
        open: RwSignal<bool>,
        on_close: Option<Box<dyn Fn()>>,
        actions: Vec<(String, Box<dyn Fn()>)>,
        theme: Option<MultiTheme>,
    }

    impl Dialog {
        pub fn new() -> Self {
            Self {
                title: String::new(),
                content: String::new(),
                open: RwSignal::new(false),
                on_close: None,
                actions: Vec::new(),
                theme: None,
            }
        }

        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.title = title.into();
            self
        }

        pub fn content(mut self, content: impl Into<String>) -> Self {
            self.content = content.into();
            self
        }

        pub fn open(mut self, open: bool) -> Self {
            self.open.set(open);
            self
        }

        pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_close = Some(Box::new(callback));
            self
        }

        /// Add an action button rendered at the bottom of the dialog. Clicking it
        /// fires `callback` and closes the dialog.
        pub fn action(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
            self.actions.push((label.into(), Box::new(callback)));
            self
        }

        /// Expose the internal open signal so an external trigger (e.g. a button)
        /// can show or hide the dialog.
        pub fn open_signal(&self) -> RwSignal<bool> {
            self.open
        }

        /// Replace the internal open signal with an external one so a trigger
        /// (built separately, e.g. on the same page) can show/hide the dialog.
        pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
            self.open = open;
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdDialog::new()
                        .title(self.title)
                        .content(self.content)
                        .with_open_signal(self.open)
                        .on_close(self.on_close.unwrap_or_else(|| Box::new(|| {})));
                    for (label, cb) in self.actions {
                        c = c.action(label, cb);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoDialog::new()
                        .title(self.title)
                        .content(self.content)
                        .with_open_signal(self.open)
                        .on_close(self.on_close.unwrap_or_else(|| Box::new(|| {})));
                    for (label, cb) in self.actions {
                        c = c.action(label, cb);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentDialog::new()
                        .title(self.title)
                        .content(self.content)
                        .with_open_signal(self.open)
                        .on_close(self.on_close.unwrap_or_else(|| Box::new(|| {})));
                    for (label, cb) in self.actions {
                        c = c.action(label, cb);
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for Dialog {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Dialog;
#[cfg(feature = "multi")]
pub use floem_widget_md_dialog::MdDialogStyle as DialogStyle;
