//! The editor in a window.
//!
//! A real OS window, through the facade's one-call entry, with the editor
//! filling it and a button underneath that clears the document — so the
//! example shows both halves of the port: the widget, and a caller reaching
//! into the state from outside it.
//!
//! Run with `cargo run -p editor`.

use floom::prelude::*;
use floom_editor::{EditorStyle, text_editor};

fn main() {
    let text = std::env::args()
        .nth(1)
        .map(|s| std::fs::read_to_string(s).unwrap())
        .unwrap_or_else(|| {
            "The quick brown fox jumps over the lazy dog.\n\
             \n\
             Click to place the caret. Drag to select.\n\
             Shift-click extends the selection.\n\
             Double-click selects a word, triple-click a line.\n\
             \n\
             Type to edit. The arrow keys, Home, End, Backspace and Delete\n\
             all work; hold Control (or Command) for word motions.\n\
             \n\
             Alt and a click adds a caret, and Control-Alt with the up and\n\
             down arrows adds one above or below. Typing with several carets\n\
             types at all of them.\n\
             \n\
             The whitespace is drawn as a dot for a space and an arrow for a\n\
             tab, and the guides down the left are the indentation.\n"
                .to_string()
        });

    // Soft wrap, the whitespace markers and the indent guides are all off by
    // default: an editor that draws them without being asked is an editor
    // that draws them over somebody else's text.
    let editor = text_editor(text).style(EditorStyle {
        soft_wrap: true,
        show_whitespace: true,
        show_indent_guides: true,
        ..EditorStyle::dark()
    });
    let state = editor.state();

    let result = floom::run_desktop("floom — editor", Size::new(720.0, 480.0), {
        let editor = editor.clone();
        let state = state.clone();
        move |host: &mut WidgetHost| {
            let root = Stack::new(Axis::Vertical)
                .gap(8.0)
                .child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new().fill(),
                    {
                        let editor = editor.clone();
                        move || Box::new(editor.clone())
                    },
                ))
                .child(Child::new(
                    ChildKey::index(1),
                    LayoutRequest::new().height(32.0),
                    {
                        let state = state.clone();
                        move || {
                            // The click handler owns its own handle: the slot
                            // builder is an `Fn`, so it runs again on every
                            // rebuild and cannot move the handle away.
                            let clicked = state.clone();
                            Box::new(Button::new("clear").on_click(move || {
                                let mut state = clicked.lock().unwrap();
                                state.doc.set_text("".into());
                                state.set_caret(0);
                            }))
                        }
                    },
                ));
            host.mount_root(LayoutStyle::default(), root);
        }
    });

    if let Err(err) = result {
        eprintln!("desktop loop failed: {err}");
        std::process::exit(1);
    }
}
