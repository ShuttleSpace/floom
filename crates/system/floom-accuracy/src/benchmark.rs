// SPDX-License-Identifier: MIT
//! Task 27.3 — the agent repair benchmark.
//!
//! An agent is handed a design contract and an imperfect implementation, may
//! inspect through the agent protocol (Phase 25), and proposes token edits.
//! The harness measures how many repair passes convergence took and *counts
//! regressions*: a pass that broke a contract path an earlier pass had already
//! made clean. Acceptance thresholds turn those two numbers into a verdict.

use std::collections::HashSet;

use floom_dsl::ui_ir::DesignContract;

use crate::evaluate::{EvaluationReport, evaluate_screen, reference_contract};
use crate::screens::Screen;
use crate::tokens::{ScreenTokens, TokenEdit};

/// What an agent does with an evaluation: propose the next round of edits.
/// Proposing nothing is how a policy says "I cannot improve this" — the loop
/// then ends honestly rather than looping.
pub trait RepairPolicy {
    fn propose(&mut self, report: &EvaluationReport) -> Vec<TokenEdit>;
}

/// One measured repair attempt.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BenchmarkRun {
    pub screen: String,
    /// Repair passes the policy used (propose + apply cycles).
    pub iterations: usize,
    pub converged: bool,
    pub regressions: usize,
    /// Problems left when the run ended.
    pub remaining_problems: usize,
}

/// The verdict rule. Defaults encode the bar the loop exists to defend:
/// converge fast and never break something that was already right.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Thresholds {
    pub max_iterations: usize,
    pub max_regressions: usize,
}

impl Thresholds {
    pub fn accepts(&self, run: &BenchmarkRun) -> bool {
        run.converged
            && run.iterations <= self.max_iterations
            && run.regressions <= self.max_regressions
    }
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            max_iterations: 8,
            max_regressions: 0,
        }
    }
}

/// Runs the loop: evaluate, propose, apply, re-evaluate — until clean, until
/// the policy stalls, or until the iteration budget runs out.
///
/// Regression counting needs the *universe* of comparable contract paths, and
/// it gets it from `compare` itself: the reference is compared against a copy
/// of itself with every comparable value perturbed, so every path `compare`
/// can ever emit is emitted once here. Restating the path list in this crate
/// would be a second implementation of the walk that would drift.
pub fn run_benchmark(
    screen: Screen,
    drifted: &ScreenTokens,
    policy: &mut dyn RepairPolicy,
) -> BenchmarkRun {
    let thresholds = Thresholds::default();
    let universe = comparable_paths(screen);
    let clean_at_start = universe.iter().cloned().collect::<HashSet<_>>();

    let mut tokens = drifted.clone();
    let mut report = evaluate_screen(screen, &tokens);
    let mut regressions = 0;
    let mut iterations = 0;
    // Paths observed clean at the start count too: a policy that re-breaks
    // one of them has regressed, exactly as if it had been clean mid-loop.
    let dirty_at = |report: &EvaluationReport| -> HashSet<String> {
        report
            .diff
            .entries
            .iter()
            .map(|entry| entry.path.clone())
            .collect()
    };
    let mut ever_clean: HashSet<String> = clean_at_start
        .difference(&dirty_at(&report))
        .cloned()
        .collect();

    let converged = loop {
        if report.is_clean() {
            break true;
        }
        if iterations >= thresholds.max_iterations {
            break false;
        }
        let edits = policy.propose(&report);
        iterations += 1;
        if edits.is_empty() {
            // Stalled honestly: the policy has nothing left to propose.
            break false;
        }
        for edit in &edits {
            edit.apply(&mut tokens);
        }
        report = evaluate_screen(screen, &tokens);

        let dirty = dirty_at(&report);
        for path in &dirty {
            if ever_clean.contains(path) {
                regressions += 1;
            }
        }
        for path in clean_at_start.difference(&dirty) {
            ever_clean.insert(path.clone());
        }
    };

    BenchmarkRun {
        screen: format!("{screen:?}"),
        iterations,
        converged,
        regressions,
        remaining_problems: report.problems(),
    }
}

/// Every contract path `compare` can emit for a screen, obtained by comparing
/// the reference against a copy of itself with every comparable value moved.
fn comparable_paths(screen: Screen) -> Vec<String> {
    let reference = reference_contract(screen);
    let perturb = |value: f32| value + 1.5;
    let probe = DesignContract {
        version: reference.version,
        name: reference.name.clone(),
        dimensions: floom_dsl::ui_ir::Dimensions {
            width: perturb(reference.dimensions.width),
            height: perturb(reference.dimensions.height),
        },
        spacing: floom_dsl::ui_ir::Spacing {
            gap: perturb(reference.spacing.gap),
            padding: reference.spacing.padding.map(perturb),
        },
        radius: floom_dsl::ui_ir::Radius {
            corner: perturb(reference.radius.corner),
        },
        typography: reference
            .typography
            .iter()
            .map(|style| floom_dsl::ui_ir::TypeStyle {
                name: style.name.clone(),
                font_size: perturb(style.font_size),
                line_height: style.line_height.map(perturb),
                weight: style
                    .weight
                    .clone()
                    .map(|w| format!("{w}zz"))
                    .or_else(|| Some("perturbed".to_string())),
            })
            .collect(),
        colors: reference
            .colors
            .iter()
            .map(|role| floom_dsl::ui_ir::ColorRole {
                name: role.name.clone(),
                value: floom_dsl::ui_ir::Rgba::new(
                    role.value.r.saturating_add(7),
                    role.value.g.saturating_add(7),
                    role.value.b.saturating_add(7),
                    role.value.a,
                ),
            })
            .collect(),
        states: reference.states.clone(),
        animation: floom_dsl::ui_ir::Animation {
            duration_ms: reference.animation.duration_ms + 1,
            easing: format!("{}x", reference.animation.easing),
        },
        accessibility: floom_dsl::ui_ir::Accessibility {
            role: format!("{}x", reference.accessibility.role),
            label: format!("{}x", reference.accessibility.label),
            min_contrast: reference.accessibility.min_contrast + 1.0,
            min_touch_target: perturb(reference.accessibility.min_touch_target),
        },
    };
    floom_dsl::ui_ir::compare(&reference, &probe)
        .entries
        .into_iter()
        .map(|entry| entry.path)
        .collect()
}

/// The policy the benchmark exists to measure: it reads each disagreement the
/// evaluation reports and proposes the token edit that repairs it.
#[derive(Debug, Default)]
pub struct DiffDrivenPolicy;

impl DiffDrivenPolicy {
    pub fn new() -> Self {
        Self
    }
}

impl RepairPolicy for DiffDrivenPolicy {
    fn propose(&mut self, report: &EvaluationReport) -> Vec<TokenEdit> {
        let mut edits: Vec<TokenEdit> = report
            .diff
            .entries
            .iter()
            .filter_map(TokenEdit::from_diff_entry)
            .collect();
        // A floor failure with a numeric repair maps too: the touch-target
        // floor names the height the action must reach.
        for floor in &report.floors {
            if floor.check.contains("touch target")
                && let Ok(height) = floor.floor.parse::<f64>() {
                    edits.push(TokenEdit::ControlHeight(height as f32));
                }
        }
        edits
    }
}
