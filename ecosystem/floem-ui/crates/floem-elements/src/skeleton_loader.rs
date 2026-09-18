pub use floem_core_theme::component_tokens::SkeletonTokens;
pub use floem_core_theme::props::skeleton_loader::{SkeletonLoaderProps, SkeletonType};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_skeleton_loader::CupertinoSkeletonLoader as SkeletonLoader;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_skeleton_loader::CupertinoSkeletonStyle as SkeletonLoaderStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_skeleton_loader::FluentSkeletonLoader as SkeletonLoader;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_skeleton_loader::FluentSkeletonStyle as SkeletonLoaderStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_skeleton_loader::MdSkeletonLoader as SkeletonLoader;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_skeleton_loader::MdSkeletonStyle as SkeletonLoaderStyle;

/// Runtime-dispatch `SkeletonLoader` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::skeleton_loader::SkeletonType;

    use floem_widget_md_skeleton_loader::MdSkeletonLoader;
    use floem_widget_cupertino_skeleton_loader::CupertinoSkeletonLoader;
    use floem_widget_fluent_skeleton_loader::FluentSkeletonLoader;

    pub struct SkeletonLoader {
        skeleton_type: SkeletonType,
        width: Option<f32>,
        height: Option<f32>,
        lines: usize,
        animated: bool,
    }

    impl SkeletonLoader {
        pub fn new() -> Self {
            Self {
                skeleton_type: SkeletonType::Text,
                width: None,
                height: None,
                lines: 3,
                animated: true,
            }
        }

        pub fn skeleton_type(mut self, skeleton_type: SkeletonType) -> Self {
            self.skeleton_type = skeleton_type;
            self
        }

        pub fn width(mut self, width: f32) -> Self {
            self.width = Some(width);
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.height = Some(height);
            self
        }

        pub fn lines(mut self, lines: usize) -> Self {
            self.lines = lines;
            self
        }

        pub fn animated(mut self, animated: bool) -> Self {
            self.animated = animated;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdSkeletonLoader::new()
                        .skeleton_type(self.skeleton_type)
                        .lines(self.lines)
                        .animated(self.animated);
                    if let Some(w) = self.width {
                        c = c.width(w);
                    }
                    if let Some(h) = self.height {
                        c = c.height(h);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoSkeletonLoader::new()
                        .skeleton_type(self.skeleton_type)
                        .lines(self.lines)
                        .animated(self.animated);
                    if let Some(w) = self.width {
                        c = c.width(w);
                    }
                    if let Some(h) = self.height {
                        c = c.height(h);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentSkeletonLoader::new()
                        .skeleton_type(self.skeleton_type)
                        .lines(self.lines)
                        .animated(self.animated);
                    if let Some(w) = self.width {
                        c = c.width(w);
                    }
                    if let Some(h) = self.height {
                        c = c.height(h);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for SkeletonLoader {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::SkeletonLoader;
#[cfg(feature = "multi")]
pub use floem_widget_md_skeleton_loader::MdSkeletonStyle as SkeletonLoaderStyle;
