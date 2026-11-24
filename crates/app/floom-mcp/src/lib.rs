// SPDX-License-Identifier: MIT
//! A Model Context Protocol server in front of the agent protocol.
//!
//! The protocol already answers "what is on screen" and "tap this"; MCP is the
//! shape a harness expects to find those in. This crate is the adapter and
//! nothing else: it owns no window, and it cannot start an app -- it attaches to
//! a socket that `floom-window-software --agent <port>` is already serving.
//!
//! That split is deliberate. A server that also launched processes would have to
//! decide who owns the window, what happens when the app exits, and where the
//! app's stderr goes; an adapter only has to translate.
//!
//! Transport is stdio with newline-delimited JSON-RPC 2.0, which is what MCP
//! servers use by default.

use std::io::{BufRead, Write};

use floom_accuracy::agent::{AgentClient, Command, Payload, Selector};
use serde_json::{Value, json};

/// What the server can ask a running app. A trait so the protocol can be tested
/// without a window: the tests drive a stub, the binary drives the socket.
pub trait Backend {
    fn call(&self, command: Command) -> Result<Payload, String>;
}

impl Backend for AgentClient {
    fn call(&self, command: Command) -> Result<Payload, String> {
        AgentClient::call(self, command).map_err(|error| format!("{error:?}"))
    }
}

const PROTOCOL_VERSION: &str = "2024-11-05";

fn tools() -> Value {
    json!([
        {
            "name": "floom_tree",
            "description": "Every node on screen: role, label, bounds and the author's key. \
                            Start here; the keys are what the other tools take.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}
        },
        {
            "name": "floom_semantics",
            "description": "The semantics tree as a screen reader would read it.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}
        },
        {
            "name": "floom_tap",
            "description": "Tap a node by its author key, its label, or its role.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": {"type": "string", "description": "the Child::named key"},
                    "label": {"type": "string"},
                    "role": {"type": "string", "description": "e.g. button, text_input"}
                },
                "additionalProperties": false
            }
        },
        {
            "name": "floom_type",
            "description": "Type into the node matching a selector. Needs a focused text field.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "key": {"type": "string"},
                    "label": {"type": "string"}
                },
                "required": ["text"],
                "additionalProperties": false
            }
        },
        {
            "name": "floom_screenshot",
            "description": "A hash and the size of what was last presented. Not the pixels: \
                            the protocol reports that a frame changed, not what it looked like.",
            "inputSchema": {"type": "object", "properties": {}, "additionalProperties": false}
        }
    ])
}

fn selector(arguments: &Value) -> Result<Selector, String> {
    if let Some(key) = arguments.get("key").and_then(Value::as_str) {
        return Ok(Selector::Key(key.to_string()));
    }
    if let Some(label) = arguments.get("label").and_then(Value::as_str) {
        return Ok(Selector::Label(label.to_string()));
    }
    if let Some(role) = arguments.get("role").and_then(Value::as_str) {
        return Ok(Selector::Role(role.to_string()));
    }
    Err("give one of key, label or role".to_string())
}

fn tool_call(name: &str, arguments: &Value, backend: &dyn Backend) -> Result<Value, String> {
    let command = match name {
        "floom_tree" => Command::Tree,
        "floom_semantics" => Command::Semantics,
        "floom_screenshot" => Command::Screenshot,
        "floom_tap" => Command::Tap {
            selector: selector(arguments)?,
        },
        "floom_type" => Command::TypeText {
            selector: selector(arguments)?,
            text: arguments
                .get("text")
                .and_then(Value::as_str)
                .ok_or("floom_type needs text")?
                .to_string(),
        },
        other => return Err(format!("there is no tool called {other}")),
    };
    let payload = backend.call(command)?;
    let text = serde_json::to_string_pretty(&payload).map_err(|error| error.to_string())?;
    Ok(json!({"content": [{"type": "text", "text": text}], "isError": false}))
}

fn handle(request: &Value, backend: &dyn Backend) -> Option<Value> {
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    // A notification has no id and takes no answer.
    let answer = |result: Value| {
        id.clone()
            .map(|id| json!({"jsonrpc": "2.0", "id": id, "result": result}))
    };
    let failed = |code: i32, message: String| {
        id.clone().map(
            |id| json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
        )
    };

    match method {
        "initialize" => answer(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "floom", "version": env!("CARGO_PKG_VERSION")}
        })),
        "notifications/initialized" => None,
        "ping" => answer(json!({})),
        "tools/list" => answer(json!({"tools": tools()})),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(Value::Null);
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            match tool_call(name, &arguments, backend) {
                Ok(result) => answer(result),
                // A refused tool call is a *result* with isError, not a JSON-RPC
                // error: the harness shows it to the model, which can act on it.
                Err(message) => answer(json!({
                    "content": [{"type": "text", "text": message}],
                    "isError": true
                })),
            }
        }
        other => failed(-32601, format!("unknown method {other}")),
    }
}

/// Serve MCP on `input` until it ends, writing one JSON-RPC message per line.
pub fn serve(
    input: impl BufRead,
    mut output: impl Write,
    backend: &dyn Backend,
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(trimmed) {
            Ok(request) => request,
            Err(error) => {
                let reply = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": {"code": -32700, "message": format!("not JSON: {error}")}
                });
                writeln!(output, "{reply}")?;
                continue;
            }
        };
        if let Some(reply) = handle(&request, backend) {
            writeln!(output, "{reply}")?;
            output.flush()?;
        }
    }
    Ok(())
}
