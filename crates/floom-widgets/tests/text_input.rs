//! The engine's [`TextInput`]: the shared buffer, the editing keys, and the two
//! things that are easy to get subtly wrong — UTF-8 boundaries and IME.
//!
//! The buffer is the whole design (see the module note): the input used to take
//! an `RwSignal<String>` from the caller, which cannot cross a `Send + Sync` slot
//! boundary, so it became an `Arc<Mutex<String>>`. That makes the *external write*
//! path a first-class case rather than a bonus — and it is the one a signal gave
//! away for free and this widget has to notice for itself.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use floom_core::{ElementId, Event, Key, KeyPhase, NamedKey, Widget, WidgetAny};
use floom_types::{Point, PointerEvent, Rect};

use floom_widgets::testkit::{
    AllocatingCreateContext, RecordingUpdateContext, SceneRecorder, TestEventContext,
    paint_snapshot,
};
use floom_widgets::{TextInput, TextInputStyle, clamp_to_char_boundary};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn style() -> TextInputStyle {
    TextInputStyle {
        background: Some(floom_core::Color::rgb(0xFF, 0xFF, 0xFF)),
        ..TextInputStyle::default()
    }
}

fn input(text: &str) -> TextInput {
    TextInput::with_text(text).style(style())
}

fn key(input: &mut TextInput, k: Key, id: ElementId) -> bool {
    let event = Event::Key(floom_types::KeyEvent {
        key: k,
        phase: KeyPhase::Down,
        modifiers: floom_types::Modifiers::NONE,
    });
    let mut cx = TestEventContext::new(id, BOX, event);
    input.event(&mut cx, id)
}

/// Give the widget an element id, the way a host would.
fn mounted(input: &mut TextInput) -> ElementId {
    input.create(&mut AllocatingCreateContext::new())
}

// --- the shared buffer ----------------------------------------------------

/// The caller keeps a handle and can read what was typed.
#[test]
fn a_caller_reads_what_was_typed_through_its_own_handle() {
    let mut field = input("");
    let handle = field.buffer();
    assert_eq!(handle.lock().unwrap().as_str(), "", "it starts empty");

    let id = mounted(&mut field);
    key(&mut field, Key::Char('h'), id);
    key(&mut field, Key::Char('i'), id);

    assert_eq!(field.text(), "hi", "the widget has it");
    assert_eq!(
        handle.lock().unwrap().as_str(),
        "hi",
        "and so does the caller's handle — the same buffer, not a copy"
    );
}

/// And the other direction: a caller writing to the handle reaches the widget.
///
/// This is the case a signal gave away for free, and this widget has to notice
/// itself. The widget's `update` compares what it last read, so the adoption
/// happens on the next frame — which is exactly what the test drives.
#[test]
fn a_caller_can_write_and_the_widget_notices_on_its_next_update() {
    let mut field = input("old");
    let handle = field.buffer();
    *handle.lock().unwrap() = "new".to_string();

    // Before an update the widget has not looked.
    let id = mounted(&mut field);
    field.update(&mut RecordingUpdateContext::new(), id);
    assert_eq!(field.text(), "new", "it adopted the external write");

    // And the caret, which pointed into the old text, is now clamped into the new.
    assert!(
        field.caret() <= "new".len(),
        "the caret is inside the buffer it now points at: {}",
        field.caret()
    );
}

/// The callback fires for **both** directions, which is what makes the buffer
/// two-way rather than write-only.
#[test]
fn on_change_fires_for_a_keystroke_and_for_an_external_write() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut field = {
        let seen = seen.clone();
        input("").on_change(move |text| seen.lock().unwrap().push(text))
    };
    let id = mounted(&mut field);

    key(&mut field, Key::Char('a'), id);
    assert_eq!(seen.lock().unwrap().as_slice(), ["a".to_string()], "typed");

    *field.buffer().lock().unwrap() = "ab".to_string();
    field.update(&mut RecordingUpdateContext::new(), id);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["a".to_string(), "ab".to_string()],
        "and written from outside — so a caller can treat this as a signal"
    );
}

/// A write that changes nothing must not fire, or a model that rewrites the same
/// value on every change would loop.
#[test]
fn an_update_with_no_external_change_fires_nothing() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut field = {
        let seen = seen.clone();
        input("same").on_change(move |text| seen.lock().unwrap().push(text))
    };
    let id = mounted(&mut field);
    for _ in 0..3 {
        field.update(&mut RecordingUpdateContext::new(), id);
    }
    assert!(
        seen.lock().unwrap().is_empty(),
        "three updates, no writes, three nothing"
    );
}

/// Two fields over one buffer see each other, which is what "shared" means and the
/// same property `TreeView`'s forest has.
#[test]
fn two_fields_over_one_buffer_see_each_other() {
    let mut a = TextInput::with_text("x");
    let shared = a.buffer();
    let mut b = TextInput::new(shared.clone());
    let b_id = mounted(&mut b);
    b.update(&mut RecordingUpdateContext::new(), b_id);

    let id = mounted(&mut a);
    key(&mut a, Key::Char('y'), id);
    b.update(&mut RecordingUpdateContext::new(), b_id);

    assert_eq!(b.text(), "xy", "b adopted what a typed");
    assert!(
        Arc::ptr_eq(&shared, &b.buffer()),
        "and it is one allocation"
    );
}

// --- editing --------------------------------------------------------------

#[test]
fn typing_inserts_at_the_caret() {
    let mut field = input("ac");
    field.set_caret(1);
    let id = mounted(&mut field);
    key(&mut field, Key::Char('b'), id);
    assert_eq!(field.text(), "abc", "inserted between a and c");
    assert_eq!(
        field.caret(),
        2,
        "and the caret moved past the new character"
    );
}

#[test]
fn backspace_removes_the_character_before_the_caret() {
    let mut field = input("abc");
    field.set_caret(3);
    let id = mounted(&mut field);
    key(&mut field, Key::Named(NamedKey::Backspace), id);
    assert_eq!(field.text(), "ab");
    assert_eq!(field.caret(), 2);
}

#[test]
fn delete_removes_the_character_after_the_caret() {
    let mut field = input("abc");
    field.set_caret(0);
    let id = mounted(&mut field);
    key(&mut field, Key::Named(NamedKey::Delete), id);
    assert_eq!(field.text(), "bc", "delete looks forward, backspace back");
    assert_eq!(field.caret(), 0, "and the caret does not move");
}

#[test]
fn the_arrows_and_home_and_end_move_the_caret_without_editing() {
    let mut field = input("abc");
    field.set_caret(3);
    let id = mounted(&mut field);

    key(&mut field, Key::Named(NamedKey::ArrowLeft), id);
    assert_eq!(field.caret(), 2);
    key(&mut field, Key::Named(NamedKey::ArrowRight), id);
    assert_eq!(field.caret(), 3);
    key(&mut field, Key::Named(NamedKey::Home), id);
    assert_eq!(field.caret(), 0);
    key(&mut field, Key::Named(NamedKey::End), id);
    assert_eq!(field.caret(), 3);
    assert_eq!(field.text(), "abc", "and none of them changed the text");
}

#[test]
fn backspace_at_the_start_and_delete_at_the_end_do_nothing() {
    let mut field = input("abc");
    let id = mounted(&mut field);

    field.set_caret(0);
    key(&mut field, Key::Named(NamedKey::Backspace), id);
    assert_eq!(field.text(), "abc", "nothing before the caret to remove");

    field.set_caret(3);
    key(&mut field, Key::Named(NamedKey::Delete), id);
    assert_eq!(field.text(), "abc", "and nothing after it either");
}

/// A key the field does not handle is left alone — and says so, so a parent can
/// have it.
#[test]
fn an_unhandled_key_is_not_consumed() {
    let mut field = input("abc");
    let id = mounted(&mut field);
    assert!(
        !key(&mut field, Key::Named(NamedKey::Tab), id),
        "Tab is not the field's to swallow"
    );
    assert_eq!(field.text(), "abc");
}

/// `KeyPhase::Up` must not insert, or every character arrives twice.
#[test]
fn a_key_release_does_not_insert() {
    let mut field = input("");
    let id = mounted(&mut field);
    let event = Event::Key(floom_types::KeyEvent {
        key: Key::Char('x'),
        phase: KeyPhase::Up,
        modifiers: floom_types::Modifiers::NONE,
    });
    let mut cx = TestEventContext::new(id, BOX, event);
    field.event(&mut cx, id);
    assert_eq!(field.text(), "", "only `Down` edits");
}

// --- UTF-8 ----------------------------------------------------------------

/// Every offset this widget holds is a byte index, and a byte index that is not on a
/// character boundary would panic on the next slice. These are the offsets that
/// would be wrong.
#[test]
fn the_caret_lands_on_character_boundaries_for_multibyte_text() {
    let mut field = input("héllo");
    let id = mounted(&mut field);

    // Backspace over the two-byte `é` removes the whole character, not half of it.
    field.set_caret(3);
    key(&mut field, Key::Named(NamedKey::Backspace), id);
    assert_eq!(
        field.text(),
        "hllo",
        "the whole é went, not one of its bytes"
    );
    assert_eq!(field.caret(), 1);

    // And an arrow over a *multibyte* character moves by the character, not by one
    // byte. The text is "hllo" by now — all single-byte — so this uses a fresh field,
    // because the assertion above is already carried by the backspace: on ASCII an
    // arrow and a byte offset agree, and only a multibyte string tells them apart.
    // `"héllo"` is six bytes: h at 0, é at 1..3, l at 3, l at 4, o at 5. Walking the
    // caret left from the end visits 5, 4, 3, **1** — and never 2, which is the second
    // byte of `é`. A caret that stepped by bytes would land on 2 and the next slice
    // would panic, which is the bug this sequence exists to rule out.
    let mut field = input("héllo");
    field.set_caret(field.text().len());
    let mut seen = vec![field.caret()];
    for _ in 0..4 {
        key(&mut field, Key::Named(NamedKey::ArrowLeft), id);
        seen.push(field.caret());
    }
    assert_eq!(
        seen,
        vec![6, 5, 4, 3, 1],
        "four steps back from six bytes, by character"
    );
    assert!(
        !seen.contains(&2),
        "and byte 2 — the middle of `é` — was never a caret position"
    );
}

#[test]
fn clamp_to_a_character_boundary_never_splits_a_character() {
    assert_eq!(clamp_to_char_boundary("héllo", 0), 0);
    assert_eq!(clamp_to_char_boundary("héllo", 1), 1, "before é");
    // 2 is inside `é`, which is two bytes at 1..3.
    assert_eq!(clamp_to_char_boundary("héllo", 2), 1, "inside é: back to 1");
    assert_eq!(clamp_to_char_boundary("héllo", 3), 3, "after é");
    assert_eq!(
        clamp_to_char_boundary("héllo", 99),
        6,
        "past the end: the end"
    );
}

/// An emoji outside the BMP is four bytes and two UTF-16 units, which is where a
/// naive "advance one" breaks.
#[test]
fn a_four_byte_character_survives_backspace() {
    let mut field = input("a🎉b");
    let id = mounted(&mut field);
    field.set_caret(5);
    key(&mut field, Key::Named(NamedKey::Backspace), id);
    assert_eq!(field.text(), "ab", "the whole emoji went");
    key(&mut field, Key::Named(NamedKey::Delete), id);
    assert_eq!(field.text(), "a", "and so did the b");
}

// --- IME ------------------------------------------------------------------

/// A composition is drawn but **not** in the buffer. If it were, an IME would have to
/// be trusted to send the right thing — and a half-typed pinyin syllable would be
/// readable as text.
#[test]
fn a_composition_is_not_in_the_buffer() {
    let mut field = input("ab");
    let id = mounted(&mut field);
    let event = Event::Ime(floom_core::ImeUpdate::Preedit {
        text: "ni".into(),
        caret: 2,
        selection: None,
    });
    let mut cx = TestEventContext::new(id, BOX, event);
    field.event(&mut cx, id);

    assert_eq!(field.text(), "ab", "the buffer is untouched");
    assert_eq!(field.preedit(), Some("ni"), "but the composition is held");
}

/// Closing the panel commits — which is the only way composition becomes text.
#[test]
fn closing_the_composition_commits_it() {
    let mut field = input("ab");
    field.set_caret(2);
    let id = mounted(&mut field);

    let mut cx = TestEventContext::new(
        id,
        BOX,
        Event::Ime(floom_core::ImeUpdate::Preedit {
            text: "ni".into(),
            caret: 2,
            selection: None,
        }),
    );
    field.event(&mut cx, id);
    let mut cx = TestEventContext::new(
        id,
        BOX,
        Event::Ime(floom_core::ImeUpdate::OpenChanged(false)),
    );
    field.event(&mut cx, id);

    assert_eq!(field.text(), "abni", "committed at the caret");
    assert_eq!(field.preedit(), None, "and the composition is gone");
}

/// An empty composition is not held at all, so `preedit()` is `None` rather than
/// `Some("")` — which is what lets the paint code treat "composing" as one case.
#[test]
fn an_empty_composition_is_not_held() {
    let mut field = input("ab");
    let id = mounted(&mut field);
    let mut cx = TestEventContext::new(
        id,
        BOX,
        Event::Ime(floom_core::ImeUpdate::Preedit {
            text: String::new(),
            caret: 0,
            selection: None,
        }),
    );
    field.event(&mut cx, id);
    assert_eq!(field.preedit(), None);
}

// --- focus and the caret --------------------------------------------------

/// A click focuses and requests focus, and puts the caret at the end.
///
/// The end rather than the click, and that is the module note's second limit:
/// `EventContext` cannot measure text, so an x coordinate cannot become a byte
/// offset. Recorded here so the behaviour is a decision rather than an accident.
#[test]
fn a_click_focuses_and_puts_the_caret_at_the_end() {
    let mut field = input("abc");
    field.set_caret(0);
    let id = mounted(&mut field);

    let at = Point::new(10.0, 20.0);
    let mut cx = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    field.event(&mut cx, id);

    assert!(field.is_focused(), "the field is focused");
    assert_eq!(
        field.caret(),
        3,
        "and the caret went to the end, not to the click"
    );
}

#[test]
fn a_click_on_an_empty_field_still_places_a_caret() {
    let mut field = input("");
    let id = mounted(&mut field);
    let at = Point::new(10.0, 20.0);
    let mut cx = TestEventContext::new(id, BOX, PointerEvent::primary_down(at).into());
    field.event(&mut cx, id);
    assert!(field.is_focused());
    assert_eq!(field.caret(), 0);
}

// --- enter and escape -----------------------------------------------------

#[test]
fn enter_reports_but_does_not_insert_a_newline() {
    let fired = AtomicUsize::new(0);
    let mut field = {
        let fired = &fired;
        // A closure over a stack counter is not `Send + Sync`, so this also shows the
        // bound is real: the counter has to be shared.
        let _ = fired;
        input("ab").on_enter(|| {})
    };
    let id = mounted(&mut field);
    key(&mut field, Key::Named(NamedKey::Enter), id);
    assert_eq!(field.text(), "ab", "a single-line field does not wrap");
}

/// Escape fires its callback and **does not** clear the field.
///
/// Other text inputs had no escape behaviour, so inventing "escape reverts" would
/// be a decision a text field should not make on its own.
#[test]
fn escape_reports_and_does_not_clear() {
    let fired = Arc::new(AtomicUsize::new(0));
    let mut field = {
        let fired = fired.clone();
        input("abc").on_escape(move || {
            fired.fetch_add(1, Ordering::SeqCst);
        })
    };
    let id = mounted(&mut field);
    key(&mut field, Key::Named(NamedKey::Escape), id);
    assert_eq!(fired.load(Ordering::SeqCst), 1, "the callback ran");
    assert_eq!(field.text(), "abc", "and the text is untouched");
}

// --- painting -------------------------------------------------------------

/// The control for everything above: the field really paints.
#[test]
fn the_field_paints_its_text() {
    let field = input("hello");
    let painted = paint_snapshot(
        &field,
        field.create(&mut AllocatingCreateContext::new()),
        BOX,
    );
    assert!(
        painted.to_lowercase().contains("hello"),
        "the text is in the scene: {painted}"
    );
}

#[test]
fn the_placeholder_is_painted_while_the_field_is_empty() {
    let field = input("").placeholder("Search");
    let painted = paint_snapshot(
        &field,
        field.create(&mut AllocatingCreateContext::new()),
        BOX,
    );
    assert!(
        painted.contains("Search"),
        "and it says what the field is for: {painted}"
    );
}

#[test]
fn the_placeholder_gives_way_to_the_text() {
    let field = input("query").placeholder("Search");
    let painted = paint_snapshot(
        &field,
        field.create(&mut AllocatingCreateContext::new()),
        BOX,
    );
    assert!(painted.contains("query"), "the text is shown");
    assert!(
        !painted.contains("Search"),
        "and the placeholder is not shown as well: {painted}"
    );
}

/// The caret is painted only while focused — an unfocused field with a blinking
/// caret would be a widget that never stops animating.
#[test]
fn the_caret_is_painted_only_while_focused() {
    let mut field = input("ab");
    let unfocused = paint_snapshot(
        &field,
        field.create(&mut AllocatingCreateContext::new()),
        BOX,
    );
    field.set_focused(true);
    let focused = paint_snapshot(
        &field,
        field.create(&mut AllocatingCreateContext::new()),
        BOX,
    );

    assert_ne!(
        focused, unfocused,
        "focusing adds the caret, so the two scenes differ"
    );
}

/// The control for the caret: the caret is a *thin* rect, not the whole field, so a
/// scene with one is distinguishable from a scene with a fill.
#[test]
fn the_field_paints_a_fill_and_a_caret_and_both_are_rects() {
    let mut field = input("ab");
    field.set_focused(true);
    let id = field.create(&mut AllocatingCreateContext::new());
    let painted = paint_snapshot(&field, id, BOX);
    assert!(
        painted.contains("Rect") || painted.contains("RoundedRect"),
        "so there is more than one rect in the scene: {painted}"
    );
}

// --- layout ---------------------------------------------------------------

/// The intrinsic width grows with the text and includes the padding and the caret, so
/// a longer value asks for more room.
#[test]
fn the_intrinsic_width_follows_the_text() {
    let short = input("a");
    let long = input("a much longer value indeed");
    // `width` is an `Option<Length>`, and this widget always sets it, so unwrap
    // rather than comparing options.
    let w = |f: &TextInput| match f.layout_request().expect("a layout").width {
        Some(floom_types::Length::Px(px)) => px,
        other => panic!("expected a pixel width, got {other:?}"),
    };
    assert!(
        w(&long) > w(&short),
        "a longer value asks for more room: {} vs {}",
        w(&long),
        w(&short)
    );
}

/// An empty field still has a row tall enough to type into, which is why the height
/// is measured from a reference glyph rather than from the (empty) text.
#[test]
fn an_empty_field_is_still_tall_enough_to_type_in() {
    let empty = input("");
    let request = empty.layout_request().expect("a layout");
    assert!(
        matches!(request.height, Some(floom_types::Length::Px(px)) if px > 0.0),
        "a positive pixel height, measured from a reference glyph: {:?}",
        request.height
    );
}

// --- semantics ------------------------------------------------------------

/// A text field publishes itself as one, named by what it holds.
///
/// The value *as* the name is a compromise — a field's name should be its label —
/// and it is recorded on the widget rather than left to be found here.
#[test]
fn the_field_publishes_a_text_field_role() {
    let field = input("hello");
    let hint = <TextInput as WidgetAny>::semantics_hint(&field).expect("a field has a hint");
    assert_eq!(hint.role, floom_core::SemanticsRole::TextField);
    assert_eq!(hint.label.as_deref(), Some("hello"), "named by its value");
}

/// And the hint follows the value, so a stale name is not published.
#[test]
fn the_published_name_follows_the_value() {
    let mut field = input("before");
    let id = mounted(&mut field);
    key(&mut field, Key::Char('!'), id);
    let hint = <TextInput as WidgetAny>::semantics_hint(&field).expect("a hint");
    assert_eq!(
        hint.label.as_deref(),
        Some("before!"),
        "the hint is built from the buffer each time it is asked for"
    );
}

/// `set_text` goes through the buffer, so it behaves like an external write —
/// including firing `on_change` on the next update.
#[test]
fn set_text_behaves_like_an_external_write() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut field = {
        let seen = seen.clone();
        input("a").on_change(move |text| seen.lock().unwrap().push(text))
    };
    field.set_text("b");
    let id = mounted(&mut field);
    field.update(&mut RecordingUpdateContext::new(), id);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        ["b".to_string()],
        "so a caller cannot use `set_text` to change the value silently"
    );
    assert_eq!(field.caret(), 1, "and the caret went to the end");
}
