// SPDX-License-Identifier: MIT
//! The AI/design accuracy loop (Phase 27).
//!
//! Three pieces that only make sense together:
//!
//! - [`screens`] is the reference gallery: nine representative screens built
//!   from real widgets as a pure function of a token table, each backed by a
//!   reference design contract fixture (Phase 26 vocabulary).
//! - [`evaluate`] is the render-and-compare loop: mount a screen headless,
//!   measure the rendered geometry, style, semantics and pixels, and diff the
//!   observed contract against the reference one.
//! - [`benchmark`] is the agent repair benchmark: hand a policy a drifted
//!   implementation, let it inspect and propose edits, and measure how fast it
//!   converges and whether it regresses anything along the way.
//!
//! The loop closes over vocabulary that already exists on purpose: the
//! contract comes from `floom-ui-ir`, the fingerprints from `floom-snapshot`,
//! the inspection surface from the `agent` module (Phase 25, folded in here) —
//! nothing here redefines what a disagreement is.

pub mod agent;
mod benchmark;
mod evaluate;
mod labeled;
mod screens;
mod tokens;

pub use agent::{
    AgentClient, AgentError, AgentRequest, AgentResponse, AgentSession, Command, ComponentSchema,
    LocalClient, LocalServer, PROTOCOL_VERSION, Payload, PerformanceSummary, PointDto,
    PropertyInfo, ScreenDto, Selector, SemanticsEntryDto, TokenRel, UiNodeInfo, UiTree, WidgetInfo,
    bind, check_version, component_schema, is_loopback_peer, local_channel, spawn_loopback,
};
pub use benchmark::{BenchmarkRun, DiffDrivenPolicy, RepairPolicy, Thresholds, run_benchmark};
pub use evaluate::{
    EvaluationReport, FloorFailure, Observation, evaluate_screen, reference_contract,
};
pub use screens::{Screen, screen_size};
pub use tokens::{ScreenTokens, TokenEdit};
