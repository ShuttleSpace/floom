//! The editor, read out of the scene.
//!
//! These are paint-level tests: they mount the editor in a host, frame it, and
//! read what the scene says was drawn. A widget that compiles and does nothing
//! passes every test that looks at its own state, and a widget that draws the
//! wrong thing passes every test that looks at its own state too — the only
//! thing that tells the two apart is the scene.
//!
//! What is checked here is the *editor's* half of the picture: the text it
//! asked to be drawn, the caret it asked for, the selection it asked for, and
//! what a keystroke did to the document.

use std::sync::Arc;

use floom_app::WidgetHost;
use floom_core::{Key, KeyEvent, KeyPhase, NamedKey, PointerEvent};
use floom_editor::{Document, EditorStyle, Styling, TextDocument, TextEditor, text_editor};
use floom_layout::LayoutStyle;
use floom_scene::{ResourceKind, SceneNode};
use floom_types::{
    Brush, Color, Event, FontFamily, FontWeight, Modifiers, Point, Rect, Size, TextSpan, TextStyle,
};

/// A viewport the editor fills.
fn viewport() -> Size {
    Size::new(600.0, 300.0)
}

/// Mounts an editor over `text` and frames it.
fn mount(editor: TextEditor) -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(viewport());
    host.mount_root(LayoutStyle::default(), editor);
    host.frame();
    host
}

/// The text nodes in the scene, in paint order.
///
/// The gutter's line numbers are text nodes too — they are text — so a caller
/// that wants the *document's* nodes asks for the ones with a given text.
fn text_nodes(host: &WidgetHost) -> Vec<&SceneNode> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Text { .. } => Some(node),
            _ => None,
        })
        .collect()
}

/// The text nodes that were shaped from `text`.
fn nodes_with_text<'a>(host: &'a WidgetHost, text: &str) -> Vec<&'a SceneNode> {
    text_nodes(host)
        .into_iter()
        .filter(|node| text_of(host, node) == text)
        .collect()
}

/// The string a text node was shaped from.
fn text_of(host: &WidgetHost, node: &SceneNode) -> String {
    let SceneNode::Text { glyphs, .. } = node else {
        panic!("not a text node: {node:?}");
    };
    match &host
        .resources()
        .get(*glyphs)
        .expect("the glyph resource")
        .kind
    {
        ResourceKind::GlyphRun { text, .. } => text.to_string(),
        other => panic!("the text node's resource is not a glyph run: {other:?}"),
    }
}

/// The colour a text node was drawn in.
fn color_of(node: &SceneNode) -> Color {
    let SceneNode::Text { color, .. } = node else {
        panic!("not a text node: {node:?}");
    };
    *color
}

/// The editor's own text, as the document holds it.
fn document_text(editor: &TextEditor) -> String {
    editor.document().text().to_string()
}

/// The document's text is painted, one text node per line.
///
/// The editor lays each line out on its own and hands each one to the host, so
/// the scene carries one text node per line — and the node knows the string it
/// was shaped from, which is what makes this an assertion about the *right*
/// text rather than about some text.
#[test]
fn the_documents_text_is_painted_one_node_per_line() {
    let host = mount(text_editor("hello\nworld"));
    assert_eq!(
        nodes_with_text(&host, "hello").len(),
        1,
        "the first line is painted once"
    );
    assert_eq!(
        nodes_with_text(&host, "world").len(),
        1,
        "the second line is painted once"
    );
}

/// The caret is painted, in the editor's caret colour.
///
/// The caret is the editor's whole point: a text view without one is a
/// document, not an editor. The assertion is on the *colour* rather than on
/// the geometry, because the geometry is the engine's answer and the engine
/// has its own tests; what the editor owns is that it asked for a caret in
/// its own ink.
#[test]
fn the_caret_is_painted_in_the_cursors_colour() {
    let style = EditorStyle::default();
    let host = mount(text_editor("hello").style(style.clone()));
    let caret = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Rect { bounds, fill, .. } => Some((*bounds, fill.clone())),
            _ => None,
        })
        .find(|(_, fill)| *fill == Brush::Solid(style.cursor_color));
    assert!(
        caret.is_some(),
        "the editor painted no rect in its caret colour"
    );
}

/// A line with two inks is painted as two text nodes.
///
/// This is the paint path's whole reason for existing: the scene's text node
/// carries one colour, so a line with a highlighted range is two nodes over
/// one layout. A host that painted the line in one colour would render the
/// highlight as nothing, and the assertion is on the *colours*, which is the
/// only place the difference shows.
#[test]
fn a_styled_line_paints_one_text_node_per_ink() {
    let style = EditorStyle::default();
    let host = mount(text_editor("let x = 1;").styling(Arc::new(KeywordStyling)));
    let nodes = nodes_with_text(&host, "let x = 1;");
    assert_eq!(nodes.len(), 2, "two inks, two text nodes");
    let mut colors: Vec<Color> = nodes.iter().map(|n| color_of(n)).collect();
    colors.sort_by_key(|c| (c.r, c.g, c.b));
    assert_ne!(
        colors[0], colors[1],
        "the two inks painted in the same colour: {colors:?}"
    );
    assert!(
        colors.contains(&style.text_color),
        "the base ink is missing: {colors:?}"
    );
}

/// Typing inserts the character, and the scene shows the new text.
///
/// The key event goes through the host's dispatch, so this is the whole path:
/// the host routes it to the focused editor, the editor edits the document,
/// and the next frame paints the new text.
#[test]
fn typing_inserts_text_and_the_scene_shows_it() {
    let editor = text_editor("ab");
    let state = editor.state();
    // The caret starts at the beginning of the document, so the test puts it
    // at the end first: the point of the test is that a keystroke reaches
    // the document, not where the caret happens to start.
    state.lock().unwrap().set_caret(2);
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    assert!(
        host.dispatch(Event::Key(KeyEvent {
            phase: KeyPhase::Down,
            key: Key::Char('c'),
            modifiers: Modifiers::NONE,
        })),
        "the editor did not take the key"
    );
    host.frame();

    assert_eq!(document_text(&editor), "abc");
    let nodes = nodes_with_text(&host, "abc");
    assert_eq!(
        nodes.len(),
        1,
        "the line is painted once, with the new text"
    );
}

/// A click places the caret where the pointer is.
///
/// The editor hit-tests the point against the line it laid out, so the caret
/// lands on the byte under the pointer rather than at the end of the text.
#[test]
fn a_click_places_the_caret_where_the_pointer_is() {
    let editor = text_editor("hello world");
    let state = editor.state();
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    // The gutter is 56 wide, so the text starts there. A click well past the
    // start of the line lands inside the text.
    let x = 56.0 + 30.0;
    let y = 6.0;
    host.dispatch(Event::Pointer(PointerEvent::primary_down(Point::new(x, y))));
    host.frame();

    let cursor = state.lock().unwrap().cursor();
    assert!(
        cursor > 0 && cursor < 11,
        "the caret landed at {cursor}, not inside the word"
    );
}

/// A shifted letter is typed as the upper-case letter.
///
/// The key is normalized to lower case and the case lives in the shift
/// modifier, so an editor that ignores the modifier types "abc" where the user
/// asked for "AbC".
#[test]
fn a_shifted_letter_is_typed_in_upper_case() {
    let editor = text_editor("");
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    host.dispatch(Event::Key(KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('a'),
        modifiers: Modifiers::SHIFT,
    }));
    host.frame();

    assert_eq!(document_text(&editor), "A");
}

/// The editor scrolls to keep the caret on screen.
///
/// The caret is what the user is looking at, so a keystroke that moves it off
/// the screen has to bring it back: an editor that scrolls the text away from
/// the caret has lost it.
#[test]
fn the_editor_scrolls_to_keep_the_caret_on_screen() {
    let editor = text_editor("");
    let state = editor.state();
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    // The viewport is 300 tall and the line height is 24, so the caret has to
    // go thirteen lines down before it leaves the screen.
    for _ in 0..20 {
        host.dispatch(Event::Key(KeyEvent {
            phase: KeyPhase::Down,
            key: Key::Named(NamedKey::Enter),
            modifiers: Modifiers::NONE,
        }));
    }
    host.frame();

    assert!(
        state.lock().unwrap().scroll_line > 0 || state.lock().unwrap().scroll_offset > 0.0,
        "the editor scrolled the caret off the screen"
    );
}

/// A selection is painted, in the editor's selection colour.
///
/// The selection is drawn *behind* the text, so it is a rect rather than a
/// text node — and the rect's colour is the editor's own, which is what makes
/// it the editor's selection and not the line highlight.
#[test]
fn a_selection_is_painted_in_the_selections_colour() {
    let style = EditorStyle::default();
    let editor = text_editor("hello world");
    let state = editor.state();
    {
        let mut state = state.lock().unwrap();
        state.sel = floom_editor_core::selection::Selection::region(
            0,
            5,
            floom_editor_core::cursor::CursorAffinity::Forward,
        );
        state.generation += 1;
    }
    let host = mount(editor);
    let selection = host
        .scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Rect { bounds, fill, .. } => Some((*bounds, fill.clone())),
            _ => None,
        })
        .find(|(_, fill)| *fill == Brush::Solid(style.selection_color));
    assert!(
        selection.is_some(),
        "the editor painted no rect in its selection colour"
    );
}

/// The rectangles painted in `color`.
fn rects_of_colour(host: &WidgetHost, color: Color) -> Vec<Rect> {
    host.scene()
        .iter()
        .filter_map(|(_, node)| match node {
            SceneNode::Rect { bounds, fill, .. } if *fill == Brush::Solid(color) => Some(*bounds),
            _ => None,
        })
        .collect()
}

/// A styling that highlights the first word of every line.
struct KeywordStyling;

/// The editor with soft wrap on.
fn wrapped_editor(text: &str) -> TextEditor {
    text_editor(text).style(EditorStyle {
        soft_wrap: true,
        ..EditorStyle::default()
    })
}

/// A long line, long enough to wrap in the viewport.
fn long_line() -> String {
    "word ".repeat(80)
}

/// A wrapped line is still one line, and the gutter numbers it once.
///
/// The line is several rows on screen, but it is one line of the document,
/// and the gutter says so: a number per row would say the document had more
/// lines than it does.
#[test]
fn a_wrapped_line_is_numbered_once_in_the_gutter() {
    let host = mount(wrapped_editor(&long_line()));
    assert_eq!(
        nodes_with_text(&host, "1").len(),
        1,
        "the gutter numbered the wrapped line more than once"
    );
}

/// The caret on a wrapped line is on its own row.
///
/// The caret is measured against the layout, and the layout knows which row
/// the caret's byte is on. A caret that stayed on the first row of a wrapped
/// line would be in the wrong place, and the scene is where that shows.
#[test]
fn the_caret_on_a_wrapped_line_is_on_its_own_row() {
    let style = EditorStyle {
        soft_wrap: true,
        ..EditorStyle::default()
    };
    let text = long_line();
    let editor = text_editor(&text).style(style.clone());
    let state = editor.state();
    {
        let mut state = state.lock().unwrap();
        state.set_caret(0);
    }
    let mut host = mount(editor.clone());

    let first = rects_of_colour(&host, style.cursor_color);
    assert_eq!(first.len(), 1, "one caret");
    let first_y = first[0].y0;

    {
        let mut state = state.lock().unwrap();
        state.set_caret(text.len());
    }
    host.frame();

    let last = rects_of_colour(&host, style.cursor_color);
    assert_eq!(last.len(), 1, "one caret");
    assert!(
        last[0].y0 > first_y + 10.0,
        "the caret at the end of the wrapped line is on a later row: {first:?} then {last:?}"
    );
}

/// Two carets are painted as two carets.
///
/// A selection with two regions is the editor's multiple cursors, and each of
/// them is a caret on screen. An editor that drew one of them would be an
/// editor that had lost one.
#[test]
fn two_carets_are_painted_as_two_carets() {
    let style = EditorStyle::default();
    let editor = text_editor("hello world");
    let state = editor.state();
    {
        let mut state = state.lock().unwrap();
        state.sel = two_carets(0, 6);
        state.generation += 1;
    }
    let host = mount(editor);
    let carets = rects_of_colour(&host, style.cursor_color);
    assert_eq!(carets.len(), 2, "two carets, two rectangles");
}

/// Typing with two carets types at both of them.
///
/// The second caret is after the first one's text, so the first insertion
/// shifts it: an editor that typed at both carets but put the second one in
/// the wrong place would leave the text looking edited but wrong.
#[test]
fn typing_with_two_carets_types_at_both() {
    let editor = text_editor("ab\ncd");
    let state = editor.state();
    {
        let mut state = state.lock().unwrap();
        state.sel = two_carets(0, 3);
        state.generation += 1;
    }
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    host.dispatch(Event::Key(KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('x'),
        modifiers: Modifiers::NONE,
    }));
    host.frame();

    assert_eq!(
        document_text(&editor),
        "xab\nxcd",
        "the character was typed at both carets"
    );
}

/// Alt and a click adds a caret rather than moving the one that is there.
#[test]
fn alt_and_a_click_adds_a_caret() {
    let editor = text_editor("hello world");
    let state = editor.state();
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    host.dispatch(Event::Pointer(PointerEvent {
        phase: floom_core::PointerPhase::Down,
        kind: floom_core::PointerKind::Mouse,
        button: Some(floom_core::PointerButton::Primary),
        position: Point::new(56.0 + 30.0, 6.0),
        modifiers: Modifiers::ALT,
        click_count: 1,
    }));
    host.frame();

    assert_eq!(
        state.lock().unwrap().caret_count(),
        2,
        "the click added a caret instead of moving one"
    );
}

/// The whitespace is drawn when it is asked for.
///
/// The markers are the point of the feature: a space is invisible until
/// something is drawn in its place.
#[test]
fn the_whitespace_is_drawn_when_it_is_asked_for() {
    let style = EditorStyle {
        show_whitespace: true,
        ..EditorStyle::default()
    };
    let host = mount(text_editor("a b").style(style.clone()));
    assert_eq!(
        rects_of_colour(&host, style.whitespace_color).len(),
        1,
        "one space, one marker"
    );

    // And it is not drawn when it is not asked for.
    let plain = EditorStyle::default();
    let host = mount(text_editor("a b").style(plain.clone()));
    assert_eq!(
        rects_of_colour(&host, plain.whitespace_color).len(),
        0,
        "the whitespace was drawn without being asked for"
    );
}

/// The indent guides are drawn when they are asked for.
///
/// Eight spaces of indentation, four to a tab stop, is two guides.
#[test]
fn the_indent_guides_are_drawn_when_they_are_asked_for() {
    let style = EditorStyle {
        show_indent_guides: true,
        ..EditorStyle::default()
    };
    let host = mount(text_editor("        x").style(style.clone()));
    assert_eq!(
        rects_of_colour(&host, style.indent_guide_color).len(),
        2,
        "two tab stops of indentation, two guides"
    );
}

/// A click on the second line puts the caret on that line.
///
/// The layout knows only its own line, so the offset it answers is an offset
/// into that line — and the editor has to add where the line starts. An
/// editor that forgot would put the caret on the first line of the document
/// no matter which line was clicked.
#[test]
fn a_click_on_the_second_line_puts_the_caret_on_that_line() {
    let editor = text_editor("hello\nworld");
    let state = editor.state();
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    // The line height is 24, so y = 30 is on the second line.
    host.dispatch(Event::Pointer(PointerEvent::primary_down(Point::new(
        56.0 + 30.0,
        30.0,
    ))));
    host.frame();

    let cursor = state.lock().unwrap().cursor();
    assert!(
        cursor >= 6,
        "the caret landed at {cursor}, which is on the first line"
    );
}

/// The caret on the second line moves as the offset moves.
///
/// The offset is a document offset and the layout is a line's, so the editor
/// has to translate. An editor that did not would put every caret on a
/// later line at the end of that line, and the difference between two
/// offsets on the same line would be invisible.
#[test]
fn the_caret_on_the_second_line_moves_as_the_offset_moves() {
    let style = EditorStyle::default();
    let editor = text_editor("hello\nworld");
    let state = editor.state();
    {
        let mut state = state.lock().unwrap();
        state.set_caret(6);
    }
    let mut host = mount(editor.clone());

    let first = rects_of_colour(&host, style.cursor_color);
    assert_eq!(first.len(), 1, "one caret");

    {
        let mut state = state.lock().unwrap();
        state.set_caret(9);
    }
    host.frame();

    let second = rects_of_colour(&host, style.cursor_color);
    assert_eq!(second.len(), 1, "one caret");
    assert!(
        second[0].x0 > first[0].x0,
        "the caret moved right with the offset: {first:?} then {second:?}"
    );
}

/// A document that fits on the screen does not scroll away.
///
/// The viewport is 300 tall and a three-line document is 72, so there is
/// nowhere to scroll to. An editor that let the wheel push the text off the
/// screen would be an editor that could lose the text.
#[test]
fn a_document_that_fits_does_not_scroll_away() {
    let editor = text_editor("one\ntwo\nthree");
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    host.dispatch(Event::Scroll(floom_types::ScrollEvent {
        delta_x: 0.0,
        delta_y: 500.0,
        position: Point::new(100.0, 100.0),
        modifiers: Modifiers::NONE,
    }));
    host.frame();

    assert_eq!(
        nodes_with_text(&host, "one").len(),
        1,
        "the first line scrolled off the screen"
    );
    assert_eq!(
        nodes_with_text(&host, "three").len(),
        1,
        "the last line scrolled off the screen"
    );
}

/// Scrolling to the end keeps the last line on screen.
///
/// The scroll stops with the last line's bottom at the viewport's bottom, so
/// the last line is still there — an editor that scrolls its last line off
/// the bottom has nothing left to show.
#[test]
fn scrolling_to_the_end_keeps_the_last_line_on_screen() {
    let text = (0..40)
        .map(|index| format!("line {index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let editor = text_editor(text);
    let mut host = mount(editor.clone());
    let id = host.root().expect("the editor is the root");
    host.set_focus(host.node_id(id));

    host.dispatch(Event::Scroll(floom_types::ScrollEvent {
        delta_x: 0.0,
        delta_y: 5000.0,
        position: Point::new(100.0, 100.0),
        modifiers: Modifiers::NONE,
    }));
    host.frame();

    assert_eq!(
        nodes_with_text(&host, "line 39").len(),
        1,
        "the last line is not on screen"
    );
    assert_eq!(
        nodes_with_text(&host, "line 0").len(),
        0,
        "the first line is still on screen after scrolling to the end"
    );
    // The last line's *bottom* is at the viewport's bottom, not its top: the
    // scroll stops at the end of the document rather than at the end of the
    // last line's row.
    let SceneNode::Text { origin, .. } = nodes_with_text(&host, "line 39")[0] else {
        panic!("the last line is not a text node");
    };
    assert!(
        origin.y > 200.0,
        "the last line is at the top of the viewport, not the bottom: {origin:?}"
    );
}

/// A selection with two carets, as the editor's state holds it.
fn two_carets(first: usize, second: usize) -> floom_editor_core::selection::Selection {
    use floom_editor_core::cursor::CursorAffinity;
    use floom_editor_core::selection::{SelRegion, Selection};
    let mut selection = Selection::new();
    selection.add_region(SelRegion::caret(first, CursorAffinity::Forward));
    selection.add_region(SelRegion::caret(second, CursorAffinity::Forward));
    selection
}

impl Styling for KeywordStyling {
    fn id(&self) -> u64 {
        7
    }

    fn font_size(&self) -> f32 {
        16.0
    }

    fn line_height(&self) -> f32 {
        24.0
    }

    fn font_family(&self) -> FontFamily {
        FontFamily::Monospace
    }

    fn weight(&self) -> FontWeight {
        FontWeight::Normal
    }

    fn style_line(&self, _line: usize, text: &str, base: TextStyle) -> Vec<TextSpan> {
        let end = text.find(' ').unwrap_or(text.len());
        if end == 0 {
            return Vec::new();
        }
        vec![TextSpan {
            range: 0..end,
            style: TextStyle {
                color: Color::rgb(255, 0, 0),
                ..base
            },
        }]
    }
}

/// Swapping in another document paints *that* document.
///
/// The IDE demo switches tabs by replacing `EditorState::doc`, and every fresh
/// `TextDocument` reports revision `0` because the counter starts at zero in
/// each buffer. A line cache keyed on the revision alone therefore cannot tell
/// one new document from another: it hands back the previous file's lines, and
/// the scene shows the wrong text. The cache has to compare the document's
/// identity too, and this is the test that says so.
#[test]
fn swapping_the_document_paints_the_new_document() {
    let editor = text_editor("first document");
    let mut host = mount(editor.clone());
    assert_eq!(
        nodes_with_text(&host, "first document").len(),
        1,
        "the first document is on screen"
    );
    assert_eq!(nodes_with_text(&host, "second document").len(), 0);

    let second: Arc<dyn Document> = Arc::new(TextDocument::new("second document"));
    assert_eq!(
        second.revision(),
        0,
        "a fresh document starts at revision 0, exactly like the first one did"
    );
    {
        let state = editor.state();
        let mut state = state.lock().unwrap();
        state.doc = second;
        state.generation += 1;
    }
    host.frame();

    assert_eq!(
        nodes_with_text(&host, "second document").len(),
        1,
        "the new document's line is painted"
    );
    assert_eq!(
        nodes_with_text(&host, "first document").len(),
        0,
        "and the old document's line is not"
    );
}
