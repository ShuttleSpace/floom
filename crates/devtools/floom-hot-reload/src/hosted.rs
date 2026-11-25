// SPDX-License-Identifier: MIT
use floom_core::{Element, ElementId, ElementKind, HostedKind, HostedNode, LayoutRequest};
use floom_dsl::ui_ir::{NodeKey, NodeKind};
use floom_style::StyleProps;

use crate::LiveTree;

/// Registers a tree the *host* already built into a live tree.
///
/// A `WidgetHost` keeps its own node table — that is what it lays out and paints
/// — and a `LiveTree` is what the inspector reads. The two are different data
/// structures built for different jobs, so a host that wants to be inspectable
/// has to describe its own nodes into one.
///
/// This is that description, one node at a time. The host walks its own tree and
/// calls `push` in document order, naming each node's key, kind and parent; the
/// live tree then answers `id_of`, `children`, `path_to_root`, `elements` and
/// the style read exactly as it does for a tree mounted from a DSL document.
///
/// It exists as a separate builder rather than a `LiveTree` method so the
/// invariant is enforced by the type: a hosted tree is built by pushing, never
/// by mixing a `mount` of a document into a host's own nodes.
#[derive(Debug, Default)]
pub struct HostedTree {
    live: LiveTree,
}

impl HostedTree {
    pub fn new() -> Self {
        Self {
            live: LiveTree::new(),
        }
    }

    /// Adds one node, returning the id it was registered under.
    ///
    /// `parent` is the id an earlier `push` returned; `None` for the root. Keys
    /// must be unique — a host that reuses a key would make `id_of` ambiguous,
    /// and the inspector's answers would depend on insertion order.
    pub fn push(
        &mut self,
        key: impl Into<NodeKey>,
        kind: ElementKind,
        parent: Option<ElementId>,
    ) -> ElementId {
        let key = key.into();
        let node_kind = hosted_kind(kind);
        let id = self
            .live
            .arena
            .insert(Element::new(ElementId::INVALID, kind));
        if let Some(parent) = parent {
            self.live
                .tree
                .append_child(&mut self.live.arena, parent, id);
        }
        self.live.register(key, node_kind, id);
        // A hosted node carries no props: the host's own layout is the truth,
        // and the live tree's copy of it would only ever be a stale guess. The
        // style read therefore answers `None` for a hosted node, which is
        // honest — see `LiveTree::set_hosted_style` for the opt-in.
        id
    }

    /// Attaches a resolved style to a hosted node, so the style readout has
    /// something to say.
    ///
    /// Optional because a host may not have a style table in the inspector's
    /// sense; when it does, this is how it publishes it.
    pub fn set_style(&mut self, id: ElementId, style: StyleProps) {
        self.live.set_style(id, style);
    }

    /// Attaches the layout request a hosted node occupies, so the layout readout
    /// has something to say.
    ///
    /// The host's own request, not a re-derivation: a widget's
    /// [`Widget::layout_request`] describes what it *wants*, while the request
    /// the node actually got is what it *is*, and the two differ for every
    /// child a parent sized.
    ///
    /// [`Widget::layout_request`]: floom_core::Widget::layout_request
    pub fn set_layout(&mut self, id: ElementId, layout: LayoutRequest) {
        self.live.set_layout(id, layout);
    }

    /// The box the node was laid out in. Separate from `set_layout` because a
    /// request and a result are different claims.
    pub fn set_bounds(&mut self, id: ElementId, bounds: floom_types::Rect) {
        self.live.set_bounds(id, bounds);
    }

    /// The live tree, ready for an `Inspector`.
    pub fn finish(self) -> LiveTree {
        self.live
    }
}

/// The IR kind a hosted element mounts as.
///
/// The inspector's three tiers come back out as the IR kind that seats them:
/// `Stack` for a host, `Text` for a primitive, and `Container` for a
/// composition. The mapping is lossy on purpose — a live tree exists to answer
/// structural questions, and the IR's own kind vocabulary is not the host's.
fn hosted_kind(kind: ElementKind) -> NodeKind {
    match kind {
        ElementKind::Host => NodeKind::Stack,
        ElementKind::Primitive => NodeKind::Text,
        ElementKind::Composition => NodeKind::Container,
    }
}

/// Describes a host's own nodes into a live tree.
///
/// The host hands over plain data — [`HostedNode`]s, in document order — because
/// it cannot build a live tree itself: the host sits at L7 and this crate at L8,
/// and a host depending upward would be a forbidden edge. The host says what its
/// nodes *are*; this says how an inspector reads them.
///
/// The host's own parent link is an element id, which is not a position in this
/// slice, so each node's key is resolved through a table built as the nodes are
/// described. A node whose parent is not in the slice — the host named an id it
/// did not describe — is dropped rather than attached to the wrong parent: a
/// tree with one wrong edge is worse than a tree missing one node.
pub fn describe(nodes: &[HostedNode]) -> LiveTree {
    let mut builder = HostedTree::new();
    // key -> the live id it was described as.
    let mut ids: std::collections::HashMap<&str, ElementId> = std::collections::HashMap::new();
    // The host's element id -> the key that described it, so a parent link can
    // be resolved through the host's own ids rather than the live ones.
    let mut host_ids: std::collections::HashMap<u64, String> = std::collections::HashMap::new();

    for node in nodes {
        let kind = match node.kind {
            HostedKind::Host => ElementKind::Host,
            HostedKind::Primitive => ElementKind::Primitive,
            HostedKind::Composition => ElementKind::Composition,
        };
        // A node that names a parent the slice does not contain is dropped
        // rather than promoted to a root: a tree with one wrong edge is worse
        // than a tree missing one node, and a silently re-rooted orphan is the
        // wrong edge.
        //
        // The host's parent link is the *host's* element id, which is not the
        // live id this builder minted, so the link is resolved through the
        // host's own id -> key table first and only then to the live id.
        let parent = match node.parent {
            None => None,
            Some(p) => match host_ids
                .get(&p.raw())
                .and_then(|k| ids.get(k.as_str()).copied())
            {
                Some(id) => Some(id),
                None => continue,
            },
        };
        let id = builder.push(node.key.as_str(), kind, parent);
        ids.insert(node.key.as_str(), id);
        // Keyed by the *host's* element id, which is what a child's `parent`
        // field names — not by the live id this builder just minted.
        host_ids.insert(node.element.raw(), node.key.clone());
        // The two readouts' data. Both are the host's own answer, carried
        // across as plain data and written straight into the tables a
        // document-mounted tree would have filled from IR props — no
        // re-derivation here, because anything this layer invented would be a
        // second opinion about a box only the host can see.
        if let Some(style) = node.style {
            builder.set_style(id, style_props(style));
        }
        if let Some(layout) = node.layout {
            builder.set_layout(id, layout);
        }
        if let Some(bounds) = node.bounds {
            builder.set_bounds(id, bounds);
        }
    }
    builder.finish()
}

/// [`PaintStyle`] as the style layer's property bag.
///
/// A field-for-field copy, and the copy is the point: `floom-core` cannot name
/// [`StyleProps`] (it sits below the style layer), so the bridge is here. The
/// one lossy word is `elevation`, which both sides already use for "how far the
/// shadow reaches" — see [`PaintStyle`]'s module docs for what it drops.
fn style_props(style: floom_core::PaintStyle) -> StyleProps {
    StyleProps {
        background: style.background,
        foreground: style.foreground,
        opacity: style.opacity,
        font_size: style.font_size,
        padding: style.padding,
        radius: style.radius,
        border_width: style.border_width,
        border_color: style.border_color,
        elevation: style.elevation,
    }
}
