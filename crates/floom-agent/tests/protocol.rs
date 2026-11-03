//! Protocol round-trips and the version gate.
//!
//! The protocol is serde all the way down, so "round trip" is not a courtesy —
//! it is the contract: a client that serialises a command must be able to read
//! its own response back, and a version gate with concrete refusal rules.

use floom_agent::{
    AgentError, AgentRequest, Command, PROTOCOL_VERSION, Payload, PerformanceSummary, PointDto,
    ScreenDto, Selector, SemanticsEntryDto, UiNodeInfo, UiTree, check_version,
};
use floom_inspector::{ProtocolVersion, SceneLines};

#[test]
fn commands_round_trip_over_json() {
    let commands = vec![
        Command::Tree,
        Command::Semantics,
        Command::Screenshot,
        Command::Performance,
        Command::Schema,
        Command::Tap {
            selector: Selector::Label("Agree".into()),
        },
        Command::Tap {
            selector: Selector::Role("checkbox".into()),
        },
        Command::TypeText {
            selector: Selector::Label("Name".into()),
            text: "Ada".into(),
        },
        Command::Drag {
            from: PointDto::new(10.0, 20.0),
            to: PointDto::new(30.0, 40.0),
        },
    ];
    for command in commands {
        let request = AgentRequest {
            version: PROTOCOL_VERSION,
            id: 7,
            token: Some("dev-token".into()),
            command: command.clone(),
        };
        let text = serde_json::to_string(&request).expect("serialise");
        let parsed: AgentRequest = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, request, "{command:?} did not survive the wire");
    }
}

#[test]
fn payloads_round_trip_over_json() {
    let payloads = vec![
        Payload::Done { repainted: true },
        Payload::Tree(UiTree {
            nodes: vec![UiNodeInfo {
                id: 3,
                role: "checkbox".into(),
                label: Some("Agree".into()),
                checked: true,
                disabled: false,
                focused: false,
                x: 8.0,
                y: 4.0,
                width: 120.0,
                height: 24.0,
            }],
        }),
        Payload::Semantics(vec![SemanticsEntryDto {
            role: "checkbox".into(),
            label: Some("Agree".into()),
            checked: true,
            disabled: false,
            focused: false,
        }]),
        Payload::Scene(SceneLines {
            version: PROTOCOL_VERSION,
            lines: vec!["Rect { .. }".into()],
        }),
        Payload::Screenshot(ScreenDto {
            width: 320,
            height: 120,
            hash: "0123456789abcdef".into(),
        }),
        Payload::Performance(PerformanceSummary {
            interactions: 3,
            repaints: 2,
            taps: 2,
            drags: 0,
            text_commits: 1,
        }),
    ];
    for payload in payloads {
        let text = serde_json::to_string(&payload).expect("serialise");
        let parsed: Payload = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, payload, "{payload:?} did not survive the wire");
    }
}

#[test]
fn errors_round_trip_over_json() {
    let errors = vec![
        AgentError::VersionMismatch("reader is 1.0, request is 2.0".into()),
        AgentError::AuthRequired,
        AgentError::SelectorNotFound {
            selector: "label \"Nope\"".into(),
            on_screen: vec!["Agree".into()],
        },
        AgentError::Unsupported("unknown thing".into()),
        AgentError::Io("connection closed".into()),
    ];
    for error in errors {
        let text = serde_json::to_string(&error).expect("serialise");
        let parsed: AgentError = serde_json::from_str(&text).expect("parse");
        assert_eq!(parsed, error, "{error:?} did not survive the wire");
    }
}

#[test]
fn the_version_gate_accepts_the_current_and_older_minors() {
    assert_eq!(check_version(PROTOCOL_VERSION), Ok(()));
    assert_eq!(check_version(ProtocolVersion::new(1, 0)), Ok(()));
}

#[test]
fn the_version_gate_refuses_a_newer_minor_or_major() {
    let newer_minor = check_version(ProtocolVersion::new(1, 99));
    assert!(
        matches!(newer_minor, Err(AgentError::VersionMismatch(_))),
        "a newer minor must be refused: {newer_minor:?}"
    );
    let newer_major = check_version(ProtocolVersion::new(2, 0));
    assert!(
        matches!(newer_major, Err(AgentError::VersionMismatch(_))),
        "a new major must be refused: {newer_major:?}"
    );
}
