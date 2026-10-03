use std::cell::Cell;
use std::rc::Rc;

use floom_elements::prelude::*;

#[test]
fn material_components_expose_controlled_interactions() {
    let clicks = Rc::new(Cell::new(0));
    let click_counter = Rc::clone(&clicks);
    Button::new("Save")
        .on_click(move || click_counter.set(click_counter.get() + 1))
        .click();
    assert_eq!(clicks.get(), 1);

    let disabled_clicks = Rc::new(Cell::new(0));
    let disabled_counter = Rc::clone(&disabled_clicks);
    Button::new("Disabled")
        .disabled(true)
        .on_click(move || disabled_counter.set(disabled_counter.get() + 1))
        .click();
    assert_eq!(disabled_clicks.get(), 0);

    let checkbox_changes = Rc::new(Cell::new(false));
    let checkbox_state = Rc::clone(&checkbox_changes);
    let mut checkbox = Checkbox::new("Accept").on_change(move |value| checkbox_state.set(value));
    checkbox.toggle();
    assert!(checkbox.is_checked());
    assert!(checkbox_changes.get());

    let mut disabled_checkbox = Checkbox::new("Locked").disabled(true);
    disabled_checkbox.toggle();
    assert!(!disabled_checkbox.is_checked());

    let switch_changes = Rc::new(Cell::new(false));
    let switch_state = Rc::clone(&switch_changes);
    let mut switch = Switch::new().on_change(move |value| switch_state.set(value));
    switch.toggle();
    assert!(switch.is_checked());
    assert!(switch_changes.get());

    let field_value_len = Rc::new(Cell::new(0));
    let field_len = Rc::clone(&field_value_len);
    let mut text_field = TextField::new()
        .max_length(3)
        .on_change(move |value| field_len.set(value.len()));
    text_field.set_value("abcd");
    assert_eq!(text_field.value(), "abc");
    assert_eq!(field_value_len.get(), 3);

    let closes = Rc::new(Cell::new(0));
    let close_counter = Rc::clone(&closes);
    let mut dialog = Dialog::new()
        .open(true)
        .on_close(move || close_counter.set(close_counter.get() + 1));
    dialog.close();
    assert!(!dialog.is_open());
    assert_eq!(closes.get(), 1);

    let selected = Rc::new(Cell::new(0));
    let selected_item = Rc::clone(&selected);
    let menu = Menu::new().item("Open", move || selected_item.set(1));
    menu.select(0);
    menu.select(1);
    assert_eq!(selected.get(), 1);
}
