pub use floem_core_theme::component_tokens::SheetTokens;
pub use floem_core_theme::props::sheet::{SheetProps, SheetSide};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_sheet::CupertinoSheet as Sheet;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_sheet::CupertinoSheetStyle as SheetStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_sheet::FluentSheet as Sheet;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_sheet::FluentSheetStyle as SheetStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_sheet::MdSheet as Sheet;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_sheet::MdSheetStyle as SheetStyle;

/// Runtime-dispatch `Sheet` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::{empty, Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::sheet::SheetSide;

    use floem_widget_md_sheet::MdSheet;
    use floem_widget_cupertino_sheet::CupertinoSheet;
    use floem_widget_fluent_sheet::FluentSheet;

    pub struct Sheet {
        side: SheetSide,
        open: RwSignal<bool>,
        elevated: bool,
        child: Option<Rc<dyn Fn() -> AnyView>>,
    }

    impl Sheet {
        pub fn new() -> Self {
            Self {
                side: SheetSide::Bottom,
                open: RwSignal::new(false),
                elevated: true,
                child: None,
            }
        }

        pub fn side(mut self, side: SheetSide) -> Self {
            self.side = side;
            self
        }

        pub fn open(mut self, open: bool) -> Self {
            self.open.set(open);
            self
        }

        /// Expose the internal open signal so an external trigger can show/hide.
        pub fn open_signal(&self) -> RwSignal<bool> {
            self.open
        }

        /// Replace the internal open signal with an external one so a trigger
        /// (built separately, e.g. on the same page) can show/hide the sheet.
        pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
            self.open = open;
            self
        }

        pub fn elevated(mut self, elevated: bool) -> Self {
            self.elevated = elevated;
            self
        }

        /// Accept a child factory so the content can be rebuilt for each
        /// concrete DS in the `view()` match below.
        pub fn child<F, IV>(mut self, child: F) -> Self
        where
            F: Fn() -> IV + 'static,
            IV: IntoView + 'static,
        {
            self.child = Some(Rc::new(move || child().into_any()));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child;
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => MdSheet::new()
                    .side(self.side)
                    .with_open_signal(self.open)
                    .elevated(self.elevated)
                    .child(child_factory())
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSheet::new()
                    .side(self.side)
                    .with_open_signal(self.open)
                    .elevated(self.elevated)
                    .child(child_factory())
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSheet::new()
                    .side(self.side)
                    .with_open_signal(self.open)
                    .elevated(self.elevated)
                    .child(child_factory())
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Sheet {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Sheet;
#[cfg(feature = "multi")]
pub use floem_widget_md_sheet::MdSheetStyle as SheetStyle;
