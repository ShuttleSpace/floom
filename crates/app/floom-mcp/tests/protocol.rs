// SPDX-License-Identifier: MIT
//! The MCP surface, driven through a stub backend: the protocol shape is what is
//! under test, not the window. `floom-window-software --agent` already covers
//! "the window answers"; this covers "a harness can find and call the tools".

use std::io::Cursor;

use floom_accuracy::agent::{Command, Payload};
use floom_mcp::{Backend, serve};
use serde_json::{Value, json};

/// Answers every command with a recognisable payload and records what it was asked.
struct Stub {
    asked: std::cell::RefCell<Vec<String>>,
}

impl Backend for Stub {
    fn call(&self, command: Command) -> Result<Payload, String> {
        self.asked.borrow_mut().push(format!("{command:?}"));
        match command {
            Command::Tap { .. } | Command::TypeText { .. } => Ok(Payload::Done { repainted: true }),
            Command::Tree => Ok(Payload::Tree(Default::default())),
            other => Err(format!("the stub does not answer {other:?}")),
        }
    }
}

fn exchange(lines: &[Value]) -> (Vec<Value>, Stub) {
    let text: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
    run_raw(&text)
}

fn run_raw(lines: &[String]) -> (Vec<Value>, Stub) {
    let stub = Stub {
        asked: std::cell::RefCell::new(Vec::new()),
    };
    let mut input = String::new();
    for line in lines {
        input.push_str(line);
        input.push('\n');
    }
    let mut output = Vec::new();
    serve(Cursor::new(input), &mut output, &stub).expect("stdio ends cleanly");
    let replies = String::from_utf8(output)
        .expect("utf-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("each line is JSON"))
        .collect();
    (replies, stub)
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

#[test]
fn an_initialize_answers_the_protocol_version_and_a_name() {
    let (replies, _) = exchange(&[request(
        1,
        "initialize",
        json!({"protocolVersion": "2024-11-05", "capabilities": {}}),
    )]);
    assert_eq!(replies.len(), 1);
    let result = &replies[0]["result"];
    assert_eq!(result["protocolVersion"], "2024-11-05");
    assert_eq!(result["serverInfo"]["name"], "floom");
    assert!(result["capabilities"]["tools"].is_object());
}

#[test]
fn the_tool_list_names_every_tool_and_its_schema() {
    let (replies, _) = exchange(&[request(1, "tools/list", json!({}))]);
    let tools = replies[0]["result"]["tools"].as_array().expect("tools");
    let names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(
        names,
        vec![
            "floom_tree",
            "floom_semantics",
            "floom_tap",
            "floom_type",
            "floom_screenshot"
        ]
    );
    for tool in tools {
        assert!(
            tool["description"]
                .as_str()
                .is_some_and(|text| !text.is_empty()),
            "{} has a description",
            tool["name"]
        );
        assert_eq!(
            tool["inputSchema"]["type"], "object",
            "{} has an object schema",
            tool["name"]
        );
    }
}

#[test]
fn tapping_by_key_reaches_the_backend_as_a_key_selector() {
    let (replies, stub) = exchange(&[request(
        7,
        "tools/call",
        json!({"name": "floom_tap", "arguments": {"key": "increment"}}),
    )]);
    assert_eq!(replies[0]["id"], 7);
    assert_eq!(replies[0]["result"]["isError"], false);
    assert!(
        stub.asked.borrow()[0].contains("Key(\"increment\")"),
        "the key travelled through: {:?}",
        stub.asked.borrow()
    );
}

#[test]
fn a_refused_call_is_a_result_the_model_can_read_not_a_transport_error() {
    let (replies, _) = exchange(&[request(
        1,
        "tools/call",
        json!({"name": "floom_tap", "arguments": {}}),
    )]);
    let result = &replies[0]["result"];
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("key, label or role")),
        "the refusal says what to pass: {result:?}"
    );
    assert!(replies[0].get("error").is_none(), "not a JSON-RPC error");
}

#[test]
fn an_unknown_method_is_a_json_rpc_error() {
    let (replies, _) = exchange(&[request(1, "floom/nonexistent", json!({}))]);
    assert_eq!(replies[0]["error"]["code"], -32601);
}

#[test]
fn a_notification_gets_no_reply() {
    let (replies, _) =
        exchange(&[json!({"jsonrpc": "2.0", "method": "notifications/initialized"})]);
    assert!(replies.is_empty(), "nothing answers a notification");
}

#[test]
fn a_line_that_is_not_json_is_reported_and_the_next_line_still_works() {
    let (replies, _) = run_raw(&[
        "not json at all".to_string(),
        request(2, "tools/list", json!({})).to_string(),
    ]);
    assert_eq!(replies.len(), 2, "the bad line was reported, not fatal");
    assert_eq!(replies[0]["error"]["code"], -32700);
    assert!(replies[1]["result"]["tools"].is_array());
}

#[test]
fn a_backend_that_does_not_answer_says_so_instead_of_inventing_a_tree() {
    let (replies, _) = exchange(&[request(
        1,
        "tools/call",
        json!({"name": "floom_screenshot", "arguments": {}}),
    )]);
    let result = &replies[0]["result"];
    assert_eq!(result["isError"], true);
    assert!(
        result["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("does not answer")),
        "{result:?}"
    );
}
