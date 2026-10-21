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
use floom_editor::{EditorStyle, Styling, TextEditor, text_editor};
use floom_layout::LayoutStyle;
use floom_scene::{ResourceKind, SceneNode};
use floom_types::{
    Brush, Color, Event, FontFamily, FontWeight, Modifiers, Point, Size, TextSpan, TextStyle,
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
    match &host.resources().get(*glyphs).expect("the glyph resource").kind {
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
    assert_eq!(nodes.len(), 1, "the line is painted once, with the new text");
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
    host.dispatch(Event::Pointer(PointerEvent::primary_down(Point::new(
        x, y,
    ))));
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
        state.lock().unwrap().scroll > 0.0,
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
        state.sel = floom_editor_core::selection::SelRegion::new(
            0,
            5,
            floom_editor_core::cursor::CursorAffinity::Forward,
            None,
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

/// A styling that highlights the first word of every line.
struct KeywordStyling;

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
