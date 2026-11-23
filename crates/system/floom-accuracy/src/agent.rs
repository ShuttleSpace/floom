// SPDX-License-Identifier: MIT
//! The agent protocol over a running Floom 2.0 app (Phase 25).
//!
//! Three pieces, each testable alone:
//!
//! * [`protocol`] — the wire: versioned requests and responses, inspection
//!   and control commands, and an error type that tells an agent what to do
//!   next (a missed selector comes back with what *is* on screen).
//! * [`schema`] — the AI-friendly component schema, emitted from the same
//!   `floom_dsl::ui_ir` tables the validator enforces, so an agent that reads it
//!   authors documents that validate.
//! * [`session`] + [`transport`] — the running-app loop (Task 25.3): a
//!   session drives an app through [`AgentApp`], the trait a headless test
//!   (`TestApp`) and a live window ([`WindowApp`]) both implement, and three
//!   transports carry the protocol to it — an in-memory channel for tests, a
//!   localhost-only TCP server with a dev token for tools, and a bridge that
//!   hands requests to whichever thread owns the window.
//!
//! # Why the selectors are semantic
//!
//! The protocol addresses the screen the way accessibility does — by label
//! and role. Host-internal node ids are handles a client cannot forge and
//! cannot keep alive across reloads; a protocol that accepted one would let
//! an agent act on a stale handle and read back a tree that is no longer
//! there.
//!
//! # Why the transport is loopback-only
//!
//! The control commands are the app's input, and the inspection commands are
//! its whole tree. A debugging surface that listened on a routable address —
//! with or without a token — would hand both to the network. `bind` refuses a
//! non-loopback host outright and the server drops non-loopback peers, so the
//! guard is the transport, not a convention.

#![forbid(unsafe_code)]

mod protocol;
mod schema;
mod session;
mod transport;
mod window;

pub use protocol::{
    AgentError, AgentRequest, AgentResponse, Command, PROTOCOL_VERSION, Payload,
    PerformanceSummary, PointDto, ScreenDto, Selector, SemanticsEntryDto, UiNodeInfo, UiTree,
    check_version,
};
pub use schema::{ComponentSchema, PropertyInfo, TokenRel, WidgetInfo, component_schema};
pub use session::{AgentApp, AgentSession};
pub use transport::{
    AgentClient, LocalClient, LocalServer, Pending, bind, is_loopback_peer, local_channel,
    spawn_bridge, spawn_loopback,
};
pub use window::WindowApp;
