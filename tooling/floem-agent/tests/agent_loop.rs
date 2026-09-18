//! The agent loop over a headless app (Task 25.3), driven through the local
//! channel transport.
//!
//! The fixture is a real widget tree — a checkbox and a text field — and the
//! agent only ever sees protocol responses. Every assertion is made from what
//! crossed the wire: a screenshot hash, a semantics entry, a counter.

use floem_agent::{
    AgentError, AgentSession, Command, Command::*, LocalClient, LocalServer, Payload, Selector,
};
use floem_core::Axis;
use floem_testkit::TestApp;
use floem_widgets::{Checkbox, Stack, TextField};

fn fixture() -> TestApp {
    let stack = Stack::new(Axis::Vertical)
        .gap(12.0)
        .push(|| Box::new(Checkbox::new("Agree")))
        .push(|| Box::new(TextField::new().placeholder("Name")));
    TestApp::new(floem_types::Size::new(320.0, 160.0), stack)
}

/// One request in, one response out, through the channel transport.
fn call(
    client: &LocalClient,
    server: &mut LocalServer,
    session: &mut AgentSession,
    command: Command,
) -> Result<Payload, AgentError> {
    client.send(command);
    while let Some(request) = server.next_request() {
        server.respond(session.handle(request));
    }
    client
        .receive()
        .expect("the server answered every request")
        .result
}

fn checked(
    client: &LocalClient,
    server: &mut LocalServer,
    session: &mut AgentSession,
) -> Option<bool> {
    match call(client, server, session, Semantics) {
        Ok(Payload::Semantics(entries)) => entries
            .iter()
            .find(|e| e.label.as_deref() == Some("Agree"))
            .map(|e| e.checked),
        other => panic!("semantics request returned {other:?}"),
    }
}

#[test]
fn tap_flips_the_checkbox_and_the_screen_changes() {
    let mut app = fixture();
    let (client, mut server) = floem_agent::local_channel();
    let mut session = AgentSession::new(&mut app);

    let before = {
        let shot = call(&client, &mut server, &mut session, Screenshot)
            .expect("screenshot before the tap");
        match shot {
            Payload::Screenshot(s) => s.hash,
            other => panic!("screenshot returned {other:?}"),
        }
    };
    assert_eq!(checked(&client, &mut server, &mut session), Some(false));

    let repainted = match call(
        &client,
        &mut server,
        &mut session,
        Tap {
            selector: Selector::Label("Agree".into()),
        },
    ) {
        Ok(Payload::Done { repainted }) => repainted,
        other => panic!("tap returned {other:?}"),
    };
    assert!(repainted, "a state change that paints nothing is a bug");

    assert_eq!(checked(&client, &mut server, &mut session), Some(true));
    let after = match call(&client, &mut server, &mut session, Screenshot) {
        Ok(Payload::Screenshot(s)) => s.hash,
        other => panic!("screenshot returned {other:?}"),
    };
    assert_ne!(before, after, "a checked box must draw differently");
}

#[test]
fn type_text_reaches_the_field_over_the_wire() {
    let mut app = fixture();
    let (client, mut server) = floem_agent::local_channel();
    let mut session = AgentSession::new(&mut app);

    let repainted = match call(
        &client,
        &mut server,
        &mut session,
        TypeText {
            selector: Selector::Label("Name".into()),
            text: "Ada".into(),
        },
    ) {
        Ok(Payload::Done { repainted }) => repainted,
        other => panic!("type_text returned {other:?}"),
    };
    assert!(repainted, "committed text must repaint");

    match call(&client, &mut server, &mut session, Semantics) {
        Ok(Payload::Semantics(entries)) => {
            let field = entries
                .iter()
                .find(|e| e.label.as_deref() == Some("Ada"))
                .expect("the field announces its value");
            assert_eq!(field.role, "textfield");
        }
        other => panic!("semantics returned {other:?}"),
    }
}

#[test]
fn a_missing_selector_is_refused_with_the_screen_contents() {
    let mut app = fixture();
    let (client, mut server) = floem_agent::local_channel();
    let mut session = AgentSession::new(&mut app);

    let miss = call(
        &client,
        &mut server,
        &mut session,
        Tap {
            selector: Selector::Label("Nope".into()),
        },
    );
    match miss {
        Err(AgentError::SelectorNotFound {
            selector,
            on_screen,
        }) => {
            assert!(selector.contains("Nope"));
            assert!(
                on_screen.iter().any(|l| l == "Agree"),
                "the error should name what IS on screen: {on_screen:?}"
            );
        }
        other => panic!("a missing selector returned {other:?}"),
    }
}

#[test]
fn the_tree_reports_live_nodes_with_geometry() {
    let mut app = fixture();
    let (client, mut server) = floem_agent::local_channel();
    let mut session = AgentSession::new(&mut app);

    match call(&client, &mut server, &mut session, Tree) {
        Ok(Payload::Tree(tree)) => {
            let agree = tree
                .nodes
                .iter()
                .find(|n| n.label.as_deref() == Some("Agree"))
                .expect("the checkbox is on screen");
            assert_eq!(agree.role, "checkbox");
            assert!(agree.width > 0.0 && agree.height > 0.0, "laid out");
        }
        other => panic!("tree returned {other:?}"),
    }
}

#[test]
fn performance_counters_add_up_over_the_session() {
    let mut app = fixture();
    let (client, mut server) = floem_agent::local_channel();
    let mut session = AgentSession::new(&mut app);

    for _ in 0..2 {
        call(
            &client,
            &mut server,
            &mut session,
            Tap {
                selector: Selector::Label("Agree".into()),
            },
        )
        .expect("tap");
    }
    call(
        &client,
        &mut server,
        &mut session,
        TypeText {
            selector: Selector::Label("Name".into()),
            text: "Grace".into(),
        },
    )
    .expect("type");

    match call(&client, &mut server, &mut session, Performance) {
        Ok(Payload::Performance(summary)) => {
            assert_eq!(summary.taps, 2);
            assert_eq!(summary.text_commits, 1);
            assert_eq!(summary.interactions, 3);
            assert!(summary.repaints >= 2, "the taps alone must have painted");
        }
        other => panic!("performance returned {other:?}"),
    }
}

#[test]
fn a_wrong_or_missing_token_is_refused_and_the_right_one_passes() {
    let mut app = fixture();
    let (client, mut server) = floem_agent::local_channel();
    let mut session = AgentSession::new(&mut app).with_token("s3cret");

    let refused = call(&client, &mut server, &mut session, Tree);
    assert_eq!(refused, Err(AgentError::AuthRequired));

    let wrong = client.send_with_token(Tree, Some("guess"));
    while let Some(request) = server.next_request() {
        server.respond(session.handle(request));
    }
    assert_eq!(
        client.receive().map(|r| r.result),
        Some(Err(AgentError::AuthRequired)),
        "a guessed token must not open the door (request {wrong})"
    );

    let accepted = client.send_with_token(Tree, Some("s3cret"));
    while let Some(request) = server.next_request() {
        server.respond(session.handle(request));
    }
    let response = client.receive().expect("answered");
    assert_eq!(response.id, accepted, "responses are matched by id");
    assert!(response.result.is_ok(), "the right token passes");
}
