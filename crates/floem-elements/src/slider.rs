pub use floem_core_theme::component_tokens::SliderTokens;
pub use floem_core_theme::props::slider::SliderProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_slider::CupertinoSlider as Slider;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_slider::CupertinoSliderStyle as SliderStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_slider::FluentSlider as Slider;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_slider::FluentSliderStyle as SliderStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_slider::MdSlider as Slider;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_slider::MdSliderStyle as SliderStyle;

/// Runtime-dispatch `Slider` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_slider::MdSlider;
    use floem_widget_cupertino_slider::CupertinoSlider;
    use floem_widget_fluent_slider::FluentSlider;

    pub struct Slider {
        value: f32,
        min: f32,
        max: f32,
        step: f32,
        disabled: bool,
        on_change: Option<Box<dyn Fn(f32)>>,
        theme: Option<MultiTheme>,
    }

    impl Slider {
        pub fn new() -> Self {
            Self {
                value: 0.0,
                min: 0.0,
                max: 100.0,
                step: 1.0,
                disabled: false,
                on_change: None,
                theme: None,
            }
        }

        pub fn value(mut self, value: f32) -> Self {
            self.value = value;
            self
        }

        pub fn min(mut self, min: f32) -> Self {
            self.min = min;
            self
        }

        pub fn max(mut self, max: f32) -> Self {
            self.max = max;
            self
        }

        pub fn step(mut self, step: f32) -> Self {
            self.step = step;
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => {
                    let mut c = MdSlider::new()
                        .value(self.value)
                        .min(self.min)
                        .max(self.max)
                        .step(self.step)
                        .disabled(self.disabled)
                        .on_change(on_change);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoSlider::new()
                        .value(self.value)
                        .min(self.min)
                        .max(self.max)
                        .step(self.step)
                        .disabled(self.disabled)
                        .on_change(on_change);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentSlider::new()
                        .value(self.value)
                        .min(self.min)
                        .max(self.max)
                        .step(self.step)
                        .disabled(self.disabled)
                        .on_change(on_change);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for Slider {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Slider;
#[cfg(feature = "multi")]
pub use floem_widget_md_slider::MdSliderStyle as SliderStyle;
