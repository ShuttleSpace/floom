
use std::cell::Cell;
use std::rc::Rc;

use floom_platform::KeyCode;
use floom_testkit::TestApp;
use floom_types::{Point, Size};
use floom_ui_composites::Select;

#[test]
fn a_select_popup_escapes_its_header_and_takes_clicks() {
    let chosen = Rc::new(Cell::new(None::<usize>));
    let sink = chosen.clone();
    let mut app = TestApp::new(
        Size::new(200.0, 32.0),
        Select::new(["Red", "Green", "Blue"]).on_change(move |index| sink.set(Some(index))),
    );
    let select = app.root();
    assert!(!app.widget::<Select>(select).is_open());

    // Click the header: the list opens as an overlay and focus moves into it,
    // so the keyboard belongs to the list for as long as it is up.
    app.press(Point::new(100.0, 8.0));
    assert!(
        app.release(Point::new(100.0, 8.0)),
        "opening the list repainted"
    );
    assert!(
        app.widget::<Select>(select).is_open(),
        "the click opened the list"
    );
    assert_eq!(
        app.overlay_count(),
        1,
        "the list is a real overlay, not paint inside the header"
    );
    assert!(
        !app.is_focused(select),
        "focus went into the list, which now owns the keyboard"
    );

    // The third row sits at y≈112 — the control is pinned to the 32px window
    // (so bounds.y1 = 32) and each row is item_height = 32px, leaving row 2
    // spanning 96..128. Well below the control's own box, and below whatever
    // else the window might be showing. This click is the point of the whole
    // change: as raw paint ops the rows could never be hit, so a mouse could
    // open the list but never use it.
    app.press(Point::new(100.0, 112.0));
    assert!(app.release(Point::new(100.0, 112.0)), "choosing repainted");

    assert_eq!(
        app.widget::<Select>(select).selected_index(),
        Some(2),
        "the click landed on the row under the pointer"
    );
    assert_eq!(app.widget::<Select>(select).selected_label(), "Blue");
    assert_eq!(chosen.get(), Some(2), "the app's callback saw the choice");
    assert!(
        !app.widget::<Select>(select).is_open(),
        "choosing took the list down"
    );
    assert_eq!(app.overlay_count(), 0);
    assert!(app.is_focused(select), "and focus came back to the control");

    // Closed again, the arrow keys move the selection directly.
    assert!(app.key(KeyCode::ARROW_UP));
    assert_eq!(app.widget::<Select>(select).selected_index(), Some(1));
    assert_eq!(chosen.get(), Some(1));

    // Reopen and dismiss without choosing: the selection survives and focus
    // still returns to the control.
    app.press(Point::new(100.0, 8.0));
    app.release(Point::new(100.0, 8.0));
    assert!(app.widget::<Select>(select).is_open());
    assert!(app.key(KeyCode::ESCAPE));
    assert!(
        !app.widget::<Select>(select).is_open(),
        "escape dismissed it"
    );
    assert_eq!(app.widget::<Select>(select).selected_index(), Some(1));
    assert_eq!(app.overlay_count(), 0);
    assert!(app.is_focused(select));

    app.settle();
}
