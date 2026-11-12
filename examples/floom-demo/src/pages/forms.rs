//! The components a reader types into or picks from.

use crate::pages::{Category, ComponentPage, page};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("autocomplete", "Autocomplete"),
    ("checkbox", "Checkbox"),
    ("color_picker", "ColorPicker"),
    ("combobox", "Combobox"),
    ("date_picker", "DatePicker"),
    ("file_input", "FileInput"),
    ("form", "Form"),
    ("number_input", "NumberInput"),
    ("otp_input", "OTPInput"),
    ("range_slider", "RangeSlider"),
    ("select", "Select"),
    ("slider", "Slider"),
    ("text-field", "TextField"),
    ("textarea", "Textarea"),
    ("time_picker", "TimePicker"),
    ("validation_messages", "ValidationMessages"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| page(component, name, Category::Forms))
        .collect()
}
