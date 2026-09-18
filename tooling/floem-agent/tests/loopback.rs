//! The loopback TCP transport: the localhost-only bind guard, the dev token,
//! and the whole agent loop across a real socket.
//!
//! The app is built *inside* the server thread — `spawn_loopback` takes a
//! factory, not an app — so nothing headless has to be `Send`.

use std::net::SocketAddr;

use floem_agent::{
    AgentClient, AgentError, Command, Payload, Selector, bind, is_loopback_peer, spawn_loopback,
};
use floem_core::Axis;
use floem_testkit::TestApp;
use floem_widgets::{Checkbox, Stack};

fn fixture() -> TestApp {
    let stack = Stack::new(Axis::Vertical)
        .gap(12.0)
        .push(|| Box::new(Checkbox::new("Agree")));
    TestApp::new(floem_types::Size::new(320.0, 120.0), stack)
}

#[test]
fn the_bind_guard_refuses_a_non_loopback_host() {
    let refused = bind("0.0.0.0", 0);
    assert!(
        refused.is_err(),
        "the agent transport must never listen on 0.0.0.0: {refused:?}"
    );
    let refused = bind("192.168.1.10", 0);
    assert!(refused.is_err(), "a LAN address is not a dev bind");

    let accepted = bind("127.0.0.1", 0);
    assert!(accepted.is_ok(), "loopback is the point: {accepted:?}");
}

#[test]
fn the_peer_guard_admits_only_loopback() {
    let good: SocketAddr = "127.0.0.1:5000".parse().expect("parse");
    let bad: SocketAddr = "192.168.1.10:5000".parse().expect("parse");
    assert!(is_loopback_peer(good));
    assert!(!is_loopback_peer(bad));
}

#[test]
fn a_round_trip_over_tcp_needs_the_token() {
    let addr = spawn_loopback(0, Some("dev-token".into()), fixture).expect("bind 127.0.0.1");
    assert!(addr.ip().is_loopback(), "the server bound loopback: {addr}");

    let anonymous = AgentClient::connect(addr, None);
    assert_eq!(
        anonymous.call(Command::Schema),
        Err(AgentError::AuthRequired)
    );

    let guessing = AgentClient::connect(addr, Some("guess".into()));
    assert_eq!(
        guessing.call(Command::Schema),
        Err(AgentError::AuthRequired)
    );

    let client = AgentClient::connect(addr, Some("dev-token".into()));
    let schema = client.call(Command::Schema).expect("authorised");
    match schema {
        Payload::Schema(s) => assert!(
            s.widgets.iter().any(|w| w.name == "Button"),
            "the schema payload is the real one"
        ),
        other => panic!("schema returned {other:?}"),
    }
}

#[test]
fn the_agent_loop_survives_the_wire() {
    let addr = spawn_loopback(0, None, fixture).expect("bind 127.0.0.1");
    let client = AgentClient::connect(addr, None);

    let before = match client.call(Command::Screenshot).expect("screenshot") {
        Payload::Screenshot(s) => s.hash,
        other => panic!("screenshot returned {other:?}"),
    };

    client
        .call(Command::Tap {
            selector: Selector::Label("Agree".into()),
        })
        .expect("tap");

    let after = match client.call(Command::Screenshot).expect("screenshot") {
        Payload::Screenshot(s) => s.hash,
        other => panic!("screenshot returned {other:?}"),
    };
    assert_ne!(before, after, "the tap must reach the pixels");

    match client.call(Command::Semantics).expect("semantics") {
        Payload::Semantics(entries) => {
            let agree = entries
                .iter()
                .find(|e| e.label.as_deref() == Some("Agree"))
                .expect("the checkbox is on screen");
            assert!(agree.checked, "the tap flipped the checkbox");
        }
        other => panic!("semantics returned {other:?}"),
    }

    match client.call(Command::Performance).expect("performance") {
        Payload::Performance(summary) => assert_eq!(summary.taps, 1, "one tap, counted once"),
        other => panic!("performance returned {other:?}"),
    }
}
