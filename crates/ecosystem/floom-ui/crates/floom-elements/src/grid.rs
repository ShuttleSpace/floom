pub use floom_core_theme::component_tokens::GridTokens;
pub use floom_core_theme::props::grid::GridProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_grid::CupertinoGrid as Grid;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_grid::CupertinoGridStyle as GridStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_grid::FluentGrid as Grid;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_grid::FluentGridStyle as GridStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_grid::MdGrid as Grid;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_grid::MdGridStyle as GridStyle;

/// Runtime-dispatch `Grid` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::AnyView;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floom_widget_md_grid::MdGrid;
    use floom_widget_cupertino_grid::CupertinoGrid;
    use floom_widget_fluent_grid::FluentGrid;

    pub struct Grid {
        cols: usize,
        gap: f32,
        row_gap: f32,
        children: Vec<AnyView>,
    }

    impl Grid {
        pub fn new() -> Self {
            Self {
                cols: 12,
                gap: 12.0,
                row_gap: 12.0,
                children: Vec::new(),
            }
        }

        pub fn cols(mut self, cols: usize) -> Self {
            self.cols = cols;
            self
        }

        pub fn gap(mut self, gap: f32) -> Self {
            self.gap = gap;
            self
        }

        pub fn row_gap(mut self, row_gap: f32) -> Self {
            self.row_gap = row_gap;
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.children.push(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdGrid::new()
                        .cols(self.cols)
                        .gap(self.gap)
                        .row_gap(self.row_gap);
                    for child in self.children {
                        c = c.child(child);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoGrid::new()
                        .cols(self.cols)
                        .gap(self.gap)
                        .row_gap(self.row_gap);
                    for child in self.children {
                        c = c.child(child);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentGrid::new()
                        .cols(self.cols)
                        .gap(self.gap)
                        .row_gap(self.row_gap);
                    for child in self.children {
                        c = c.child(child);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for Grid {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Grid;
#[cfg(feature = "multi")]
pub use floom_widget_md_grid::MdGridStyle as GridStyle;
