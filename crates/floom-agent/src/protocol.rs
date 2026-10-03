//! The wire protocol (Task 25.1).
//!
//! One request, one response, both fully serde — an agent in any language can
//! speak this without linking Floom. Inspection commands observe a running
//! app; control commands act on it. The version gate reuses the inspector's
//! rule verbatim, because "a reader must never be handed fields it cannot
//! represent" is the same contract in both directions of this protocol.

use floom_inspector::{ProtocolVersion, SceneLines};
use serde::{Deserialize, Serialize};

/// The version this crate writes. Requests carry it; a server refusing a
/// newer minor is the same rule the inspector applies to snapshots.
pub const PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::current();

/// The version gate, exposed for tests and for transports that want to reject
/// before the session is even involved.
pub fn check_version(offered: ProtocolVersion) -> Result<(), AgentError> {
    PROTOCOL_VERSION
        .accepts_err(offered)
        .map_err(AgentError::VersionMismatch)
}

/// What an agent asks the app to do.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRequest {
    pub version: ProtocolVersion,
    /// Matched verbatim in the response, so a concurrent client can pair them.
    pub id: u64,
    /// The dev token, when the server was started with one. Absent means
    /// "unauthenticated", which only a tokenless server accepts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    pub command: Command,
}

/// What came back. `id` pairs the response with its request; everything else
/// is the payload or the reason there is none.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: u64,
    pub result: Result<Payload, AgentError>,
}

/// Inspection and control commands.
///
/// Inspection answers *what is on screen*; control answers *what happens when
/// a person interacts*. An agent that loops screenshot → decide → act needs
/// exactly these two halves and nothing else.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    // --- inspection -------------------------------------------------------
    /// Every live node with its role, state and geometry, in mount order.
    Tree,
    /// What the accessibility tree publishes, sorted for a stable panel order.
    Semantics,
    /// The painted scene, one line per node — the same text the snapshot
    /// engine hashes.
    Scene,
    /// A deterministic screenshot: dimensions plus the pixel hash of the
    /// software-rendered frame.
    Screenshot,
    /// Session-observed interaction counters.
    Performance,
    /// The AI-friendly component schema (Task 25.2).
    Schema,

    // --- control ----------------------------------------------------------
    /// A click on whatever `selector` resolves to.
    Tap { selector: Selector },
    /// Focus `selector` and commit `text` — the path a real keyboard's
    /// composed input takes.
    TypeText { selector: Selector, text: String },
    /// Press at `from`, move to `to`, release.
    Drag { from: PointDto, to: PointDto },
}

/// How an agent names something on screen.
///
/// Deliberately two semantic selectors and no raw ids: a `NodeId` is a
/// host-internal handle with no constructor outside the app, and a protocol
/// that invited clients to guess one would invite them to operate on stale
/// handles. The screen is addressed the way accessibility sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Selector {
    /// The node whose published label is exactly this.
    Label(String),
    /// The first node whose role has this name (`"checkbox"`, `"button"`, …).
    Role(String),
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Selector::Label(label) => write!(f, "label {label:?}"),
            Selector::Role(role) => write!(f, "role {role:?}"),
        }
    }
}

/// A window position on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointDto {
    pub x: f64,
    pub y: f64,
}

impl PointDto {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// Why there is no payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentError {
    /// The request spoke a newer protocol than the server serves.
    VersionMismatch(String),
    /// The server has a token and the request did not present it.
    AuthRequired,
    /// The selector matched nothing. `on_screen` is what *is* there, so the
    /// agent can correct itself instead of re-guessing blind.
    SelectorNotFound {
        selector: String,
        on_screen: Vec<String>,
    },
    /// The command is known but not servable.
    Unsupported(String),
    /// The transport failed underneath the protocol.
    Io(String),
}

/// Everything a command can answer with.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Payload {
    Done { repainted: bool },
    Tree(UiTree),
    Semantics(Vec<SemanticsEntryDto>),
    Scene(SceneLines),
    Screenshot(ScreenDto),
    Performance(PerformanceSummary),
    Schema(crate::ComponentSchema),
}

/// One live node as the tree command serves it, in mount order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiNodeInfo {
    /// The node's position in mount order — the identity this protocol offers,
    /// stable within a session.
    pub id: u64,
    pub role: String,
    pub label: Option<String>,
    pub checked: bool,
    pub disabled: bool,
    pub focused: bool,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The tree payload.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UiTree {
    pub nodes: Vec<UiNodeInfo>,
}

/// One semantics entry, states flattened to the flags an agent acts on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticsEntryDto {
    pub role: String,
    pub label: Option<String>,
    pub checked: bool,
    pub disabled: bool,
    pub focused: bool,
}

/// A screenshot as the snapshot engine sees it: the pixel hash is the
/// fingerprint — two equal hashes are the same frame, a different hash is a
/// different frame — and the agent never ships pixels around to learn that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenDto {
    pub width: u32,
    pub height: u32,
    pub hash: String,
}

/// Session-observed counters.
///
/// These are what a headless session can measure honestly: how many commands
/// acted on the app and how many of them painted. Per-stage frame timings
/// belong to the profiler (Phase 23) and are not re-invented here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub interactions: u32,
    pub repaints: u32,
    pub taps: u32,
    pub drags: u32,
    pub text_commits: u32,
}
