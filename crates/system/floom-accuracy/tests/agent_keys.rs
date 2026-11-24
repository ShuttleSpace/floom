// SPDX-License-Identifier: MIT
//! Author-declared keys: the address that survives reordering, list insertion
//! and translated labels, which a position cannot.

use floom_accuracy::agent::{
    AgentError, AgentRequest, AgentSession, Command, PROTOCOL_VERSION, Payload, Selector,
};
use floom_core::{Axis, Child, ChildKey, LayoutRequest};
use floom_testkit::TestApp;
use floom_widgets::{Button, Stack};

fn row(name: &'static str, label: &'static str) -> Child {
    Child::new(
        ChildKey::from(name),
        LayoutRequest::new().width(160.0).height(40.0),
        move || Box::new(Button::new(label)),
    )
    .named(name)
}

/// `first` decides whether the keyed child sits before or after a plain one, so
/// the test can show the key does not care about position.
fn app(first: bool) -> TestApp {
    let mut stack = Stack::new(Axis::Vertical);
    if !first {
        stack = stack.child(row("reset", "Reset"));
    }
    stack = stack.child(row("increment", "Increment"));
    if first {
        stack = stack.child(row("reset", "Reset"));
    }
    TestApp::new(floom_types::Size::new(320.0, 160.0), stack)
}

fn ask(app: &mut TestApp, command: Command) -> Result<Payload, AgentError> {
    let request = AgentRequest {
        version: PROTOCOL_VERSION,
        id: 1,
        token: None,
        command,
    };
    AgentSession::new(app).handle(request).result
}

fn tap(app: &mut TestApp, selector: Selector) -> Result<Payload, AgentError> {
    ask(app, Command::Tap { selector })
}

#[test]
fn a_named_child_reports_its_key() {
    let mut app = app(true);
    let Ok(Payload::Tree(tree)) = ask(&mut app, Command::Tree) else {
        panic!("Tree answers with a tree");
    };
    let increment = tree
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("Increment"))
        .expect("the keyed button is in the tree");
    assert_eq!(increment.key.as_deref(), Some("increment"));
}

#[test]
fn a_key_can_be_tapped() {
    let mut app = app(true);
    match tap(&mut app, Selector::Key("increment".to_string())) {
        Ok(Payload::Done { repainted }) => assert!(repainted, "tapping the keyed button repaints"),
        other => panic!("Tap by key answered with {other:?}"),
    }
}

#[test]
fn a_key_resolves_wherever_the_node_sits() {
    // Same key, different positions: an agent that remembered "increment" from
    // one tree can act on the other, which is the whole point of the key.
    for first in [true, false] {
        let mut app = app(first);
        assert!(
            matches!(
                tap(&mut app, Selector::Key("increment".to_string())),
                Ok(Payload::Done { .. })
            ),
            "the key resolves with the child first = {first}"
        );
    }
}

#[test]
fn an_unknown_key_is_refused_with_what_is_on_screen() {
    let mut app = app(true);
    match tap(&mut app, Selector::Key("nowhere".to_string())) {
        Err(AgentError::SelectorNotFound {
            selector,
            on_screen,
        }) => {
            assert_eq!(selector, "key \"nowhere\"");
            assert!(
                on_screen.iter().any(|label| label == "Increment"),
                "the refusal says what is on screen: {on_screen:?}"
            );
        }
        other => panic!("an unknown key must be refused: {other:?}"),
    }
}
