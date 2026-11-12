//! The props tables, parsed out of the library's own sources.
//!
//! A props table is only worth reading if it is the library's own truth, so
//! these tables are not written by hand. They are parsed at run time out of
//! `floom-core-theme`'s own props sources, which `include_str!` embeds in the
//! binary. A field the library adds shows up in the gallery without anyone
//! editing it, and a field the library renames cannot be left behind in a
//! hand-written table.
//!
//! The parser reads three things out of a props source:
//!
//! - the `pub` fields of the module's `…Props` struct, in the order it
//!   declares them, with their types as the source writes them;
//! - the `Default` initialiser of each field, from the `impl Default` block;
//! - the doc comment above each field, which becomes the row's description.
//!
//! A field with no doc comment keeps an empty description, and the table
//! renders an empty cell as an em dash.

use std::collections::HashMap;

use crate::api::ApiRow;
use crate::props_sources;

pub use crate::props_sources::COMPONENT_PROPS;

/// What a column says when the library does not say it.
const MISSING: &str = "—";

/// The props module that describes a component, by the component's key.
///
/// The component's key is the one the satellite names it after -- `text-field`,
/// `datatable` -- with the satellite's own spelling normalised, so the same
/// module is found whichever way the key is written.
pub fn module_for(component: &str) -> Option<&'static str> {
    let wanted = component.replace('-', "_");
    COMPONENT_PROPS
        .iter()
        .find(|(name, _)| name.replace('-', "_") == wanted)
        .map(|(_, module)| *module)
}

/// The props rows of a component, from the library's own props source.
pub fn props_for_component(component: &str) -> Vec<ApiRow> {
    module_for(component).map(props_for).unwrap_or_default()
}

/// The props rows of a props module, in the order the struct declares them.
pub fn props_for(module: &str) -> Vec<ApiRow> {
    let Some(source) = props_sources::source_for(module) else {
        return Vec::new();
    };
    let Some((name, body)) = props_struct(source) else {
        return Vec::new();
    };
    let defaults = defaults_for(source, &name);
    fields_of(body)
        .into_iter()
        .map(|(field, type_, description)| {
            let default = defaults
                .get(&field)
                .cloned()
                .unwrap_or_else(|| MISSING.to_string());
            ApiRow::new(&field, &type_, &default, &description)
        })
        .collect()
}

/// How many `pub` fields a props struct has.
///
/// This counts the fields by a path of its own -- it reads the struct's lines
/// rather than parsing them -- so a parser that quietly drops a field shows up
/// as a row count that does not match.
pub fn field_count(module: &str) -> usize {
    let Some(source) = props_sources::source_for(module) else {
        return 0;
    };
    let Some((_, body)) = props_struct(source) else {
        return 0;
    };
    body.lines().filter(|line| is_field_line(line)).count()
}

/// Whether a line of a props struct declares a field.
///
/// A field is a `pub` name, a colon, and a type. Anything else in the struct's
/// body -- a doc comment, an attribute -- is not one.
fn is_field_line(line: &str) -> bool {
    let Some(rest) = line.trim().strip_prefix("pub ") else {
        return false;
    };
    let Some((name, type_)) = rest.split_once(':') else {
        return false;
    };
    is_identifier(name.trim()) && !type_.trim().trim_end_matches(',').trim().is_empty()
}

/// Whether a string is a field name: an identifier.
fn is_identifier(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// The `…Props` struct of a props source, and the text of its body.
fn props_struct(source: &str) -> Option<(String, &str)> {
    let mut rest = source;
    while let Some(at) = rest.find("pub struct ") {
        let after = &rest[at + "pub struct ".len()..];
        let Some(brace) = after.find('{') else {
            break;
        };
        let name = after[..brace].trim();
        rest = &after[brace..];
        if !name.ends_with("Props") {
            continue;
        }
        return Some((name.to_string(), brace_body(rest)));
    }
    None
}

/// The text between the brace at the start of `source` and its match.
fn brace_body(source: &str) -> &str {
    let mut depth = 0usize;
    let mut open = 0usize;
    for (index, byte) in source.as_bytes().iter().enumerate() {
        match byte {
            b'{' => {
                if depth == 0 {
                    open = index;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[open + 1..index];
                }
            }
            _ => {}
        }
    }
    source
}

/// The `Default` initialiser of each field of a props struct.
fn defaults_for(source: &str, struct_name: &str) -> HashMap<String, String> {
    let mut defaults = HashMap::new();
    let needle = format!("impl Default for {struct_name}");
    let Some(at) = source.find(&needle) else {
        return defaults;
    };
    let rest = &source[at..];
    let Some(brace) = rest.find('{') else {
        return defaults;
    };
    let body = brace_body(&rest[brace..]);
    let Some(self_at) = body.find("Self {") else {
        return defaults;
    };
    let Some(self_brace) = body[self_at..].find('{') else {
        return defaults;
    };
    for line in brace_body(&body[self_at + self_brace..]).lines() {
        let trimmed = line.trim();
        if !trimmed.ends_with(',') {
            continue;
        }
        let Some((name, value)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if !is_identifier(name) {
            continue;
        }
        let value = value.trim();
        defaults.insert(name.to_string(), value.trim_end_matches(',').to_string());
    }
    defaults
}

/// The `pub` fields of a props struct: name, type and description.
fn fields_of(body: &str) -> Vec<(String, String, String)> {
    let lines: Vec<&str> = body.lines().collect();
    let mut fields = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("pub ") else {
            continue;
        };
        let Some((name, type_)) = rest.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if !is_identifier(name) {
            continue;
        }
        let type_ = type_.trim().trim_end_matches(',').trim();
        if type_.is_empty() {
            continue;
        }
        fields.push((
            name.to_string(),
            type_.to_string(),
            description_above(&lines, index),
        ));
    }
    fields
}

/// The doc comment above a field, if it has one.
fn description_above(lines: &[&str], index: usize) -> String {
    let mut parts: Vec<&str> = Vec::new();
    let mut cursor = index;
    while cursor > 0 {
        cursor -= 1;
        let trimmed = lines[cursor].trim();
        if let Some(comment) = trimmed.strip_prefix("///") {
            let comment = comment.trim();
            if !comment.is_empty() {
                parts.push(comment);
            }
        } else if trimmed.starts_with("#[") {
            // An attribute may sit between the doc comment and the field.
            continue;
        } else {
            break;
        }
    }
    parts.reverse();
    parts.join(" ")
}
