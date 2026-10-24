//! A miniature IDE around the ported `floom-editor` widget.
//!
//! The demo exists to show the editor in the shape an application uses it:
//! several files, one editor, a tab per file and a panel that reads the
//! editor's state. Everything here is ordinary 2.0 composition — a `Stack` for
//! the three regions, a leaf widget per piece of chrome, and the editor as the
//! one child of its pane — so the demo adds no vocabulary to the framework.
//!
//! ## The one editor
//!
//! A tab is not a widget. The editor widget is mounted once, and switching
//! tabs swaps the document behind it (`EditorState::doc`) along with the
//! selection and the scroll. Each document is a [`floom_editor::TextDocument`],
//! so each tab keeps its own undo history; each [`state::OpenFile`] remembers
//! where its caret was.
//!
//! That is also why the editor's line cache compares the document's identity
//! as well as its revision: two freshly opened files both report revision `0`,
//! and a cache keyed on the number alone would draw the previous file.

pub mod chrome;
pub mod lang;
pub mod state;
pub mod theme;

use std::path::PathBuf;
use std::sync::Arc;

use floom::floom_app::WidgetHost;
use floom::floom_layout::LayoutStyle;
use floom::floom_types::ElementId;
use floom_core::{Axis, Child, ChildKey, LayoutRequest};
use floom_editor::{TextEditor, text_editor};
use floom::floom_widgets::Stack;

use crate::chrome::{EditorPane, Explorer, StatusBar, TabBar};
use crate::lang::{Lang, Syntax};
use crate::state::{App, SIDEBAR_WIDTH, STATUS_BAR_HEIGHT, Shared, TAB_BAR_HEIGHT};

/// The demo: the model, the shared handles, and the editor widget.
pub struct Ide {
    pub shared: Shared,
    pub editor: TextEditor,
}

impl Ide {
    /// Opens `root` and prepares an editor showing its first file.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let app = App::new(root);
        let lang = app
            .active
            .and_then(|active| app.open.get(active))
            .map(|file| file.lang)
            .unwrap_or(Lang::Text);
        let initial = app
            .active
            .and_then(|active| app.open.get(active))
            .map(|file| file.doc.clone());

        let syntax = Arc::new(Syntax::new(lang));
        let editor = text_editor(String::new())
            .style(theme::editor_style())
            .styling(syntax.clone());
        let editor_state = editor.state();
        if let Some(doc) = initial {
            let mut state = editor_state.lock().unwrap_or_else(|error| error.into_inner());
            state.doc = doc;
            state.generation += 1;
        }
        let shared = Shared::new(app, editor_state, syntax);
        Self { shared, editor }
    }
}

/// Mounts the demo's tree into `host` and gives the editor keyboard focus.
///
/// The focus is found by type rather than by position: the editor's element id
/// is minted when it mounts, and the explorer needs to hand focus back to it
/// after a file is clicked. Looking it up once, here, is what makes that
/// possible without teaching the widget tree about focus routing.
pub fn mount_ide(host: &mut WidgetHost, ide: &Ide) {
    let explorer = Explorer::new(ide.shared.clone());
    let root = Stack::new(Axis::Horizontal)
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().width(SIDEBAR_WIDTH as f32),
            move || Box::new(explorer.clone()),
        ))
        .child(Child::new(
            ChildKey::index(1),
            LayoutRequest::new().fill(),
            {
                let main = main_column(ide);
                move || Box::new(main.clone())
            },
        ));
    host.mount_root(LayoutStyle::default(), root);

    // Children are built by the reconciler, not by `mount_root`: the builder
    // closure is called when the node is first reconciled. Reconciling here —
    // before the host's first frame — is what makes the whole tree, the
    // editor included, exist for the lookup below.
    host.reconcile_all();
    if let Some(element) = editor_element(host) {
        ide.shared.set_editor_element(element);
        host.set_focus(element);
    }
}

/// The tab strip, the editor and the status bar.
fn main_column(ide: &Ide) -> Stack {
    let tabs = TabBar::new(ide.shared.clone());
    let status = StatusBar::new(ide.shared.clone());
    let pane = EditorPane::new(ide.editor.clone(), ide.shared.clone());
    Stack::new(Axis::Vertical)
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().height(TAB_BAR_HEIGHT as f32),
            move || Box::new(tabs.clone()),
        ))
        .child(Child::new(
            ChildKey::index(1),
            LayoutRequest::new().fill(),
            move || Box::new(pane.clone()),
        ))
        .child(Child::new(
            ChildKey::index(2),
            LayoutRequest::new().height(STATUS_BAR_HEIGHT as f32),
            move || Box::new(status.clone()),
        ))
}

/// The element the editor mounted under.
fn editor_element(host: &WidgetHost) -> Option<ElementId> {
    host.tree_nodes().into_iter().find_map(|node| {
        let id = host.node_for(node.element)?;
        host.widget::<TextEditor>(id).map(|_| node.element)
    })
}

#[cfg(test)]
mod tests {
    // Mounted, clicked and read out of the scene.
    //
    // Each test drives the demo the way a user does — a pointer at a
    // coordinate, a key — and then asserts on the text the host recorded, not
    // on the demo's own model. A widget that stores the right state and paints
    // nothing fails here.

    use super::*;
    use floom::floom_scene::ResourceKind;
    use floom::floom_types::{
        Event, Key, KeyEvent, Point, PointerEvent, Size,
    };


    /// The workspace shipped beside the demo.
    fn workspace() -> PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("workspace")
    }

    fn mount() -> (WidgetHost, Ide) {
        let ide = Ide::new(workspace());
        let mut host = WidgetHost::new();
        host.set_size(Size::new(1100.0, 700.0));
        mount_ide(&mut host, &ide);
        host.frame();
        (host, ide)
    }

    /// Every string the scene asked the renderer to draw.
    fn texts(host: &WidgetHost) -> Vec<String> {
        host.scene()
            .iter()
            .filter_map(|(_, node)| match node {
                floom::floom_scene::SceneNode::Text { glyphs, .. } => {
                    match &host.resources().get(*glyphs)?.kind {
                        ResourceKind::GlyphRun { text, .. } => Some(text.to_string()),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect()
    }

    fn has(host: &WidgetHost, needle: &str) -> bool {
        texts(host).iter().any(|text| text.contains(needle))
    }

    fn click(host: &mut WidgetHost, point: Point) {
        host.dispatch(Event::Pointer(PointerEvent::primary_down(point)));
        host.dispatch(Event::Pointer(PointerEvent::primary_up(point)));
    }

    fn click_row(host: &mut WidgetHost, ide: &Ide, name: &str) {
        let index = ide
            .shared
            .app()
            .rows()
            .iter()
            .position(|row| row.name == name && !row.dir)
            .unwrap_or_else(|| panic!("{name} is not in the explorer"));
        click(host, state::row_center(index));
        host.frame();
    }

    #[test]
    fn the_workspace_opens_in_a_tab_beside_the_explorer() {
        let (host, _ide) = mount();
        assert!(has(&host, "EXPLORER"), "the explorer's header");
        assert!(has(&host, "src"), "the src directory row");
        assert!(has(&host, "main.rs"), "a file row");
        assert!(has(&host, "README.md"), "a file row");
        assert!(
            has(&host, "A tiny expression language"),
            "the editor drew the open file"
        );
        assert!(has(&host, "src/main.rs"), "the status bar's path");
        assert!(has(&host, "Rust"), "the language");
        assert!(has(&host, "Ln 1, Col 1"), "the caret");
    }

    #[test]
    fn clicking_a_file_opens_it_and_the_old_document_leaves_the_scene() {
        let (mut host, ide) = mount();
        click_row(&mut host, &ide, "README.md");

        assert!(
            has(&host, "floom-editor IDE demo"),
            "the README's heading is painted"
        );
        assert!(has(&host, "Markdown"), "the status bar follows the file");
        assert!(
            !has(&host, "A tiny expression language"),
            "the previous document's cached lines are gone"
        );
    }

    #[test]
    fn clicking_a_tab_switches_back_and_typing_marks_the_file_dirty() {
        let (mut host, ide) = mount();
        click_row(&mut host, &ide, "README.md");
        assert!(has(&host, "floom-editor IDE demo"));

        // The first tab is the Rust file; a click anywhere in its first hundred
        // pixels lands on it.
        click(
            &mut host,
            Point::new(
                crate::state::SIDEBAR_WIDTH + 30.0,
                crate::state::TAB_BAR_HEIGHT / 2.0,
            ),
        );
        host.frame();
        assert!(
            has(&host, "A tiny expression language"),
            "the Rust document is back"
        );
        assert!(
            !has(&host, "floom-editor IDE demo"),
            "and the Markdown document is not"
        );

        // The tab click handed focus back to the text, so a typed key lands at
        // the caret rather than nowhere.
        host.dispatch(Event::Key(KeyEvent::down(Key::Char('x'))));
        host.frame();
        assert!(
            has(&host, "x//! A tiny expression language"),
            "the keystroke reached the editor: {:?}",
            texts(&host)
                .into_iter()
                .filter(|text| text.contains("expression"))
                .collect::<Vec<_>>()
        );
        assert!(has(&host, "● unsaved"), "the status bar noticed the edit");
    }

}
