// SPDX-License-Identifier: MIT
//! Phase 30 — the documentation cannot drift away from the tree.
//!
//! `docs/2.0/EXAMPLES.md`, `ARCHITECTURE.md` and `RELEASE-POLICY.md` all make
//! claims that are checkable: which examples exist and which of them run
//! headless, which crate sits at which layer, which test and bench targets the
//! release gate names. A doc that lists a file that was renamed — or omits an
//! example that was added — is worse than no doc, so this test parses the three
//! pages and compares them against the repository.
//!
//! Deliberately *not* parsed: prose. Only the tables and the explicitly marked
//! lists are, because those are the parts that go stale silently.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    // The facade lives at crates/app/floom; three levels up is the repo root.
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn read(path: &str) -> String {
    let full = root().join(path);
    std::fs::read_to_string(&full)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", full.display()))
}

/// Markdown table rows, as trimmed cells (`cells[1]` is the first column).
fn rows(md: &str) -> impl Iterator<Item = Vec<String>> + '_ {
    md.lines()
        .filter(|line| line.starts_with('|'))
        .filter(|line| !line.replace(['|', '-', ' '], "").is_empty())
        .map(|line| {
            line.split('|')
                .map(|cell| cell.trim().to_string())
                .collect::<Vec<_>>()
        })
}

/// The first backtick-quoted token in a cell, ignoring any trailing prose
/// (the size table writes "`floom-window` (`--features real`)").
fn first_token(cell: &str) -> String {
    let rest = cell.trim_start_matches('`');
    let end = rest.find('`').unwrap_or(rest.len());
    rest[..end].trim().to_string()
}


// ---------------------------------------------------------------------------
// EXAMPLES.md <-> examples/ <-> scripts/run-golden-examples.sh
// ---------------------------------------------------------------------------

fn documented_examples() -> Vec<(String, String)> {
    // (name, status) for every row of the index table.
    rows(&read("docs/2.0/EXAMPLES.md"))
        .filter(|cells| cells.len() >= 5 && cells[1].starts_with('`'))
        .map(|cells| (first_token(&cells[1]), cells[4].clone()))
        .filter(|(name, _)| name != "example")
        .collect()
}

fn example_directories() -> Vec<String> {
    let dir = root().join("examples");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("examples/")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("Cargo.toml").exists())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

fn scripted_examples() -> Vec<String> {
    let script = read("scripts/run-golden-examples.sh");
    let start = script
        .find("EXAMPLES=(")
        .expect("the script must declare an EXAMPLES list");
    let body = &script[start..];
    let body = &body[..body.find(')').expect("unterminated EXAMPLES list")];
    let mut names: Vec<String> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.contains('('))
        .map(str::to_string)
        .collect();
    names.sort();
    names
}

#[test]
fn every_floom2_example_is_documented() {
    let documented: Vec<String> = documented_examples().into_iter().map(|(n, _)| n).collect();

    for name in example_directories() {
        if !name.starts_with("floom-") {
            continue; // these examples document themselves outside the 2.0 guide
        }
        assert!(
            documented.contains(&name),
            "`examples/{name}` is not in docs/2.0/EXAMPLES.md — add a row or delete the example"
        );
    }
}

#[test]
fn every_documented_example_either_exists_or_is_marked_gated() {
    let on_disk = example_directories();

    for (name, status) in documented_examples() {
        if status.contains("gated") {
            continue; // device/display/target-gated: absence is the honest state
        }
        assert!(
            on_disk.contains(&name),
            "docs/2.0/EXAMPLES.md advertises `{name}` as {status} but examples/{name} does not exist"
        );
    }
}

#[test]
fn the_headless_rows_are_exactly_the_examples_the_script_runs() {
    let mut headless: Vec<String> = documented_examples()
        .into_iter()
        .filter(|(_, status)| status.contains("headless"))
        .map(|(name, _)| name)
        .collect();
    headless.sort();

    assert_eq!(
        headless,
        scripted_examples(),
        "docs/2.0/EXAMPLES.md's headless rows and scripts/run-golden-examples.sh disagree — \
         one of them will run (or skip) an example the other does not know about"
    );
    assert!(
        !headless.is_empty(),
        "the headless list parsed empty: the table or the script changed shape"
    );
}

// ---------------------------------------------------------------------------
// ARCHITECTURE.md <-> scripts/check-architecture-deps.sh
// ---------------------------------------------------------------------------

/// `crate -> layer` as the architecture gate defines it.
fn gate_layers() -> BTreeMap<String, u8> {
    let script = read("scripts/check-architecture-deps.sh");
    let start = script
        .find("for _names, _level in [")
        .expect("the gate must declare its LAYERS table");
    let body = &script[start..];
    let body = &body[..body.find("\n]:").expect("unterminated LAYERS table")];

    let mut layers = BTreeMap::new();
    let mut pending: Vec<String> = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        for name in trimmed.split('"').skip(1).step_by(2) {
            pending.push(name.to_string());
        }
        // A level line ends with `, <digits>[)] [,]`.
        let tail = trimmed.trim_end_matches([')', ',', ' ', ']']);
        let last = tail.rsplit(',').next().unwrap_or(tail).trim();
        if let Ok(level) = last.parse::<u8>() {
            for name in pending.drain(..) {
                layers.insert(name, level);
            }
        }
    }
    assert!(
        !layers.is_empty() && pending.is_empty(),
        "LAYERS did not parse cleanly: {} crates, {} unassigned",
        layers.len(),
        pending.len()
    );
    layers
}

/// `crate -> layer` as the architecture document claims it.
fn documented_layers() -> BTreeMap<String, u8> {
    let mut layers = BTreeMap::new();
    for cells in rows(&read("docs/2.0/ARCHITECTURE.md")) {
        if cells.len() < 3 || !cells[1].starts_with('`') {
            continue;
        }
        let level: u8 = cells[2]
            .trim()
            .trim_start_matches('L')
            .parse()
            .unwrap_or_else(|_| panic!("row for {} has no `L<n>` layer", cells[1]));
        for name in cells[1].split("`, `") {
            layers.insert(name.trim_matches('`').trim().to_string(), level);
        }
    }
    assert!(
        layers.contains_key("floom") && layers.contains_key("floom-types"),
        "the layer table did not parse: {:?}",
        layers.keys().take(5).collect::<Vec<_>>()
    );
    layers
}

#[test]
fn the_layer_table_matches_the_architecture_gate() {
    let gate = gate_layers();
    let doc = documented_layers();

    let missing_in_doc: Vec<&String> = gate.keys().filter(|c| !doc.contains_key(*c)).collect();
    let extra_in_doc: Vec<&String> = doc.keys().filter(|c| !gate.contains_key(*c)).collect();

    assert!(
        missing_in_doc.is_empty(),
        "in scripts/check-architecture-deps.sh but not docs/2.0/ARCHITECTURE.md: {missing_in_doc:?}"
    );
    assert!(
        extra_in_doc.is_empty(),
        "in docs/2.0/ARCHITECTURE.md but not scripts/check-architecture-deps.sh: {extra_in_doc:?}"
    );

    let wrong_level: Vec<String> = gate
        .iter()
        .filter(|(crate_, level)| doc.get(*crate_) != Some(level))
        .map(|(crate_, level)| format!("{crate_}: gate L{} vs doc L{:?}", level, doc.get(crate_)))
        .collect();
    assert!(wrong_level.is_empty(), "layer mismatch: {wrong_level:?}");
}

// ---------------------------------------------------------------------------
// RELEASE-POLICY.md <-> the tree
// ---------------------------------------------------------------------------

#[test]
fn the_release_gate_names_only_targets_that_exist() {
    let policy = read("docs/2.0/RELEASE-POLICY.md");

    // Every path in backticks: test targets, scripts, docs.
    let mut checked = 0;
    for token in policy.split('`').skip(1).step_by(2) {
        let token = token.trim();
        if !(token.ends_with(".rs") || token.ends_with(".sh") || token.ends_with(".md")) {
            continue;
        }
        if !token.contains('/') || token.contains(' ') {
            continue;
        }
        assert!(
            root().join(token).exists(),
            "docs/2.0/RELEASE-POLICY.md names `{token}`, which does not exist"
        );
        checked += 1;
    }
    assert!(
        checked >= 10,
        "only {checked} paths were checked — the policy's shape changed, or it stopped naming targets"
    );
}

#[test]
fn the_size_table_names_examples_that_exist() {
    let policy = read("docs/2.0/RELEASE-POLICY.md");
    let on_disk = example_directories();
    let documented: Vec<String> = documented_examples().into_iter().map(|(n, _)| n).collect();

    // Only the size table — the policy has other tables whose first column is
    // also backticked (the semver-locked surfaces).
    let start = policy
        .find("| example | bytes |")
        .expect("the size table header is gone");
    let table: Vec<Vec<String>> = policy[start..]
        .lines()
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            line.split('|')
                .map(|cell| cell.trim().to_string())
                .collect::<Vec<_>>()
        })
        .collect();

    let mut checked = 0;
    for cells in table {
        if cells.len() < 3 || !cells[1].starts_with('`') {
            continue; // the header and the separator
        }
        let name = first_token(&cells[1]);
        assert!(
            on_disk.contains(&name),
            "the size table names `{name}`, which is not an example"
        );
        assert!(
            documented.contains(&name.to_string()),
            "`{name}` is in the size table but not in docs/2.0/EXAMPLES.md"
        );
        checked += 1;
    }
    assert!(checked == 3, "expected three measured sizes, got {checked}");
}

#[test]
fn the_bench_targets_named_by_the_policy_exist() {
    // The policy's bench list moved from a prose sentence to a table, and the
    // targets moved with it: there is no workspace-root `benches/` crate any
    // more, so every suite lives beside the crate it measures and is named
    // `crate::target`.
    //
    // The previous version of this test looked for the marker
    // "Bench targets at the workspace root:" and resolved each name against
    // `benches/{name}.rs`. Both halves were true when written and neither is now,
    // and a test that has quietly stopped describing the thing it guards is worse
    // than no test — so the shape is asserted explicitly below rather than
    // inferred from a sentence that may move.
    let policy = read("docs/2.0/RELEASE-POLICY.md");

    // Scoped to the benchmark section. Parsing the whole document picks up every
    // other table in it -- the release-gate matrix, the supported-target rows --
    // and asserts that each of *their* backticked cells names a bench file, which
    // is how the first version of this test ended up asking for
    // `crates/aarch64-apple-darwin/benches/...`.
    let start = policy
        .find("| crate | target | what it measures |")
        .expect("the policy must carry the bench table");
    let section = &policy[start..];
    let section = &section[..section.find("\n\n").unwrap_or(section.len())];

    let table: Vec<(String, String)> = rows(section)
        // `rows` splits on `|`, so every row has a leading and trailing empty
        // cell and the table's own columns start at index 1.
        .filter(|cells| cells.len() >= 4 && cells[1].starts_with('`') && cells[2].starts_with('`'))
        .filter_map(|cells| {
            let crate_name = cells[1].trim_matches('`').to_string();
            let target = cells[2].trim_matches('`').to_string();
            (target != "crate" && crate_name != "crate").then_some((crate_name, target))
        })
        .collect();

    assert!(
        table.len() >= 10,
        "the bench table parsed short: {table:?} — did the table shape move?"
    );

    for (crate_name, target) in &table {
        // After the P2 layering restructure crates live under
        // `crates/<layer>/<crate>`, so walk the layer subdirs.
        let mut found = None;
        if let Ok(entries) = std::fs::read_dir(root().join("crates")) {
            for entry in entries.flatten() {
                let candidate = entry
                    .path()
                    .join(crate_name)
                    .join("benches")
                    .join(format!("{target}.rs"));
                if candidate.exists() {
                    found = Some(candidate);
                    break;
                }
            }
        }
        let path = found.unwrap_or_else(|| {
            root().join(format!("crates/{crate_name}/benches/{target}.rs"))
        });
        assert!(
            path.exists(),
            "docs/2.0/RELEASE-POLICY.md names bench `{crate_name}::{target}`, \
             but {} does not exist",
            path.strip_prefix(root()).unwrap_or(&path).display()
        );
    }
}


