//! Acceptance tests for the showcase shell: navigation, editor focus, devtools
//! interaction.
//!
//! These are GUI-adjacent tests run headless through `floom_testkit` — they
//! assert behaviour, not pixels.

use std::sync::Arc;

use floom::floom_app::WidgetHost;
use floom::floom_layout::LayoutStyle;
use floom::floom_types::{Event, Key, KeyEvent, PointerEvent};
use floom_testkit::{Selector, TestApp};

use crate::WINDOW;
use crate::editor::SharedEditor;
use crate::routes::{AppRoute, NavIntent};
use crate::shell::ShowcaseShell;
use crate::state::AppState;

/// The editor page focuses its editor on mount, so typing works without
/// the user having to find the box first.
#[test]
fn the_editor_page_takes_the_keyboard_on_mount() {
    let state = Arc::new(AppState::new());
    let mut app = TestApp::unpinned(WINDOW, ShowcaseShell::new(state.clone()));

    // Navigate the way a user does: press the sidebar item.
    let nav = app.node(Selector::label("Editor"));
    app.tap(nav);

    let before = state.editor().lock().unwrap().text();
    app.commit("x");
    let after = state.editor().lock().unwrap().text();
    assert_ne!(
        before, after,
        "typing after navigating to /editor did not reach the document"
    );
}

/// Leaving the page and coming back takes the keyboard again: the editor
/// widget outlives the page, but its focus does not.
#[test]
fn returning_to_the_editor_page_takes_the_keyboard_again() {
    let state = Arc::new(AppState::new());
    let mut app = TestApp::unpinned(WINDOW, ShowcaseShell::new(state.clone()));

    app.tap(app.node(Selector::label("Editor")));
    app.tap(app.node(Selector::label("Home")));
    app.tap(app.node(Selector::label("Editor")));

    let before = state.editor().lock().unwrap().text();
    app.commit("y");
    assert_ne!(before, state.editor().lock().unwrap().text());
}

/// The devtools panel is mounted as a drawer at startup. While hidden it
/// must not eat the clicks in its column — the editor spans most of the
/// window, so a press on its right half has to reach the editor.
#[test]
fn the_hidden_devtools_drawer_does_not_eat_editor_clicks() {
    let state = Arc::new(AppState::new());
    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    host.mount_root(
        LayoutStyle::default(),
        ShowcaseShell::new_at(state.clone(), AppRoute::Editor),
    );
    let mut devtools = floom::devtools::attach_with_rows(&mut host, |_| Vec::new());
    devtools.sync(&mut host);
    host.frame();

    let bounds = host
        .tree_nodes()
        .into_iter()
        .find_map(|n| {
            host.node_for(n.element)
                .filter(|&nid| host.widget::<SharedEditor>(nid).is_some())
                .map(|nid| host.bounds(nid))
        })
        .expect("the editor page mounts the editor");

    // The right-hand third is where the hidden drawer sits.
    let point = floom::floom_types::Point::new(bounds.x1 - 20.0, bounds.y0 + bounds.height() / 2.0);
    host.dispatch(PointerEvent::primary_down(point).into());
    host.dispatch(PointerEvent::primary_up(point).into());
    host.frame();
    assert!(
        host.focused().is_some(),
        "a press on the editor focused nothing"
    );

    let before = state.editor().lock().unwrap().text();
    host.dispatch(Event::Key(KeyEvent::down(Key::Char('x'))));
    host.frame();
    assert_ne!(state.editor().lock().unwrap().text(), before);
}

/// The shell routes push/back through a real host.
#[test]
fn shell_navigates_push_and_back() {
    let state = Arc::new(AppState::new());
    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    let root = host.mount_root(LayoutStyle::default(), ShowcaseShell::new(state.clone()));
    host.frame();

    for route in [
        AppRoute::Components,
        AppRoute::Kits,
        AppRoute::Editor,
        AppRoute::Devtools,
    ] {
        if let Some(shell) = host.widget_mut::<ShowcaseShell>(root) {
            *shell.pending.lock().unwrap() = Some(NavIntent::Go(route));
        }
        host.frame();
        let rendered = host
            .widget::<ShowcaseShell>(root)
            .map(|shell| shell.rendered)
            .expect("the shell is the root");
        assert_eq!(rendered, route, "router did not reach {}", route.path());
    }
    if let Some(shell) = host.widget_mut::<ShowcaseShell>(root) {
        *shell.pending.lock().unwrap() = Some(NavIntent::Back);
    }
    host.frame();
    let rendered = host
        .widget::<ShowcaseShell>(root)
        .map(|shell| shell.rendered)
        .expect("the shell is the root");
    assert_eq!(rendered, AppRoute::Editor, "Back did not pop to /editor");
}
