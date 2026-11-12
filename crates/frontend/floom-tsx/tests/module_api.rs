// SPDX-License-Identifier: MIT
//! The `.d.ts` against the API it has to cover.
//!
//! Two directions, and both matter.
//!
//! The existing parity test asks whether everything the declaration offers
//! is *accepted* — the declaration cannot promise an API the parser rejects.
//! This file asks the other question: whether everything the schema and the
//! runtime offer is *declared*. A declaration that omits a prop the schema
//! knows, or an export the runtime implements, leaves a TSX author with no
//! completion and no type for something that exists — the failure that a
//! one-way check cannot see.

use floom_dsl::ui_ir::{NativeKind, NodeKind, all_prop_names, events_for, prop_spec};
use floom_tsx::{ModuleExport, generate_dts};
use std::collections::HashSet;

/// The name a kind's props interface carries.
///
/// Native kinds use the parser's spellings (`Web`/`Video`/`Map`/`TextInput`),
/// not `NativeWeb`; the rest are `name()` converted to PascalCase.
fn ts_name(kind: NodeKind) -> String {
    match kind {
        NodeKind::Native(NativeKind::Web) => "Web".to_string(),
        NodeKind::Native(NativeKind::Video) => "Video".to_string(),
        NodeKind::Native(NativeKind::Map) => "Map".to_string(),
        NodeKind::Native(NativeKind::TextInput) => "TextInput".to_string(),
        other => other
            .name()
            .split('_')
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                    None => String::new(),
                }
            })
            .collect(),
    }
}

/// The member names declared by one `export interface` block.
fn interface_members(dts: &str, interface: &str) -> Vec<String> {
    let marker = format!("export interface {interface} ");
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut inside = false;
    for line in dts.lines() {
        let trimmed = line.trim_start();
        if !inside {
            if trimmed.starts_with(&marker) {
                inside = true;
                // The `{` is on this line, so the body starts one level in.
                depth = 1;
            }
            continue;
        }
        depth += trimmed.matches('{').count() as i32 - trimmed.matches('}').count() as i32;
        if let Some((name, _)) = trimmed.split_once(':') {
            let name = name.trim().trim_end_matches('?');
            if !name.is_empty() && name.chars().next().is_some_and(|c| c.is_alphabetic()) {
                out.push(name.to_string());
            }
        }
        if depth <= 0 {
            break;
        }
    }
    out
}

/// The names exported by the `declare module "floom"` block.
fn module_exports(dts: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in dts.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("declare module \"floom\"") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if trimmed == "}" {
            break;
        }
        if let Some(rest) = trimmed
            .strip_prefix("export function ")
            .or_else(|| trimmed.strip_prefix("export const "))
            .or_else(|| trimmed.strip_prefix("export type "))
        {
            let name = rest
                .split(['<', '(', ':', ' '])
                .next()
                .unwrap_or_default()
                .trim();
            if !name.is_empty() {
                out.push(name.to_string());
            }
        }
    }
    out
}

/// Every prop the schema accepts for a kind is declared for that kind.
#[test]
fn every_prop_the_schema_accepts_is_declared() {
    let dts = generate_dts();
    for kind in NodeKind::ALL {
        let interface = format!("{}Props", ts_name(*kind));
        let members = interface_members(&dts, &interface);
        let declared: HashSet<&str> = members.iter().map(String::as_str).collect();
        for name in all_prop_names() {
            if prop_spec(*kind, name).is_none() {
                continue;
            }
            assert!(
                declared.contains(name),
                "{interface} does not declare the prop {name}, which the schema accepts for {kind:?}"
            );
        }
    }
}

/// Every event the schema gives a kind is declared for that kind.
#[test]
fn every_event_the_schema_gives_is_declared() {
    let dts = generate_dts();
    for kind in NodeKind::ALL {
        let interface = format!("{}Props", ts_name(*kind));
        let members = interface_members(&dts, &interface);
        let declared: HashSet<&str> = members.iter().map(String::as_str).collect();
        for event in events_for(*kind) {
            let mut chars = event.name().chars();
            let handler = format!(
                "on{}{}",
                chars.next().unwrap().to_uppercase().collect::<String>(),
                chars.as_str()
            );
            assert!(
                declared.contains(handler.as_str()),
                "{interface} does not declare the handler {handler}"
            );
        }
    }
}

/// The `"floom"` module declares exactly the exports the runtime implements.
///
/// [`ModuleExport`] is the one list: the compiler lowers a TSX file against
/// it and the generator writes the declaration from it. This test is what
/// makes that a fact rather than a hope — an export added to the enum but
/// not declared, or declared but not implemented, fails here.
#[test]
fn the_module_declares_exactly_the_runtime_exports() {
    let dts = generate_dts();
    let declared = module_exports(&dts);
    let expected: Vec<String> = ModuleExport::ALL
        .iter()
        .map(|export| export.name().to_string())
        .collect();
    assert_eq!(
        declared, expected,
        "the \"floom\" module and the runtime's export list disagree"
    );
}

/// The module's exports are distinct — a name declared twice is a name the
/// editor resolves to one of two things.
#[test]
fn the_module_exports_are_distinct() {
    let dts = generate_dts();
    let declared = module_exports(&dts);
    let mut unique = declared.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        declared.len(),
        "the \"floom\" module declares a name twice"
    );
}

/// Every export the module declares is usable in a TSX file.
///
/// The declaration is only worth anything if the compiler accepts the code it
/// describes. This walks the whole surface in one source: the reactive
/// functions, the context pair, the entry call and the three surfaces.
#[test]
fn every_module_export_is_usable_in_a_tsx_file() {
    let src = r#"
        import {
          app, createEffect, createMemo, createSignal, mount, platform,
          provideContext, useContext, window
        } from "floom";
        const [count, setCount] = createSignal(0);
        const doubled = createMemo(() => count() * 2);
        provideContext("theme", "dark");
        const theme = useContext("theme");
        createEffect(() => { setCount(doubled()); });
        app.setName("every export");
        window.setTitle("every export");
        window.setSize(640, 480);
        window.setSize("640x480");
        window.setResizable(true);
        platform.require("ime", "warn");
        function App() {
          return <Stack><Text label={theme} /><Text label={doubled()} /></Stack>;
        }
        mount(App);
    "#;
    floom_tsx::compile(src).expect("every module export must be usable");
}
