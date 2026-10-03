pub use floom_core_theme::component_tokens::ColorPickerTokens;
pub use floom_core_theme::props::color_picker::ColorPickerProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_color_picker::CupertinoColorPicker as ColorPicker;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_color_picker::CupertinoColorPickerStyle as ColorPickerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_color_picker::FluentColorPicker as ColorPicker;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_color_picker::FluentColorPickerStyle as ColorPickerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_color_picker::MdColorPicker as ColorPicker;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_color_picker::MdColorPickerStyle as ColorPickerStyle;

/// Runtime-dispatch `ColorPicker` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::RwSignal;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::color_picker::ColorPickerProps;
    use floom_core_theme::tokens::color::Color;

    use floom_widget_md_color_picker::MdColorPicker;
    use floom_widget_cupertino_color_picker::CupertinoColorPicker;
    use floom_widget_fluent_color_picker::FluentColorPicker;

    pub struct ColorPicker {
        color: RwSignal<Color>,
        show_alpha: bool,
        on_change: Option<Box<dyn Fn(Color)>>,
    }

    impl ColorPicker {
        pub fn new() -> Self {
            let props = ColorPickerProps::default();
            let c = Color::rgb(33, 150, 243);
            Self {
                color: RwSignal::new(c),
                show_alpha: props.show_alpha,
                on_change: None,
            }
        }

        pub fn value(mut self, value: Color) -> Self {
            self.color = RwSignal::new(value);
            self
        }

        pub fn show_alpha(mut self, show_alpha: bool) -> Self {
            self.show_alpha = show_alpha;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(Color) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn color_signal(&self) -> RwSignal<Color> {
            self.color
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self
                .on_change
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdColorPicker::new()
                    .value(self.color.get())
                    .show_alpha(self.show_alpha)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoColorPicker::new()
                    .value(self.color.get())
                    .show_alpha(self.show_alpha)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentColorPicker::new()
                    .value(self.color.get())
                    .show_alpha(self.show_alpha)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for ColorPicker {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ColorPicker;
#[cfg(feature = "multi")]
pub use floom_widget_md_color_picker::MdColorPickerStyle as ColorPickerStyle;
