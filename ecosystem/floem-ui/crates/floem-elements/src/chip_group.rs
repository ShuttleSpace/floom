pub use floem_core_theme::component_tokens::ChipGroupTokens;
pub use floem_core_theme::props::chip_group::ChipGroupProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ChipGroup` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_chip_group::CupertinoChipGroup as ChipGroup;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_chip_group::FluentChipGroup as ChipGroup;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_chip_group::MdChipGroup as ChipGroup;

/// Runtime-dispatch `ChipGroup` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::chip_group::ChipGroupProps;

    use floem_widget_md_chip_group::MdChipGroup;
    use floem_widget_cupertino_chip_group::CupertinoChipGroup;
    use floem_widget_fluent_chip_group::FluentChipGroup;

    pub struct ChipGroup {
        props: ChipGroupProps,
        labels: Vec<String>,
    }

    impl ChipGroup {
        pub fn new() -> Self {
            Self {
                props: ChipGroupProps::default(),
                labels: Vec::new(),
            }
        }

        pub fn chip(mut self, label: impl Into<String>) -> Self {
            self.labels.push(label.into());
            self
        }

        pub fn multiple(mut self, multiple: bool) -> Self {
            self.props.multiple = multiple;
            self
        }

        pub fn mandatory(mut self, mandatory: bool) -> Self {
            self.props.mandatory = mandatory;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(Vec<usize>) + 'static) -> Self {
            self.props.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            let labels = self.labels;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdChipGroup::new();
                    for l in labels {
                        b = b.chip(l);
                    }
                    b = b.multiple(props.multiple).mandatory(props.mandatory);
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoChipGroup::new();
                    for l in labels {
                        b = b.chip(l);
                    }
                    b = b.multiple(props.multiple).mandatory(props.mandatory);
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentChipGroup::new();
                    for l in labels {
                        b = b.chip(l);
                    }
                    b = b.multiple(props.multiple).mandatory(props.mandatory);
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for ChipGroup {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ChipGroup;
