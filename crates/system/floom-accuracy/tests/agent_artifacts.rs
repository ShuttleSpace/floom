// SPDX-License-Identifier: MIT
//! What a failed command leaves behind: the command, the tree, the semantics
//! and the pixels, in one directory a person can open.
//!
//! "No node with that label" is a sentence; the evidence is what the window
//! looked like when it said so. Without it, every failure is reproduced by hand.

use std::path::PathBuf;

use floom_accuracy::agent::{AgentSession, Command, Selector};
use floom_core::{Axis, Child, ChildKey, LayoutRequest};
use floom_testkit::TestApp;
use floom_widgets::{Button, Stack};

fn app() -> TestApp {
    let child = Child::new(
        ChildKey::from("increment"),
        LayoutRequest::new().width(160.0).height(40.0),
        || Box::new(Button::new("Increment")),
    )
    .named("increment");
    TestApp::new(
        floom_types::Size::new(320.0, 120.0),
        Stack::new(Axis::Vertical).child(child),
    )
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("floom-artifacts-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_refused_command_leaves_what_it_saw() {
    let dir = scratch("refused");
    let mut app = app();
    let refusal = AgentSession::new(&mut app).with_artifact_dir(&dir).handle(
        floom_accuracy::agent::AgentRequest {
            version: floom_accuracy::agent::PROTOCOL_VERSION,
            id: 1,
            token: None,
            command: Command::Tap {
                selector: Selector::Key("nowhere".to_string()),
            },
        },
    );
    assert!(refusal.result.is_err(), "the command was refused");

    let failure = dir.join("failure-01");
    for name in [
        "README.md",
        "command.json",
        "tree.json",
        "semantics.json",
        "screenshot.png",
    ] {
        assert!(
            failure.join(name).exists(),
            "{name} is part of the artefact: {:?}",
            std::fs::read_dir(&failure).map(|entries| entries
                .filter_map(|e| e.ok().map(|e| e.file_name()))
                .collect::<Vec<_>>())
        );
    }

    let command = std::fs::read_to_string(failure.join("command.json")).expect("command.json");
    assert!(
        command.contains("nowhere"),
        "the artefact names what was asked for: {command}"
    );
    let readme = std::fs::read_to_string(failure.join("README.md")).expect("README.md");
    assert!(
        readme.contains("screenshot.png") && readme.contains("semantics.json"),
        "the README says what each file is: {readme}"
    );
    let png = std::fs::read(failure.join("screenshot.png")).expect("screenshot");
    assert!(
        png.starts_with(b"\x89PNG"),
        "the screenshot is a PNG, not a placeholder"
    );
    assert!(!png.is_empty(), "the screenshot has bytes");
    let tree = std::fs::read_to_string(failure.join("tree.json")).expect("tree.json");
    assert!(
        tree.contains("Increment"),
        "the tree shows what was on screen: {tree}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_command_that_works_writes_nothing() {
    let dir = scratch("answered");
    let mut app = app();
    let answer = AgentSession::new(&mut app).with_artifact_dir(&dir).handle(
        floom_accuracy::agent::AgentRequest {
            version: floom_accuracy::agent::PROTOCOL_VERSION,
            id: 1,
            token: None,
            command: Command::Tap {
                selector: Selector::Key("increment".to_string()),
            },
        },
    );
    assert!(answer.result.is_ok(), "the key resolves: {answer:?}");
    assert!(
        !dir.exists(),
        "an answered command leaves no directory behind: {dir:?}"
    );
}

#[test]
fn every_failure_gets_its_own_directory() {
    let dir = scratch("both");
    let mut app = app();
    for (id, key) in [(1, "first"), (2, "second")] {
        let _ = AgentSession::new(&mut app).with_artifact_dir(&dir).handle(
            floom_accuracy::agent::AgentRequest {
                version: floom_accuracy::agent::PROTOCOL_VERSION,
                id,
                token: None,
                command: Command::Tap {
                    selector: Selector::Key(key.to_string()),
                },
            },
        );
    }
    assert!(dir.join("failure-01").exists() && dir.join("failure-02").exists());
    let _ = std::fs::remove_dir_all(&dir);
}
