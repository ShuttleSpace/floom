//! The components a reader types into or picks from.
//!
//! Every component here exists in all three design systems with the same
//! builder methods, so a sample is written once and shown three times.

use floom::floom_core::{Axis, WidgetAny};
use floom::floom_types::FontWeight;
use floom::floom_widgets::Stack;
use floom_core_theme::tokens::color::Color;
use floom_ui_composites::{DateValue, TimeValue};

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::{compare_children, themed_widget};
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::{compare_row, label, widget};

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
        .map(|(component, name)| match *component {
            "autocomplete" => autocomplete_page(),
            "checkbox" => checkbox_page(),
            "color_picker" => color_picker_page(),
            "combobox" => combobox_page(),
            "date_picker" => date_picker_page(),
            "file_input" => file_input_page(),
            "form" => form_page(),
            "number_input" => number_input_page(),
            "otp_input" => otp_input_page(),
            "range_slider" => range_slider_page(),
            "select" => select_page(),
            "slider" => slider_page(),
            "text-field" => text_field_page(),
            "textarea" => textarea_page(),
            "time_picker" => time_picker_page(),
            "validation_messages" => validation_messages_page(),
            _ => page(component, name, Category::Forms),
        })
        .collect()
}

/// A page in this category: an introduction, its samples and its API.
fn forms_page(
    component: &'static str,
    name: &'static str,
    intro: &'static str,
    samples: Vec<Sample>,
    api: ApiDoc,
) -> ComponentPage {
    let mut page = page(component, name, Category::Forms);
    page.intro = intro;
    page.samples = samples;
    page.api = api;
    page
}

/// The event row every field in this category shares: it reports what the
/// reader typed or chose.
fn change_event(payload: &str, description: &str) -> ApiRow {
    ApiRow::new(
        "on_change",
        "impl Fn(..) + Send + Sync",
        payload,
        description,
    )
}

// -- Autocomplete ---------------------------------------------------------

/// The Autocomplete page.
fn autocomplete_page() -> ComponentPage {
    forms_page(
        "autocomplete",
        "Autocomplete",
        "A field that suggests as the reader types.",
        vec![
            Sample {
                title: "With suggestions",
                note: None,
                build: autocomplete_options,
            },
            Sample {
                title: "One chosen",
                note: Some("A chosen suggestion stays in the field."),
                build: autocomplete_selected,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "Option<String>",
                "Called with the chosen suggestion, or `None` when the field is cleared.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Combobox",
                    "Combobox",
                    "Build the engine combobox this component renders.",
                ),
                ApiRow::new(
                    "query",
                    "&self -> floom_widgets::SharedBuffer",
                    "SharedBuffer",
                    "What the reader has typed so far.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The autocomplete, with suggestions to choose from.
fn autocomplete_options() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Autocomplete,
        ds::cupertino::Autocomplete,
        ds::fluent::Autocomplete,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Rust".to_string(),
                    "Floom".to_string(),
                    "Widgets".to_string(),
                ])
                .placeholder("Search")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The autocomplete, with a suggestion already chosen.
fn autocomplete_selected() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Autocomplete,
        ds::cupertino::Autocomplete,
        ds::fluent::Autocomplete,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Rust".to_string(),
                    "Floom".to_string(),
                    "Widgets".to_string(),
                ])
                .placeholder("Search")
                .selected(Some(1))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Checkbox -------------------------------------------------------------

/// The Checkbox page.
fn checkbox_page() -> ComponentPage {
    forms_page(
        "checkbox",
        "Checkbox",
        "A box the reader ticks, with its label beside it.",
        vec![
            Sample {
                title: "Unticked and ticked",
                note: None,
                build: checkbox_states,
            },
            Sample {
                title: "Disabled",
                note: Some("A disabled checkbox keeps its tick; it just stops responding."),
                build: checkbox_disabled,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "bool",
                "Called with the new ticked state when the reader changes it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Checkbox",
                "Checkbox",
                "Build the engine checkbox this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The checkbox, unticked and ticked.
fn checkbox_states() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Checkbox,
        ds::cupertino::Checkbox,
        ds::fluent::Checkbox,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "unticked",
                    themed_widget!(b, theme, "Send me a receipt", |c| c.view()),
                ))
                .child(widget(
                    "ticked",
                    themed_widget!(b, theme, "Send me a receipt", |c| c.checked(true).view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The checkbox, disabled.
fn checkbox_disabled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Checkbox,
        ds::cupertino::Checkbox,
        ds::fluent::Checkbox,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "unticked",
                    themed_widget!(b, theme, "Unavailable", |c| c.disabled(true).view()),
                ))
                .child(widget(
                    "ticked",
                    themed_widget!(b, theme, "Unavailable", |c| c
                        .checked(true)
                        .disabled(true)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ColorPicker ----------------------------------------------------------

/// The ColorPicker page.
fn color_picker_page() -> ComponentPage {
    forms_page(
        "color_picker",
        "ColorPicker",
        "A field the reader picks a colour in.",
        vec![
            Sample {
                title: "A colour",
                note: None,
                build: color_picker_value,
            },
            Sample {
                title: "With alpha",
                note: Some("Alpha is the fourth channel: how opaque the colour is."),
                build: color_picker_alpha,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "Color",
                "Called with the new colour when the reader picks one.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Surface",
                    "Surface",
                    "Build the engine surface this component renders.",
                ),
                ApiRow::new(
                    "hex",
                    "&self -> floom_ui_composites::SharedHex",
                    "SharedHex",
                    "The chosen colour as hex, shared with whoever is reading.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The colour picker, on one colour.
fn color_picker_value() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ColorPicker,
        ds::cupertino::ColorPicker,
        ds::fluent::ColorPicker,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(Color::rgb(0x2B, 0x6C, 0xB0))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The colour picker, with the alpha channel shown.
fn color_picker_alpha() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ColorPicker,
        ds::cupertino::ColorPicker,
        ds::fluent::ColorPicker,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(Color::rgba(0x2B, 0x6C, 0xB0, 0x80))
                .show_alpha(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Combobox -------------------------------------------------------------

/// The Combobox page.
fn combobox_page() -> ComponentPage {
    forms_page(
        "combobox",
        "Combobox",
        "A field the reader picks one of several options from.",
        vec![
            Sample {
                title: "With options",
                note: None,
                build: combobox_options,
            },
            Sample {
                title: "One chosen",
                note: Some("A chosen option stays in the field."),
                build: combobox_selected,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "Option<usize>",
                "Called with the chosen option's index, or `None` when the field is cleared.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Combobox",
                "Combobox",
                "Build the engine combobox this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The combobox, with options to choose from.
fn combobox_options() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Combobox,
        ds::cupertino::Combobox,
        ds::fluent::Combobox,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Material".to_string(),
                    "Cupertino".to_string(),
                    "Fluent".to_string(),
                ])
                .placeholder("Pick one")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The combobox, with an option already chosen.
fn combobox_selected() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Combobox,
        ds::cupertino::Combobox,
        ds::fluent::Combobox,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Material".to_string(),
                    "Cupertino".to_string(),
                    "Fluent".to_string(),
                ])
                .placeholder("Pick one")
                .selected(Some(0))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- DatePicker -----------------------------------------------------------

/// The DatePicker page.
fn date_picker_page() -> ComponentPage {
    forms_page(
        "date_picker",
        "DatePicker",
        "A field the reader picks a calendar date in.",
        vec![
            Sample {
                title: "Empty",
                note: None,
                build: date_picker_empty,
            },
            Sample {
                title: "With a date",
                note: Some("A picked date stays in the field."),
                build: date_picker_value,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "DateValue",
                "Called with the chosen date when the reader picks one.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::DatePicker",
                    "DatePicker",
                    "Build the engine date picker this component renders.",
                ),
                ApiRow::new(
                    "selected",
                    "&self -> Option<DateValue>",
                    "None",
                    "The date chosen right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The date picker, with nothing chosen.
fn date_picker_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DatePicker,
        ds::cupertino::DatePicker,
        ds::fluent::DatePicker,
        |b, theme| b::new().with_theme(theme).placeholder("Pick a date").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The date picker, with a date chosen.
fn date_picker_value() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DatePicker,
        ds::cupertino::DatePicker,
        ds::fluent::DatePicker,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(DateValue {
                    year: 2026,
                    month: 11,
                    day: 12,
                })
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- FileInput ------------------------------------------------------------

/// The FileInput page.
fn file_input_page() -> ComponentPage {
    forms_page(
        "file_input",
        "FileInput",
        "A field the reader chooses a file in.",
        vec![
            Sample {
                title: "One file",
                note: Some(
                    "It reports the chosen path through `path_buffer()`; it does not open an OS dialog.",
                ),
                build: file_input_one,
            },
            Sample {
                title: "Several files, and disabled",
                note: None,
                build: file_input_many,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "Option<String>",
                "Called with the chosen path, or `None` when the choice is cleared.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Pressable",
                    "Pressable",
                    "Build the engine pressable this component renders.",
                ),
                ApiRow::new(
                    "path",
                    "&self -> String",
                    "\"\"",
                    "The path chosen right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The file input, taking one file.
fn file_input_one() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::FileInput,
        ds::cupertino::FileInput,
        ds::fluent::FileInput,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .label("Choose a file")
                .accept("image/*")
                .child(label(
                    "chosen",
                    "No file chosen",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::MUTED,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The file input, taking several files, and disabled.
fn file_input_many() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::FileInput,
        ds::cupertino::FileInput,
        ds::fluent::FileInput,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .label("Choose images")
                .accept("image/*")
                .multiple(true)
                .disabled(true)
                .child(label(
                    "chosen",
                    "No files chosen",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::MUTED,
                ))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Form -----------------------------------------------------------------

/// The Form page.
fn form_page() -> ComponentPage {
    forms_page(
        "form",
        "Form",
        "A group of fields with a submit button, and the errors they collected.",
        vec![
            Sample {
                title: "With a submit button",
                note: None,
                build: form_with_submit,
            },
            Sample {
                title: "Without a submit button",
                note: Some("A form can be submitted from the outside instead."),
                build: form_without_submit,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_submit",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader submits the form.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Surface",
                    "Surface",
                    "Build the engine surface this component renders.",
                ),
                ApiRow::new(
                    "is_valid",
                    "&self -> bool",
                    "true",
                    "Whether every field in the form is valid.",
                ),
                ApiRow::new(
                    "errors",
                    "&self -> Vec<String>",
                    "[]",
                    "The errors the form has collected.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The form, with a submit button.
fn form_with_submit() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Form,
        ds::cupertino::Form,
        ds::fluent::Form,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .child(label(
                    "field",
                    "A field",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .show_submit(true)
                .submit_label("Save")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The form, with no submit button.
fn form_without_submit() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Form,
        ds::cupertino::Form,
        ds::fluent::Form,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .child(label(
                    "field",
                    "A field",
                    13.0,
                    FontWeight::Normal,
                    crate::widgets::INK,
                ))
                .show_submit(false)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- NumberInput ----------------------------------------------------------

/// The NumberInput page.
fn number_input_page() -> ComponentPage {
    forms_page(
        "number_input",
        "NumberInput",
        "A field the reader types a number into, within a range.",
        vec![
            Sample {
                title: "A number",
                note: None,
                build: number_input_default,
            },
            Sample {
                title: "A range that crosses zero",
                note: Some("The step is how far one press moves the number."),
                build: number_input_range,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "i64",
                "Called with the new value when the reader changes it.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Stack",
                    "Stack",
                    "Build the engine stack this component renders.",
                ),
                ApiRow::new(
                    "current_value",
                    "&self -> i64",
                    "0",
                    "The value in the field right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The number input, in its ordinary range.
fn number_input_default() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::NumberInput,
        ds::cupertino::NumberInput,
        ds::fluent::NumberInput,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .label("Quantity")
                .value(7)
                .min(0)
                .max(10)
                .step(1)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The number input, in a range that crosses zero.
fn number_input_range() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::NumberInput,
        ds::cupertino::NumberInput,
        ds::fluent::NumberInput,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .label("Offset")
                .value(-3)
                .min(-10)
                .max(10)
                .step(5)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- OTPInput -------------------------------------------------------------

/// The OTPInput page.
fn otp_input_page() -> ComponentPage {
    forms_page(
        "otp_input",
        "OTPInput",
        "A row of boxes the reader types a one-time code into.",
        vec![
            Sample {
                title: "Six digits",
                note: None,
                build: otp_input_plain,
            },
            Sample {
                title: "Masked",
                note: Some("A masked code hides what the reader typed."),
                build: otp_input_masked,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "String",
                "Called with the whole code as the reader types it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The one-time-code field, showing what is typed.
fn otp_input_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::OtpInput,
        ds::cupertino::OtpInput,
        ds::fluent::OtpInput,
        |b, theme| b::new().with_theme(theme).length(6).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The one-time-code field, hiding what is typed.
fn otp_input_masked() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::OtpInput,
        ds::cupertino::OtpInput,
        ds::fluent::OtpInput,
        |b, theme| b::new().with_theme(theme).length(4).masked(true).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- RangeSlider ----------------------------------------------------------

/// The RangeSlider page.
fn range_slider_page() -> ComponentPage {
    forms_page(
        "range_slider",
        "RangeSlider",
        "A slider with two thumbs: a range the reader picks.",
        vec![
            Sample {
                title: "A wide range",
                note: None,
                build: range_slider_wide,
            },
            Sample {
                title: "A narrow range, and disabled",
                note: Some("The two thumbs cannot cross."),
                build: range_slider_narrow,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "(f32, f32)",
                "Called with the low and high values when the reader moves a thumb.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::RangeSlider",
                    "RangeSlider",
                    "Build the engine range slider this component renders.",
                ),
                ApiRow::new(
                    "low_value",
                    "&self -> f32",
                    "0.0",
                    "The low thumb's value right now.",
                ),
                ApiRow::new(
                    "high_value",
                    "&self -> f32",
                    "0.0",
                    "The high thumb's value right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The range slider, over a wide range.
fn range_slider_wide() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::RangeSlider,
        ds::cupertino::RangeSlider,
        ds::fluent::RangeSlider,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .min(0.0)
                .max(100.0)
                .low(20.0)
                .high(80.0)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The range slider, over a narrow range and disabled.
fn range_slider_narrow() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::RangeSlider,
        ds::cupertino::RangeSlider,
        ds::fluent::RangeSlider,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .min(0.0)
                .max(10.0)
                .low(2.0)
                .high(8.0)
                .step(1.0)
                .disabled(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Select ---------------------------------------------------------------

/// The Select page.
fn select_page() -> ComponentPage {
    forms_page(
        "select",
        "Select",
        "A field the reader picks one option from.",
        vec![
            Sample {
                title: "With options",
                note: None,
                build: select_options,
            },
            Sample {
                title: "One chosen, and disabled",
                note: None,
                build: select_selected,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "usize",
                "Called with the chosen option's index when it changes.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Select",
                "Select",
                "Build the engine select this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The select, with options to choose from.
fn select_options() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Select,
        ds::cupertino::Select,
        ds::fluent::Select,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Material".to_string(),
                    "Cupertino".to_string(),
                    "Fluent".to_string(),
                ])
                .placeholder("Pick one")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The select, with an option chosen and the whole field disabled.
fn select_selected() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Select,
        ds::cupertino::Select,
        ds::fluent::Select,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .options(vec![
                    "Material".to_string(),
                    "Cupertino".to_string(),
                    "Fluent".to_string(),
                ])
                .selected(Some(1))
                .disabled(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Slider ---------------------------------------------------------------

/// The Slider page.
fn slider_page() -> ComponentPage {
    forms_page(
        "slider",
        "Slider",
        "A slider the reader drags along a track.",
        vec![
            Sample {
                title: "One value",
                note: None,
                build: slider_single,
            },
            Sample {
                title: "Stepped",
                note: Some("The step is how far the thumb moves at a time."),
                build: slider_stepped,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "f32",
                "Called with the new value when the reader moves the thumb.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::Slider",
                    "Slider",
                    "Build the engine slider this component renders.",
                ),
                ApiRow::new(
                    "props",
                    "&self -> &SliderProps",
                    "SliderProps",
                    "The slider's props, as the theme reads them.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The slider, showing one value.
fn slider_single() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Slider,
        ds::cupertino::Slider,
        ds::fluent::Slider,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .min(0.0)
                .max(100.0)
                .value(40.0)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The slider, with a step.
fn slider_stepped() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Slider,
        ds::cupertino::Slider,
        ds::fluent::Slider,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .min(0.0)
                .max(10.0)
                .value(4.0)
                .step(2.0)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- TextField ------------------------------------------------------------

/// The TextField page.
fn text_field_page() -> ComponentPage {
    forms_page(
        "text-field",
        "TextField",
        "A field the reader types into.",
        vec![
            Sample {
                title: "With a placeholder",
                note: None,
                build: text_field_placeholder,
            },
            Sample {
                title: "A password",
                note: Some("A password field hides what is typed as it is typed."),
                build: text_field_password,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "&str",
                "Called with the text as the reader types it.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::TextField",
                "TextField",
                "Build the engine text field this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The text field, showing its placeholder.
fn text_field_placeholder() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::TextField,
        ds::cupertino::TextField,
        ds::fluent::TextField,
        |b, theme| b::new().with_theme(theme).placeholder("Your name").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The text field, as a password field.
fn text_field_password() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::TextField,
        ds::cupertino::TextField,
        ds::fluent::TextField,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value("hunter2")
                .password(true)
                .max_length(32)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Textarea -------------------------------------------------------------

/// The Textarea page.
fn textarea_page() -> ComponentPage {
    forms_page(
        "textarea",
        "Textarea",
        "A field the reader types several lines into.",
        vec![
            Sample {
                title: "Empty",
                note: None,
                build: textarea_empty,
            },
            Sample {
                title: "With text",
                note: Some("The rows are how tall the field is."),
                build: textarea_value,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "String",
                "Called with the text as the reader types it.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Surface",
                    "Surface",
                    "Build the engine surface this component renders.",
                ),
                ApiRow::new(
                    "text",
                    "&self -> String",
                    "\"\"",
                    "The text in the field right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The textarea, showing its placeholder.
fn textarea_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Textarea,
        ds::cupertino::Textarea,
        ds::fluent::Textarea,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .placeholder("Tell us more")
                .rows(4)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The textarea, with text in it.
fn textarea_value() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Textarea,
        ds::cupertino::Textarea,
        ds::fluent::Textarea,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value("A gallery of every component,\nin all three design systems.")
                .rows(6)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- TimePicker -----------------------------------------------------------

/// The TimePicker page.
fn time_picker_page() -> ComponentPage {
    forms_page(
        "time_picker",
        "TimePicker",
        "A field the reader picks a time of day in.",
        vec![
            Sample {
                title: "Empty",
                note: None,
                build: time_picker_empty,
            },
            Sample {
                title: "With a time",
                note: Some("A picked time stays in the field."),
                build: time_picker_value,
            },
        ],
        ApiDoc {
            events: vec![change_event(
                "TimeValue",
                "Called with the chosen time when the reader picks one.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::TimePicker",
                    "TimePicker",
                    "Build the engine time picker this component renders.",
                ),
                ApiRow::new(
                    "selected",
                    "&self -> Option<TimeValue>",
                    "None",
                    "The time chosen right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The time picker, with nothing chosen.
fn time_picker_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::TimePicker,
        ds::cupertino::TimePicker,
        ds::fluent::TimePicker,
        |b, theme| b::new().with_theme(theme).placeholder("Pick a time").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The time picker, with a time chosen.
fn time_picker_value() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::TimePicker,
        ds::cupertino::TimePicker,
        ds::fluent::TimePicker,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(TimeValue {
                    hour: 9,
                    minute: 30,
                })
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ValidationMessages ---------------------------------------------------

/// The ValidationMessages page.
fn validation_messages_page() -> ComponentPage {
    forms_page(
        "validation_messages",
        "ValidationMessages",
        "The list of things a reader has to fix.",
        vec![
            Sample {
                title: "Two messages",
                note: None,
                build: validation_messages_two,
            },
            Sample {
                title: "One message",
                note: Some("The gap is how far apart the messages sit."),
                build: validation_messages_one,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::ValidationMessagesList",
                    "ValidationMessagesList",
                    "Build the engine list this component renders.",
                ),
                ApiRow::new(
                    "current_messages",
                    "&self -> Vec<String>",
                    "[]",
                    "The messages on show right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The validation messages, two of them.
fn validation_messages_two() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ValidationMessages,
        ds::cupertino::ValidationMessages,
        ds::fluent::ValidationMessages,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .messages(vec![
                    "Name is required".to_string(),
                    "Email is not valid".to_string(),
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The validation messages, one of them.
fn validation_messages_one() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ValidationMessages,
        ds::cupertino::ValidationMessages,
        ds::fluent::ValidationMessages,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .messages(vec!["Try again".to_string()])
                .font_size(15.0)
                .gap(10.0)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}
