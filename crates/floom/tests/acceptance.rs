//! Phase 31 — the P0–P4 acceptance matrix is evidence, and evidence rots.
//!
//! `docs/superpowers/plans/2026-09-02-…plan.md` judges every P0–P4 line against
//! a named test target, bench or observed run. That is the right shape — "a
//! crate existing is not evidence" — but a path written into a document is a
//! claim about the tree, and by 2026-09-19 the matrix had already decayed: four
//! P3 lines still read "BLOCKED — Phase 22 / 25 / 26 / 27 unstarted" for phases
//! that had shipped weeks earlier.
//!
//! This suite makes the matrix check itself:
//!
//! * every path it names as evidence must exist on disk;
//! * a ticked line must name evidence (or be an explicit waiver);
//! * P0 is a floor — nothing there may be unticked;
//! * the section must carry a dated acceptance run, so "we judged this" cannot
//!   quietly become "somebody once wrote this".
//!
//! It does **not** re-run the named targets; `cargo test --workspace` already
//! does that. What it adds is the part no test run covers: the document and the
//! tree agreeing about what the evidence even is.

use std::path::{Path, PathBuf};

const PLAN: &str = "docs/superpowers/plans/2026-09-02-floom-2-complete-implementation-plan.md";

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/crates/floom
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plan_text() -> String {
    let path = repo_root().join(PLAN);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The Phase 31 section: from its heading to the next `# Phase` heading.
///
/// Bounded before the Appendix on purpose. The Appendix is a retroactive record
/// of work done outside the plan, and its entries legitimately name no file
/// ("runner routes preedit to the focused widget"); the matrix proper is the
/// thing that claims to be judged against named evidence.
fn acceptance_section() -> String {
    let text = plan_text();
    let start = text
        .find("# Phase 31")
        .unwrap_or_else(|| panic!("{PLAN} has no Phase 31 section"));
    let rest = &text[start..];
    let end = rest.find("\n# Appendix").unwrap_or(rest.len());
    rest[..end].to_string()
}

/// Every acceptance line: (ticked, body).
fn lines() -> Vec<(bool, String)> {
    acceptance_section()
        .lines()
        .filter_map(|l| {
            let t = l.trim_start();
            if let Some(body) = t.strip_prefix("- [x] ") {
                Some((true, body.to_string()))
            } else {
                t.strip_prefix("- [ ] ")
                    .map(|body| (false, body.to_string()))
            }
        })
        .collect()
}

/// Backticked tokens in a line.
fn tokens(line: &str) -> Vec<String> {
    line.split('`')
        .skip(1)
        .step_by(2)
        .map(|t| t.trim().to_string())
        .collect()
}

/// Extensions that mark a token as naming a file.
const EXTENSIONS: &[&str] = &[".rs", ".md", ".sh", ".yml", ".yaml", ".toml", ".json"];

/// Top-level directories a crate can live in. A slash-bearing token only counts
/// as a path if it starts with one of these — otherwise a bench id like
/// `update_reuse/scroll_100k_recycle` would be read as a directory.
const LIB_DIRS: &[&str] = &[
    "foundation",
    "runtime",
    "layout",
    "scene",
    "render",
    "platform",
    "native",
    "framework",
    "widgets",
    "app",
    "tooling",
    "compiler",
    "design-systems",
    "examples",
    "benches",
    "docs",
    "scripts",
    "src",
    "legacy",
    "vendor",
    "ecosystem",
    ".github",
    ".superpowers",
];

/// Does this token claim to name something in the tree?
fn looks_like_path(token: &str) -> bool {
    if token.contains('*') || token.contains('{') {
        return false;
    }
    EXTENSIONS.iter().any(|e| token.ends_with(e))
        || (token.contains('/') && LIB_DIRS.iter().any(|d| token.starts_with(d)))
}

/// Resolves the evidence tokens of one line, in order.
///
/// The matrix writes bare filenames relative to whatever it named earlier in
/// the same line — `crates/floom-runtime-core/src/arena.rs` + `arena_tests.rs`,
/// or `crates/floom-platform-desktop` (… incl. `lifecycle.rs`, …). A
/// repo-root join would call all of those missing. So the resolver carries the
/// directory the line last named and looks in it (and its `src`/`tests`/
/// `benches`) for a bare filename, which is what a reader does.
struct Resolver {
    root: PathBuf,
    dir: Option<PathBuf>,
}

impl Resolver {
    fn new(root: PathBuf) -> Self {
        Self { root, dir: None }
    }

    fn resolve(&mut self, token: &str) -> Option<PathBuf> {
        if token.contains('*') || token.contains('{') {
            return None;
        }
        let direct = self.root.join(token);
        if direct.exists() {
            let dir = if direct.is_dir() {
                direct.clone()
            } else {
                direct.parent().unwrap_or(&direct).to_path_buf()
            };
            self.dir = Some(dir);
            return Some(direct);
        }
        // A bare crate name: `<lib dir>/<crate>`.
        if !token.contains('/') && !EXTENSIONS.iter().any(|e| token.ends_with(e)) {
            for lib in LIB_DIRS {
                let candidate = self.root.join(lib).join(token);
                if candidate.exists() {
                    self.dir = Some(candidate.clone());
                    return Some(candidate);
                }
            }
        }
        // A bare filename relative to the crate or directory named earlier.
        if !token.contains('/') {
            if let Some(base) = &self.dir {
                for sub in ["", "src", "tests", "benches"] {
                    let candidate = if sub.is_empty() {
                        base.join(token)
                    } else {
                        base.join(sub).join(token)
                    };
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }
}

#[test]
fn the_matrix_exists_and_has_lines() {
    let all = lines();
    assert!(
        all.len() >= 25,
        "the P0-P4 matrix should have ~27 lines, found {}",
        all.len()
    );
}

#[test]
fn every_path_the_matrix_names_as_evidence_exists() {
    let root = repo_root();
    let mut checked = 0;
    for (ticked, body) in lines() {
        let mut resolver = Resolver::new(root.clone());
        for token in tokens(&body) {
            // Resolve *every* token, in order: a bare crate name such as
            // `floom-platform-desktop` is not path-like, but it is what the
            // bare filenames later in the same line are relative to. Only the
            // path-like ones are then held to existing.
            let resolved = resolver.resolve(&token).is_some();
            if !looks_like_path(&token) {
                continue;
            }
            assert!(
                resolved,
                "the matrix names `{token}` as evidence{} but nothing in the tree matches \
                 (repo root {})",
                if ticked { " for a TICKED line" } else { "" },
                root.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 20,
        "only {checked} evidence paths checked — the parse is probably wrong"
    );
}

#[test]
fn no_ticked_line_is_evidence_free() {
    for (ticked, body) in lines() {
        if !ticked {
            continue;
        }
        // A waiver says so out loud; anything else has to point at something.
        if body.contains("WAIVED") {
            continue;
        }
        let mut resolver = Resolver::new(repo_root());
        let named: Vec<PathBuf> = tokens(&body)
            .iter()
            .filter_map(|t| resolver.resolve(t))
            .collect();
        assert!(
            !named.is_empty(),
            "a ticked acceptance line with no existing evidence target:\n  {body}"
        );
    }
}

#[test]
fn p0_is_a_floor_and_stays_ticked() {
    let section = acceptance_section();
    let p0 = section
        .split("\n## P1 Acceptance")
        .next()
        .expect("no P1 heading")
        .split("# Phase 31")
        .nth(1)
        .expect("no Phase 31 heading");
    let mut count = 0;
    for (ticked, body) in lines() {
        let head: String = body.chars().take(40).collect();
        if !p0.contains(&head) {
            continue;
        }
        count += 1;
        assert!(
            ticked,
            "a P0 line is unticked — P0 is the floor, not a backlog item:\n  {body}"
        );
    }
    assert!(count >= 9, "expected ~10 P0 lines, matched {count}");
}

#[test]
fn the_matrix_carries_a_dated_acceptance_run() {
    let section = acceptance_section();
    assert!(
        section.contains("## Acceptance run"),
        "the matrix must record the run it was judged against, not just the verdicts"
    );
    let dated = section
        .lines()
        .any(|l| l.starts_with("## Acceptance run") && l.contains("2026-09-19"));
    assert!(
        dated,
        "no acceptance run recorded for this pass — add `## Acceptance run — 2026-09-19`"
    );
}
