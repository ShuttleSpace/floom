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
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::empty;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::sheet::SheetSide;

    use floem_widget_md_sheet::MdSheet;
    use floem_widget_cupertino_sheet::CupertinoSheet;
    use floem_widget_fluent_sheet::FluentSheet;

    pub struct Sheet {
        side: SheetSide,
        open: bool,
        elevated: bool,
        child: Option<AnyView>,
    }

    impl Sheet {
        pub fn new() -> Self {
            Self {
                side: SheetSide::Bottom,
                open: false,
                elevated: true,
                child: None,
            }
        }

        pub fn side(mut self, side: SheetSide) -> Self {
            self.side = side;
            self
        }

        pub fn open(mut self, open: bool) -> Self {
            self.open = open;
            self
        }

        pub fn elevated(mut self, elevated: bool) -> Self {
            self.elevated = elevated;
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| empty().into_any());
            match ds {
                DesignSystem::Material => MdSheet::new()
                    .side(self.side)
                    .open(self.open)
                    .elevated(self.elevated)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSheet::new()
                    .side(self.side)
                    .open(self.open)
                    .elevated(self.elevated)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSheet::new()
                    .side(self.side)
                    .open(self.open)
                    .elevated(self.elevated)
                    .child(child)
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
