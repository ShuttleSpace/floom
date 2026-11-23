// SPDX-License-Identifier: MIT
//! Ids an agent reads from `Tree`, used to act, and the promise that two runs of
//! the same app answer the same bytes.
//!
//! The id is a position in mount order, not an internal handle: it is stable for
//! as long as the node is on screen. That is what makes "read the tree, then tap
//! what you read" work without an agent inventing its own addressing.

use floom_accuracy::agent::{
    AgentError, AgentRequest, AgentSession, Command, PROTOCOL_VERSION, Payload, Selector, UiTree,
};
use floom_core::Axis;
use floom_testkit::TestApp;
use floom_widgets::{Button, Stack};

fn app() -> TestApp {
    let stack = Stack::new(Axis::Vertical).gap(8.0).push(|| {
        Box::new(
            Stack::new(Axis::Vertical)
                .push(|| Box::new(Button::new("Increment")))
                .push(|| Box::new(Button::new("Reset"))),
        )
    });
    TestApp::new(floom_types::Size::new(320.0, 120.0), stack)
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

fn tree(app: &mut TestApp) -> UiTree {
    match ask(app, Command::Tree) {
        Ok(Payload::Tree(tree)) => tree,
        other => panic!("Tree answered with {other:?}"),
    }
}

#[test]
fn an_id_read_from_the_tree_can_be_tapped() {
    let mut app = app();
    let read = tree(&mut app);
    let increment = read
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("Increment"))
        .expect("the tree has the button");

    match ask(
        &mut app,
        Command::Tap {
            selector: Selector::Id(increment.id),
        },
    ) {
        Ok(Payload::Done { repainted }) => assert!(repainted, "tapping the button repaints"),
        other => panic!("Tap by id answered with {other:?}"),
    }
}

#[test]
fn an_id_that_is_not_on_screen_is_refused_with_what_is() {
    let mut app = app();
    match ask(
        &mut app,
        Command::Tap {
            selector: Selector::Id(9_999),
        },
    ) {
        Err(AgentError::SelectorNotFound {
            selector,
            on_screen,
        }) => {
            assert_eq!(selector, "id 9999");
            assert!(
                on_screen.iter().any(|label| label == "Increment"),
                "the refusal says what is on screen: {on_screen:?}"
            );
        }
        other => panic!("an id off screen must be refused: {other:?}"),
    }
}

#[test]
fn two_runs_of_the_same_app_answer_the_same_tree_and_semantics() {
    // The id, the order and the geometry are all things a second run could
    // quietly change; an agent that caches any of them needs this to hold.
    let mut first = app();
    let mut second = app();
    let (first_tree, first_semantics) = (tree(&mut first), ask(&mut first, Command::Semantics));
    let (second_tree, second_semantics) = (tree(&mut second), ask(&mut second, Command::Semantics));
    assert_eq!(
        serde_json::to_string(&first_tree).expect("serialise"),
        serde_json::to_string(&second_tree).expect("serialise"),
        "the same app twice is the same tree"
    );
    assert_eq!(
        serde_json::to_string(&first_semantics).expect("serialise"),
        serde_json::to_string(&second_semantics).expect("serialise"),
        "the same app twice is the same semantics"
    );
}
