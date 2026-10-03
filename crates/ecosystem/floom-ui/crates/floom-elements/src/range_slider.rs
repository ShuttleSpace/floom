pub use floom_core_theme::component_tokens::{ComponentId, RangeSliderTokens};
pub use floom_core_theme::props::range_slider::RangeSliderProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_range_slider::CupertinoRangeSlider as RangeSlider;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_range_slider::FluentRangeSlider as RangeSlider;
#[cfg(feature = "material")]
pub use floom_widget_md_range_slider::MdRangeSlider as RangeSlider;
