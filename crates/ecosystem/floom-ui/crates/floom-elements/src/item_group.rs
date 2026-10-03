pub use floom_core_theme::component_tokens::ItemGroupTokens;
pub use floom_core_theme::props::item_group::ItemGroupProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_item_group::CupertinoItemGroup as ItemGroup;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_item_group::CupertinoItemGroupStyle as ItemGroupStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_item_group::FluentItemGroup as ItemGroup;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_item_group::FluentItemGroupStyle as ItemGroupStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_item_group::MdItemGroup as ItemGroup;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_item_group::MdItemGroupStyle as ItemGroupStyle;

/// Runtime-dispatch `ItemGroup` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::RwSignal;
    use floom::reactive::SignalGet;
    use floom::view::{AnyView, View};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::item_group::ItemGroupProps;

    use floom_widget_md_item_group::MdItemGroup;
    use floom_widget_cupertino_item_group::CupertinoItemGroup;
    use floom_widget_fluent_item_group::FluentItemGroup;

    pub struct ItemGroup {
        items: Vec<AnyView>,
        selected: Option<usize>,
        multiple: bool,
        mandatory: bool,
        on_change: Option<Box<dyn Fn(usize)>>,
        on_change_multi: Option<Box<dyn Fn(Vec<usize>)>>,
        single: RwSignal<usize>,
        multi: RwSignal<Vec<usize>>,
    }

    impl ItemGroup {
        pub fn new() -> Self {
            let props = ItemGroupProps::default();
            Self {
                items: Vec::new(),
                selected: None,
                multiple: props.multiple,
                mandatory: props.mandatory,
                on_change: None,
                on_change_multi: None,
                single: RwSignal::new(0),
                multi: RwSignal::new(Vec::new()),
            }
        }

        pub fn items(mut self, items: Vec<AnyView>) -> Self {
            self.items = items;
            self
        }

        pub fn selected(mut self, selected: usize) -> Self {
            self.selected = Some(selected);
            self.single = RwSignal::new(selected);
            self.multi = RwSignal::new(vec![selected]);
            self
        }

        pub fn multiple(mut self, multiple: bool) -> Self {
            self.multiple = multiple;
            self
        }

        pub fn mandatory(mut self, mandatory: bool) -> Self {
            self.mandatory = mandatory;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn on_change_multi(mut self, callback: impl Fn(Vec<usize>) + 'static) -> Self {
            self.on_change_multi = Some(Box::new(callback));
            self
        }

        pub fn selected_signal(&self) -> RwSignal<usize> {
            self.single
        }

        pub fn selected_multi_signal(&self) -> RwSignal<Vec<usize>> {
            self.multi
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self
                .on_change
                .unwrap_or_else(|| Box::new(|_| {}));
            let on_change_multi = self
                .on_change_multi
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => {
                    let mut b = MdItemGroup::new()
                        .items(self.items)
                        .multiple(self.multiple)
                        .mandatory(self.mandatory)
                        .on_change(on_change)
                        .on_change_multi(on_change_multi);
                    if let Some(s) = self.selected {
                        b = b.selected(s);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoItemGroup::new()
                        .items(self.items)
                        .multiple(self.multiple)
                        .mandatory(self.mandatory)
                        .on_change(on_change)
                        .on_change_multi(on_change_multi);
                    if let Some(s) = self.selected {
                        b = b.selected(s);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentItemGroup::new()
                        .items(self.items)
                        .multiple(self.multiple)
                        .mandatory(self.mandatory)
                        .on_change(on_change)
                        .on_change_multi(on_change_multi);
                    if let Some(s) = self.selected {
                        b = b.selected(s);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for ItemGroup {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ItemGroup;
#[cfg(feature = "multi")]
pub use floom_widget_md_item_group::MdItemGroupStyle as ItemGroupStyle;
