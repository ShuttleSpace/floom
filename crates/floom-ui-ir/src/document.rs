//! The document: load, validate, serialize.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::error::IrError;
use crate::node::{EventName, NodeKey, NodeKind, UiNode};
use crate::schema::{emits, prop_is_declared, prop_spec};
use crate::value::{Binding, BindingKind};
use crate::version::{IrVersion, migrate};

/// A whole UI, as authored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiDocument {
    pub version: IrVersion,
    pub root: NodeKey,
    pub bindings: Vec<Binding>,
    pub nodes: Vec<UiNode>,
}

/// A loaded document plus what had to be done to read it.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadOutcome {
    pub document: UiDocument,
    /// The version as written, before any migration.
    pub source_version: IrVersion,
    /// The `from_minor` of each migration step applied, in order.
    pub applied: Vec<u32>,
}

impl UiDocument {
    /// Parse and gate the version, but do **not** validate.
    ///
    /// Two phases on purpose: a tool that inspects, formats or repairs a broken
    /// document needs to see the parsed shape rather than only an error.
    pub fn from_json(json: &str) -> Result<LoadOutcome, IrError> {
        let doc: UiDocument =
            serde_json::from_str(json).map_err(|e| IrError::Json(e.to_string()))?;
        let source_version = doc.version;
        IrVersion::CURRENT.check(source_version)?;
        let mut doc = doc;
        let applied = migrate(&mut doc, source_version)?;
        Ok(LoadOutcome {
            document: doc,
            source_version,
            applied,
        })
    }

    /// Parse, gate the version, migrate, and validate. What a consumer wants.
    pub fn load_json(json: &str) -> Result<LoadOutcome, IrError> {
        let outcome = Self::from_json(json)?;
        outcome.document.validate()?;
        Ok(outcome)
    }

    /// Canonical JSON, with a trailing newline so the bytes are diff-friendly.
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).expect("the IR always serializes");
        text.push('\n');
        text
    }

    pub fn node(&self, key: &str) -> Option<&UiNode> {
        self.nodes.iter().find(|n| n.key.as_str() == key)
    }

    pub fn binding(&self, name: &str) -> Option<&Binding> {
        self.bindings.iter().find(|b| b.name == name)
    }

    /// Every rule the IR enforces, in the order that gives the most useful
    /// error: shape before semantics, and each check's precondition before it.
    ///
    /// The order is load-bearing where two rules could both fire. A node that
    /// is half of a two-node cycle is *also* unreachable from the root; saying
    /// "cycle" is what an author can act on, so cycles are reported first.
    pub fn validate(&self) -> Result<(), IrError> {
        // 1. Keys: legal, and unique.
        let mut index: HashMap<&str, usize> = HashMap::with_capacity(self.nodes.len());
        for (i, node) in self.nodes.iter().enumerate() {
            if !NodeKey::is_valid(node.key.as_str()) {
                return Err(IrError::BadKey {
                    key: node.key.to_string(),
                });
            }
            if index.insert(node.key.as_str(), i).is_some() {
                return Err(IrError::DuplicateKey {
                    key: node.key.to_string(),
                });
            }
        }

        // 2. The root must be one of them.
        if !index.contains_key(self.root.as_str()) {
            return Err(IrError::MissingRoot {
                root: self.root.to_string(),
            });
        }

        // 3. Binding names are unique before anything resolves against them.
        let mut bindings: HashMap<&str, &Binding> = HashMap::with_capacity(self.bindings.len());
        for binding in &self.bindings {
            if bindings.insert(binding.name.as_str(), binding).is_some() {
                return Err(IrError::DuplicateBinding {
                    name: binding.name.clone(),
                });
            }
        }

        // 4. Per-node properties and events.
        self.validate_props_and_events()?;

        // 5. Slots: children exist, and each child has one parent.
        let mut parent: HashMap<&str, &str> = HashMap::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let mut slot_names: HashSet<&str> = HashSet::new();
            for slot in &node.slots {
                if !slot_names.insert(slot.name.as_str()) {
                    return Err(IrError::DuplicateSlot {
                        key: node.key.to_string(),
                        slot: slot.name.clone(),
                    });
                }
                for child in &slot.children {
                    if !index.contains_key(child.as_str()) {
                        return Err(IrError::UnknownChild {
                            parent: node.key.to_string(),
                            child: child.to_string(),
                        });
                    }
                    if let Some(first) = parent.insert(child.as_str(), node.key.as_str()) {
                        return Err(IrError::TwoParents {
                            child: child.to_string(),
                            first: first.to_string(),
                            second: node.key.to_string(),
                        });
                    }
                }
            }
        }

        // 6. Cycles. Every node has at most one parent by now, so a cycle is a
        //    parent chain that returns to where it started.
        for node in &self.nodes {
            let mut seen: HashSet<String> = HashSet::new();
            let mut current = Some(node.key.as_str());
            while let Some(key) = current {
                if !seen.insert(key.to_string()) {
                    return Err(IrError::Cycle {
                        at: key.to_string(),
                    });
                }
                current = parent.get(key).copied();
            }
        }

        // 7. Reachability from the root. A node nothing claims is a node the
        //    document pretends to have but can never render.
        let mut reachable: HashSet<&str> = HashSet::new();
        let mut stack = vec![self.root.as_str()];
        while let Some(key) = stack.pop() {
            if !reachable.insert(key) {
                continue;
            }
            if let Some(i) = index.get(key) {
                stack.extend(self.nodes[*i].children().map(|c| c.as_str()));
            }
        }
        if let Some(node) = self
            .nodes
            .iter()
            .find(|n| !reachable.contains(n.key.as_str()))
        {
            return Err(IrError::Unreachable {
                key: node.key.to_string(),
            });
        }

        // 8. References resolve, and resolve to the right namespace.
        self.resolve_bindings(&bindings)?;

        Ok(())
    }

    fn validate_props_and_events(&self) -> Result<(), IrError> {
        for node in &self.nodes {
            let mut names: HashSet<&str> = HashSet::new();
            for prop in &node.props {
                if !names.insert(prop.name.as_str()) {
                    return Err(IrError::DuplicateProp {
                        key: node.key.to_string(),
                        prop: prop.name.clone(),
                    });
                }
                let Some(spec) = prop_spec(node.kind, &prop.name) else {
                    // "You spelled it wrong" and "that is not a Button
                    // property" want different fixes, so they are different
                    // errors.
                    return Err(if prop_is_declared(&prop.name) {
                        IrError::PropNotAllowed {
                            key: node.key.to_string(),
                            kind: node.kind,
                            prop: prop.name.clone(),
                        }
                    } else {
                        IrError::UnknownProp {
                            key: node.key.to_string(),
                            prop: prop.name.clone(),
                        }
                    });
                };
                // A reference is checked against its binding's type later; only
                // literals are type-checked here.
                if let Some(ty) = prop.value.value_type() {
                    if ty != spec.ty {
                        return Err(IrError::WrongValueType {
                            key: node.key.to_string(),
                            prop: prop.name.clone(),
                            expected: spec.ty,
                            found: prop.value.kind_name(),
                        });
                    }
                    if !spec.domain.is_empty()
                        && let Some(ident) = prop.value.as_ident()
                        && !spec.domain.contains(&ident)
                    {
                        return Err(IrError::ValueOutsideDomain {
                            key: node.key.to_string(),
                            prop: prop.name.clone(),
                            value: ident.to_string(),
                        });
                    }
                }
            }

            let mut events: HashSet<EventName> = HashSet::new();
            for binding in &node.events {
                if !events.insert(binding.event) {
                    return Err(IrError::DuplicateEvent {
                        key: node.key.to_string(),
                        event: binding.event,
                    });
                }
                if !emits(node.kind, binding.event) {
                    return Err(IrError::EventNotAllowed {
                        key: node.key.to_string(),
                        kind: node.kind,
                        event: binding.event,
                    });
                }
            }
        }
        Ok(())
    }

    fn resolve_bindings(&self, bindings: &HashMap<&str, &Binding>) -> Result<(), IrError> {
        for node in &self.nodes {
            for prop in &node.props {
                let Some(name) = prop.value.as_ref_name() else {
                    continue;
                };
                let Some(binding) = bindings.get(name) else {
                    return Err(IrError::UnresolvedBinding {
                        key: node.key.to_string(),
                        name: name.to_string(),
                    });
                };
                let Some(spec) = prop_spec(node.kind, &prop.name) else {
                    continue; // already rejected above
                };
                match binding.kind {
                    BindingKind::Handler => {
                        return Err(IrError::HandlerUsedAsValue {
                            name: name.to_string(),
                        });
                    }
                    BindingKind::Value(ty) if ty != spec.ty => {
                        return Err(IrError::BindingTypeMismatch {
                            name: name.to_string(),
                            expected: spec.ty,
                            found: ty,
                        });
                    }
                    BindingKind::Value(_) => {}
                }
            }

            for event in &node.events {
                let Some(binding) = bindings.get(event.handler.as_str()) else {
                    return Err(IrError::UnresolvedBinding {
                        key: node.key.to_string(),
                        name: event.handler.clone(),
                    });
                };
                if let BindingKind::Value(_) = binding.kind {
                    return Err(IrError::ValueUsedAsHandler {
                        name: event.handler.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Every node kind used by this document.
    pub fn kinds(&self) -> Vec<NodeKind> {
        let mut out: Vec<NodeKind> = self.nodes.iter().map(|n| n.kind).collect();
        out.sort_by_key(|k| format!("{k:?}"));
        out.dedup();
        out
    }
}
