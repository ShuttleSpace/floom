// SPDX-License-Identifier: MIT
//! The loopback bridge, end to end: a client on 127.0.0.1 asks, the app thread
//! answers, the answer comes back to the client.
//!
//! This is the half the window example cannot check on every host (it asserts a
//! selftest over the in-memory transport instead), so the TCP forwarding gets a
//! test of its own that runs wherever `cargo test` runs.

use std::sync::mpsc;

use floom_accuracy::agent::{AgentClient, AgentSession, Command, Payload, bind, spawn_bridge};
use floom_core::Axis;
use floom_testkit::TestApp;
use floom_widgets::{Button, Stack};

fn app() -> TestApp {
    TestApp::new(
        floom_types::Size::new(320.0, 240.0),
        Stack::new(Axis::Vertical).child(floom_core::Child::new(
            floom_core::ChildKey::index(0),
            floom_core::LayoutRequest::new().width(160.0).height(40.0),
            || Box::new(Button::new("Increment")),
        )),
    )
}

#[test]
fn a_request_crosses_the_bridge_and_its_answer_comes_back() {
    let listener = bind("127.0.0.1", 0).expect("loopback binds");
    let addr = listener
        .local_addr()
        .expect("a bound listener has an address");
    let requests = spawn_bridge(listener).expect("the bridge thread starts");

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let app_thread = std::thread::spawn(move || {
        let mut app = app();
        loop {
            if stop_rx.try_recv().is_ok() {
                return;
            }
            let Ok(pending) = requests.recv_timeout(std::time::Duration::from_millis(50)) else {
                continue;
            };
            let response = AgentSession::new(&mut app).handle(pending.request().clone());
            pending.answer(response);
        }
    });

    let client = AgentClient::connect(addr, None);
    let tree = client
        .call(Command::Tree)
        .expect("the tree crosses the bridge");
    let Payload::Tree(tree) = tree else {
        panic!("Tree answered with {tree:?}");
    };
    assert!(
        tree.nodes
            .iter()
            .any(|node| node.label.as_deref() == Some("Increment")),
        "the button is in the tree the client read: {tree:?}"
    );

    let tapped = client.call(Command::Tap {
        selector: floom_accuracy::agent::Selector::Label("Increment".to_string()),
    });
    assert!(
        matches!(tapped, Ok(Payload::Done { .. })),
        "the tap crosses the bridge: {tapped:?}"
    );

    stop_tx.send(()).expect("the app thread is listening");
    app_thread.join().expect("the app thread leaves");
}

#[test]
fn a_port_already_in_use_is_an_error_rather_than_a_silent_second_listener() {
    let first = bind("127.0.0.1", 0).expect("loopback binds");
    let port = first.local_addr().expect("an address").port();
    assert!(
        bind("127.0.0.1", port).is_err(),
        "two listeners cannot share a port"
    );
}
