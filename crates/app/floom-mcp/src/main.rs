// SPDX-License-Identifier: MIT
//! `floom-mcp --attach 127.0.0.1:4877` speaks MCP on stdio and forwards to an app
//! that is already serving the agent protocol.
//!
//! It does not start the app. A server that launched processes would own the
//! question of who holds the window and what happens when it exits; an adapter
//! only translates.

use std::io::{self, BufReader};
use std::net::SocketAddr;

use floom_accuracy::agent::AgentClient;

const USAGE: &str = "\
floom-mcp: a Model Context Protocol server in front of a running app.

    floom-mcp [--attach HOST:PORT]

Start the app with the agent protocol first:

    floom-window-software --agent 4877
    floom-mcp --attach 127.0.0.1:4877
";

fn main() -> Result<(), String> {
    let mut attach = "127.0.0.1:4877".to_string();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--attach" => attach = args.next().ok_or("--attach needs HOST:PORT")?,
            "--help" | "-h" => {
                print!("{USAGE}");
                return Ok(());
            }
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }

    let addr: SocketAddr = attach
        .parse()
        .map_err(|error| format!("--attach {attach} is not HOST:PORT: {error}"))?;
    let token = std::env::var("FLOOM_AGENT_TOKEN").ok();
    let client = AgentClient::connect(addr, token);

    let stdin = io::stdin();
    let stdout = io::stdout();
    floom_mcp::serve(BufReader::new(stdin.lock()), stdout.lock(), &client)
        .map_err(|error| format!("stdio ended: {error}"))
}
