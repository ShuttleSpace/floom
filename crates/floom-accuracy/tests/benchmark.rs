//! Task 27.3 — the agent repair benchmark.
//!
//! An agent is handed a design contract and an imperfect implementation, may
//! inspect through the agent protocol (Phase 25), and proposes token edits.
//! The harness measures how many passes convergence took and *counts
//! regressions* — a repair that broke a path that used to be clean.

use floem_accuracy::{
    DiffDrivenPolicy, Screen, ScreenTokens, Thresholds, TokenEdit, run_benchmark,
};

#[test]
fn a_diff_driven_policy_repairs_a_drifted_screen() {
    let drifted = ScreenTokens::reference().drifted(&["gap", "radius", "accent", "body_size"]);
    let run = run_benchmark(Screen::Settings, &drifted, &mut DiffDrivenPolicy::new());
    assert!(
        run.converged,
        "did not converge: {}",
        run.remaining_problems
    );
    assert_eq!(run.regressions, 0, "the clean policy regressed something");
    assert!(
        Thresholds::default().accepts(&run),
        "a converged, regression-free run must be accepted"
    );
}

#[test]
fn the_benchmark_measures_convergence_across_screens() {
    // Every drift the token table can express, on three screen shapes.
    let drifted = ScreenTokens::reference().drifted(ScreenTokens::ALL_DRIFTS);
    for screen in [Screen::Settings, Screen::Dashboard, Screen::Form] {
        let run = run_benchmark(screen, &drifted, &mut DiffDrivenPolicy::new());
        assert!(
            run.converged,
            "{screen:?} left {} problems after {} passes",
            run.remaining_problems, run.iterations
        );
        assert_eq!(run.regressions, 0, "{screen:?} thrashed");
    }
}

/// A policy that fixes whatever it is shown *and* shatters a corner radius
/// that started clean. The harness has to count that as a regression, or a
/// benchmark would reward thrash. (The radius is clamped by the painter, but
/// clamped to 24 px, which is still nowhere near the contract's 10.)
#[derive(Default)]
struct BlunderingPolicy;

impl floem_accuracy::RepairPolicy for BlunderingPolicy {
    fn propose(&mut self, report: &floem_accuracy::EvaluationReport) -> Vec<TokenEdit> {
        if report.diff.entries.is_empty() {
            return Vec::new();
        }
        let mut edits = DiffDrivenPolicy::new().propose(report);
        edits.push(TokenEdit::Radius(999.0));
        edits
    }
}

#[test]
fn a_blundering_policy_earns_regressions() {
    let drifted = ScreenTokens::reference().drifted(&["gap", "accent"]);
    let run = run_benchmark(Screen::Settings, &drifted, &mut BlunderingPolicy);
    assert!(run.regressions >= 1, "thrash went uncounted: {run:?}");
    assert!(!run.converged);
    assert!(!Thresholds::default().accepts(&run));
}

/// A policy that proposes nothing: the loop must stall honestly, not claim
/// convergence on a screen it never touched.
#[derive(Default)]
struct DoNothingPolicy;

impl floem_accuracy::RepairPolicy for DoNothingPolicy {
    fn propose(&mut self, _report: &floem_accuracy::EvaluationReport) -> Vec<TokenEdit> {
        Vec::new()
    }
}

#[test]
fn thresholds_reject_a_stalled_run() {
    let drifted = ScreenTokens::reference().drifted(&["gap"]);
    let run = run_benchmark(Screen::Settings, &drifted, &mut DoNothingPolicy);
    assert!(!run.converged);
    assert!(!Thresholds::default().accepts(&run));
}

#[test]
fn proposed_edits_name_schema_declared_properties() {
    // Every edit the policy can propose lowers to a property the IR schema
    // declares, so an agent that reads the schema (Phase 25.2) authors edits
    // that a validator would accept.
    let edits = vec![
        TokenEdit::Gap(8.0),
        TokenEdit::Padding(12.0),
        TokenEdit::Radius(6.0),
        TokenEdit::TitleSize(16.0),
        TokenEdit::BodySize(15.0),
        TokenEdit::ControlHeight(44.0),
        TokenEdit::Accent(floem_types::Color::rgb(0xBA, 0x68, 0xC8)),
        TokenEdit::OnAccent(floem_types::Color::rgb(0x12, 0x12, 0x12)),
        TokenEdit::Surface(floem_types::Color::rgb(0xF0, 0xF0, 0xF0)),
        TokenEdit::Card(floem_types::Color::rgb(0xE0, 0xE0, 0xE0)),
        TokenEdit::Text(floem_types::Color::rgb(0x00, 0x00, 0x00)),
    ];
    for edit in edits {
        assert!(
            floem_ui_ir::prop_is_declared(edit.property_name()),
            "{} is not in the schema",
            edit.property_name()
        );
    }
}

#[test]
fn the_agent_protocol_carries_the_inspection_loop() {
    // The benchmark's information is available over the Phase 25 protocol:
    // labels via Semantics, geometry via Tree, the edit vocabulary via Schema.
    use floem_agent::{AgentRequest, AgentSession, Command, PROTOCOL_VERSION, Payload};

    let mut app = floem_testkit::TestApp::new(
        floem_accuracy::screen_size(),
        Screen::Settings.build(&ScreenTokens::reference()),
    );
    let mut session = AgentSession::new(&mut app);

    let tree = session.handle(AgentRequest {
        version: PROTOCOL_VERSION,
        id: 1,
        token: None,
        command: Command::Tree,
    });
    let nodes = match tree.result {
        Ok(Payload::Tree(tree)) => tree.nodes,
        other => panic!("tree request failed: {other:?}"),
    };
    assert!(nodes.iter().any(|n| n.label.as_deref() == Some("Settings")));

    let schema = session.handle(AgentRequest {
        version: PROTOCOL_VERSION,
        id: 2,
        token: None,
        command: Command::Schema,
    });
    match schema.result {
        Ok(Payload::Schema(schema)) => assert!(
            schema
                .widgets
                .iter()
                .any(|w| w.properties.iter().any(|p| p.name == "radius")),
            "the schema did not expose radius"
        ),
        other => panic!("schema request failed: {other:?}"),
    }
}
