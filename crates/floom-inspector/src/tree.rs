//! The element-tree snapshot.

use serde::{Deserialize, Serialize};

use crate::{JsonSnapshot, ProtocolError, ProtocolVersion, SourceLocation};

/// One element on the wire.
///
/// `id` is the stable `ElementId::raw()` — the identity the rest of the
/// protocol (styles, layout, performance) keys on. `key` is the IR `NodeKey`,
/// present when the element came from a document; `source` is where that node
/// was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNodeInfo {
    pub id: u64,
    /// `"host"`, `"primitive"` or `"composition"`.
    pub kind: String,
    pub key: Option<String>,
    pub source: Option<SourceLocation>,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
}

/// A whole element tree at a point in time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeSnapshot {
    pub version: ProtocolVersion,
    pub root: u64,
    pub nodes: Vec<TreeNodeInfo>,
}

impl JsonSnapshot for TreeSnapshot {}

impl TreeSnapshot {
    /// Reject snapshots that contradict themselves: a duplicate id, a parent
    /// or child that names nothing, two parents for one node, a cycle, or a
    /// root that is not a node.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        let current = ProtocolVersion::current();
        current
            .accepts_err(self.version)
            .map_err(|_| ProtocolError::UnsupportedVersion {
                reader: current,
                snapshot: self.version,
            })?;

        let mut ids = std::collections::HashMap::new();
        for node in &self.nodes {
            if ids.insert(node.id, node).is_some() {
                return Err(ProtocolError::InvalidTree {
                    reason: format!("duplicate id {}", node.id),
                });
            }
        }
        if !ids.contains_key(&self.root) {
            return Err(ProtocolError::MissingRoot { root: self.root });
        }
        for node in &self.nodes {
            if let Some(parent) = node.parent {
                if !ids.contains_key(&parent) {
                    return Err(ProtocolError::InvalidTree {
                        reason: format!(
                            "node {} names parent {parent}, which does not exist",
                            node.id
                        ),
                    });
                }
            }
            for child in &node.children {
                let Some(info) = ids.get(child) else {
                    return Err(ProtocolError::InvalidTree {
                        reason: format!(
                            "node {} lists child {child}, which does not exist",
                            node.id
                        ),
                    });
                };
                if info.parent != Some(node.id) {
                    return Err(ProtocolError::InvalidTree {
                        reason: format!(
                            "node {} lists child {child}, but the child's parent is {:?}",
                            node.id, info.parent
                        ),
                    });
                }
            }
        }
        // A cycle cannot survive "one parent per node, root reachable" unless
        // two nodes disagree about who is the ancestor; walk from every node
        // and cap the walk at the node count.
        for node in &self.nodes {
            let mut current_id = node.parent;
            let mut steps = 0;
            while let Some(id) = current_id {
                if id == node.id {
                    return Err(ProtocolError::InvalidTree {
                        reason: format!("nodes {} and {} are each other's parent", node.id, id),
                    });
                }
                current_id = ids.get(&id).and_then(|n| n.parent);
                steps += 1;
                if steps > self.nodes.len() {
                    return Err(ProtocolError::InvalidTree {
                        reason: format!("a parent cycle passes through {}", node.id),
                    });
                }
            }
        }
        Ok(())
    }

    /// The chain from `id` up to the root, `id` first, root last.
    pub fn path_to_root(&self, id: u64) -> Option<Vec<u64>> {
        let by_id: std::collections::HashMap<u64, &TreeNodeInfo> =
            self.nodes.iter().map(|n| (n.id, n)).collect();
        let start = by_id.get(&id)?;
        let mut path = vec![id];
        let mut current = start.parent;
        while let Some(next) = current {
            path.push(next);
            current = by_id.get(&next).and_then(|n| n.parent);
        }
        Some(path)
    }
}
