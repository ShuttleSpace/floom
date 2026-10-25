//! The live tree: a retained element tree plus the side tables a reload
//! patches.

use std::collections::HashMap;

use floom_core::{Align, Axis, Element, ElementKind, LayoutRequest, Length, Position};
use floom_runtime_core::{ElementArena, ElementTree};
use floom_style::{StyleMap, StyleProps, StyleSet, StyleState};
use floom_types::ElementId;
use floom_ui_ir::{EventBinding, NodeKey, NodeKind, Prop, UiDocument};

/// A mounted document: the element tree it produced and the tables keyed by
/// the stable [`NodeKey`]s that produced it.
#[derive(Debug, Default)]
pub struct LiveTree {
    pub(crate) arena: ElementArena,
    pub(crate) tree: ElementTree,
    /// Stable key → live id.
    keys: HashMap<NodeKey, ElementId>,
    /// Live id (raw) → stable key.
    owners: HashMap<u64, NodeKey>,
    /// Live id (raw) → the IR node kind it was mounted from. The retained
    /// `Element` only knows Host/Primitive/Composition; property validation
    /// needs the real kind.
    kinds: HashMap<u64, NodeKind>,
    /// Style sets keyed by raw id (`StyleMap`'s own keying).
    styles: StyleMap,
    /// Layout requests keyed by raw id.
    layouts: HashMap<u64, LayoutRequest>,
    /// Event wiring keyed by raw id.
    events: HashMap<u64, Vec<EventBinding>>,
    /// State a layer stashed on an element, keyed by raw id. Reloads must
    /// preserve it — losing a scroll offset on an unrelated edit is the bug
    /// this whole crate exists to prevent.
    stash: HashMap<u64, String>,
}

/// A read-only view of one live element for inspection.
#[derive(Debug, Clone)]
pub struct ElementRef {
    pub key: NodeKey,
    pub id: ElementId,
    pub kind: NodeKind,
    pub parent: Option<ElementId>,
    pub children: Vec<ElementId>,
}

/// What kind of element an IR node mounts as.
pub(crate) fn element_kind(kind: NodeKind) -> ElementKind {
    if matches!(kind, NodeKind::Native(_)) {
        ElementKind::Composition
    } else if kind.is_container() {
        ElementKind::Host
    } else {
        ElementKind::Primitive
    }
}

impl LiveTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mount a whole document. The document's root becomes the tree's root.
    ///
    /// Takes a validated document (the IR's `validate` guarantees the root
    /// exists and every slot child resolves).
    pub fn mount(&mut self, document: &UiDocument) {
        let by_key: HashMap<&NodeKey, &floom_ui_ir::UiNode> =
            document.nodes.iter().map(|n| (&n.key, n)).collect();
        let Some(root) = by_key.get(&document.root) else {
            return;
        };
        self.mount_node(root, None, &by_key);
    }

    fn mount_node(
        &mut self,
        node: &floom_ui_ir::UiNode,
        parent: Option<ElementId>,
        by_key: &HashMap<&NodeKey, &floom_ui_ir::UiNode>,
    ) -> ElementId {
        let id = self
            .arena
            .insert(Element::new(ElementId::INVALID, element_kind(node.kind)));
        if let Some(parent) = parent {
            self.tree.append_child(&mut self.arena, parent, id);
        }
        self.register(node.key.clone(), node.kind, id);
        self.set_props(id, node.kind, &node.props);
        self.events.insert(id.raw(), node.events.clone());
        for slot in &node.slots {
            for child_key in &slot.children {
                if let Some(child) = by_key.get(child_key) {
                    self.mount_node(child, Some(id), by_key);
                }
            }
        }
        id
    }

    pub(crate) fn register(&mut self, key: NodeKey, kind: NodeKind, id: ElementId) {
        self.owners.insert(id.raw(), key.clone());
        self.keys.insert(key, id);
        self.kinds.insert(id.raw(), kind);
    }

    pub(crate) fn unregister(&mut self, id: ElementId) {
        if let Some(key) = self.owners.remove(&id.raw()) {
            self.keys.remove(&key);
        }
        self.kinds.remove(&id.raw());
        self.styles.invalidate(id.raw());
        self.layouts.remove(&id.raw());
        self.events.remove(&id.raw());
        self.stash.remove(&id.raw());
    }

    /// Write one node's properties into the style and layout tables.
    pub(crate) fn set_props(&mut self, id: ElementId, kind: NodeKind, props: &[Prop]) {
        let (style, layout) = seat_props(kind, props);
        let mut set = StyleSet::new();
        set.set(StyleState::NORMAL, style);
        self.styles.insert(id.raw(), set);
        self.layouts.insert(id.raw(), layout);
    }

    pub(crate) fn set_style(&mut self, id: ElementId, style: StyleProps) {
        let mut set = StyleSet::new();
        set.set(StyleState::NORMAL, style);
        self.styles.insert(id.raw(), set);
    }

    /// Records a hosted node's layout request, so the layout readout can answer
    /// for a node that has no document behind it.
    ///
    /// The same table a document-mounted tree fills from IR props: the host's
    /// own request is the truth about the box, and the inspector only needs to
    /// read it.
    pub(crate) fn set_layout(&mut self, id: ElementId, layout: LayoutRequest) {
        self.layouts.insert(id.raw(), layout);
    }

    // --- reads, for tests and for a host that wants to reconcile ----------

    /// The live id a stable key currently owns.
    pub fn id_of(&self, key: &NodeKey) -> Option<ElementId> {
        self.keys.get(key).copied()
    }

    /// The stable key that owns a live id, if any.
    pub fn key_of(&self, id: ElementId) -> Option<&NodeKey> {
        self.owners.get(&id.raw())
    }

    /// The resolved style of a key at a given interaction state.
    pub fn style_of(&self, key: &NodeKey, state: StyleState) -> Option<StyleProps> {
        let id = self.id_of(key)?;
        Some(self.styles.resolve(id.raw(), state))
    }

    /// The layout request recorded for a key.
    pub fn layout_of(&self, key: &NodeKey) -> Option<&LayoutRequest> {
        let id = self.id_of(key)?;
        self.layouts.get(&id.raw())
    }

    /// The event wiring recorded for a key.
    pub fn events_of(&self, key: &NodeKey) -> &[EventBinding] {
        match self.id_of(key) {
            Some(id) => self
                .events
                .get(&id.raw())
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            None => &[],
        }
    }

    /// Stash a piece of state on an element (scroll offset, focus, …).
    pub fn set_stash(&mut self, key: &NodeKey, value: impl Into<String>) -> bool {
        match self.id_of(key) {
            Some(id) => {
                self.stash.insert(id.raw(), value.into());
                true
            }
            None => false,
        }
    }

    /// Read back stashed state.
    pub fn stash_of(&self, key: &NodeKey) -> Option<&str> {
        let id = self.id_of(key)?;
        self.stash.get(&id.raw()).map(|s| s.as_str())
    }

    /// How many elements are live.
    pub fn element_count(&self) -> usize {
        self.arena.len()
    }

    /// Every live element, sorted by stable key so the order is reproducible.
    pub fn elements(&self) -> Vec<ElementRef> {
        let mut keys: Vec<&NodeKey> = self.keys.keys().collect();
        keys.sort();
        keys.iter()
            .filter_map(|key| {
                let id = *self.keys.get(*key)?;
                let element = self.arena.get(id)?;
                Some(ElementRef {
                    key: (*key).clone(),
                    id,
                    kind: *self.kinds.get(&id.raw())?,
                    parent: element.parent,
                    children: self.tree.children(id).to_vec(),
                })
            })
            .collect()
    }

    pub(crate) fn layouts_mut(&mut self) -> &mut HashMap<u64, LayoutRequest> {
        &mut self.layouts
    }

    pub(crate) fn events_mut(&mut self) -> &mut HashMap<u64, Vec<EventBinding>> {
        &mut self.events
    }

    /// A structural fingerprint of everything the tree holds: every key with
    /// its id, kind, parent, child order and each side table. A rollback is
    /// only *proven* by comparing this before and after a failed patch —
    /// "apply returned an error" alone does not say whether it left debris.
    pub fn snapshot(&self) -> Snapshot {
        // Sorted by key: iteration order of the map must not leak into the
        // fingerprint.
        let mut keys: Vec<&NodeKey> = self.keys.keys().collect();
        keys.sort();
        let mut out = String::new();
        for key in keys {
            let Some(id) = self.keys.get(key) else {
                continue;
            };
            let id = *id;
            let parent = self
                .arena
                .get(id)
                .and_then(|e| e.parent)
                .and_then(|p| self.owners.get(&p.raw()).map(|k| k.as_str()));
            let children: Vec<&str> = self
                .tree
                .children(id)
                .iter()
                .filter_map(|c| self.owners.get(&c.raw()).map(|k| k.as_str()))
                .collect();
            out.push_str(&format!(
                "{} #{:x} kind={:?} parent={parent:?} children={children:?}\n",
                key.as_str(),
                id.raw(),
                self.kinds.get(&id.raw()),
            ));
            if let Some(layout) = self.layouts.get(&id.raw()) {
                out.push_str(&format!("  layout={layout:?}\n"));
            }
            if let Some(events) = self.events.get(&id.raw()) {
                out.push_str(&format!("  events={events:?}\n"));
            }
            if let Some(stash) = self.stash.get(&id.raw()) {
                out.push_str(&format!("  stash={stash:?}\n"));
            }
            let props = self.styles.resolve(id.raw(), StyleState::NORMAL);
            out.push_str(&format!("  style={props:?}\n"));
        }
        Snapshot(out)
    }
}

/// Structural fingerprint of a [`LiveTree`]; equality is the rollback proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot(String);

/// Everything needed to put the tree back exactly as it was.
pub(crate) struct Checkpoint {
    /// One entry per live element: the element (carrying its id and parent)
    /// with its child list in order.
    entries: Vec<(Element, Vec<ElementId>)>,
    keys: HashMap<NodeKey, ElementId>,
    owners: HashMap<u64, NodeKey>,
    kinds: HashMap<u64, NodeKind>,
    styles: HashMap<u64, StyleSet>,
    layouts: HashMap<u64, LayoutRequest>,
    events: HashMap<u64, Vec<EventBinding>>,
    stash: HashMap<u64, String>,
}

impl LiveTree {
    /// Capture the whole tree. The arena itself is not cloneable by design —
    /// its id allocator is — so the checkpoint walks the live set through the
    /// `owners` table and rebuilds via [`ElementArena::restore`], which puts
    /// each element back at its own id.
    pub(crate) fn checkpoint(&self) -> Checkpoint {
        let entries = self
            .owners
            .values()
            .filter_map(|key| {
                let id = self.keys.get(key)?;
                let element = *self.arena.get(*id)?;
                Some((element, self.tree.children(*id).to_vec()))
            })
            .collect();
        Checkpoint {
            entries,
            keys: self.keys.clone(),
            owners: self.owners.clone(),
            kinds: self.kinds.clone(),
            styles: self.styles.entries.clone(),
            layouts: self.layouts.clone(),
            events: self.events.clone(),
            stash: self.stash.clone(),
        }
    }

    /// Put everything back, in place.
    ///
    /// The arena is *not* rebuilt from scratch: a fresh allocator has no slot
    /// state, so the checkpointed ids could not be restored into it. Instead
    /// the patch's insertions are removed, the checkpoint's freed ids are
    /// restored, and the tree links are re-established from the checkpoint.
    /// Ids are preserved: `restore` refuses to resurrect a superseded
    /// generation, so this only succeeds under the rollback discipline.
    pub(crate) fn restore(&mut self, checkpoint: Checkpoint) {
        // 1. Elements the patch mounted: live now, absent from the checkpoint.
        //    Their ids may even reuse indexes the patch itself freed.
        let stray: Vec<ElementId> = self
            .owners
            .iter()
            .filter(|(raw, _)| !checkpoint.owners.contains_key(*raw))
            .map(|(raw, _)| {
                let raw = *raw;
                ElementId::new(raw as u32, (raw >> 32) as u32)
            })
            .collect();
        for id in &stray {
            self.tree.clear_children(&mut self.arena, *id);
            self.unregister(*id);
            assert!(self.arena.remove(*id).is_some());
        }

        // 2. Drop every link held by a checkpointed node, so the re-append
        //    below starts from clean lists (and loses any link to a stray).
        for (element, _) in &checkpoint.entries {
            self.tree.clear_children(&mut self.arena, element.id);
        }

        // 3. Freed checkpointed ids come back at their own generation.
        for (element, _) in &checkpoint.entries {
            if self.arena.get(element.id).is_none() {
                assert!(
                    self.arena.restore(*element),
                    "a checkpointed element must be restorable under the \
                     rollback discipline (no generation superseded outside \
                     the failed patch)"
                );
            }
        }

        // 4. Links: both ends are live now, append in the recorded order.
        for (element, children) in &checkpoint.entries {
            for child in children {
                self.tree.append_child(&mut self.arena, element.id, *child);
            }
        }

        // 5. Side tables and the key maps, back to the checkpoint verbatim.
        self.keys = checkpoint.keys;
        self.owners = checkpoint.owners;
        self.kinds = checkpoint.kinds;
        self.styles = StyleMap {
            entries: checkpoint.styles,
            invalidate_count: 0,
        };
        self.layouts = checkpoint.layouts;
        self.events = checkpoint.events;
        self.stash = checkpoint.stash;
    }
}

fn set_style_prop(props: &mut StyleProps, prop: &Prop) {
    use floom_ui_ir::PropValue;
    match (prop.name.as_str(), &prop.value) {
        ("background", PropValue::Color(c)) => props.background = Some((*c).into()),
        ("foreground", PropValue::Color(c)) => props.foreground = Some((*c).into()),
        ("font_size", PropValue::Number(n)) => props.font_size = Some(*n as f32),
        ("opacity", PropValue::Number(n)) => props.opacity = Some(*n as f32),
        ("padding", PropValue::Number(n)) => props.padding = Some(*n as f32),
        ("radius", PropValue::Number(n)) => props.radius = Some(*n as f32),
        ("border_width", PropValue::Number(n)) => props.border_width = Some(*n as f32),
        ("border_color", PropValue::Color(c)) => props.border_color = Some((*c).into()),
        ("elevation", PropValue::Number(n)) => props.elevation = Some(*n as f32),
        _ => {}
    }
}

fn set_layout_prop(layout: &mut LayoutRequest, prop: &Prop) {
    use floom_ui_ir::PropValue;
    match (prop.name.as_str(), &prop.value) {
        ("width", PropValue::Number(n)) => layout.width = Some(Length::Px(*n as f32)),
        ("height", PropValue::Number(n)) => layout.height = Some(Length::Px(*n as f32)),
        ("grow", PropValue::Number(n)) => layout.flex_grow = Some(*n as f32),
        ("position", PropValue::Ident(word)) => {
            layout.position = Some(if word == "absolute" {
                Position::Absolute
            } else {
                Position::Relative
            });
        }
        ("inset_left", PropValue::Number(n)) => set_inset(layout, 0, Some(*n as f32)),
        ("inset_top", PropValue::Number(n)) => set_inset(layout, 1, Some(*n as f32)),
        ("inset_right", PropValue::Number(n)) => set_inset(layout, 2, Some(*n as f32)),
        ("inset_bottom", PropValue::Number(n)) => set_inset(layout, 3, Some(*n as f32)),
        // Flow intent. These used to sit in the IR's Widget class and were
        // silently dropped here; they are the LayoutRequest vocabulary.
        ("axis", PropValue::Ident(word)) => {
            layout.axis = Some(if word == "horizontal" {
                Axis::Horizontal
            } else {
                Axis::Vertical
            });
        }
        ("gap", PropValue::Number(n)) => layout.gap = Some(*n as f32),
        ("main_align", PropValue::Ident(word)) => {
            layout.main_align = Some(align_from(word));
        }
        ("cross_align", PropValue::Ident(word)) => {
            layout.cross_align = Some(align_from(word));
        }
        _ => {}
    }
}

/// Map an IR alignment ident onto the core enum. Unaligned props cannot reach
/// here (the IR validates idents against its domain), so a fallthrough is a
/// vocabulary bug, not a data condition.
fn align_from(word: &str) -> Align {
    match word {
        "start" => Align::Start,
        "center" => Align::Center,
        _ => Align::End,
    }
}

fn set_inset(layout: &mut LayoutRequest, side: usize, value: Option<f32>) {
    let inset = layout.inset.get_or_insert_with(Default::default);
    match side {
        0 => inset.left = value,
        1 => inset.top = value,
        2 => inset.right = value,
        _ => inset.bottom = value,
    }
}

/// Split properties into their seats: the style table and the layout table.
///
/// Classification goes through the IR's own `prop_spec` — this module owns no
/// vocabulary, so a property the IR reclassifies moves seats automatically.
/// Both the mount path and the differ funnel through here, or the two would
/// disagree the moment a property changed class.
pub(crate) fn seat_props(kind: NodeKind, props: &[Prop]) -> (StyleProps, LayoutRequest) {
    let mut style = StyleProps::default();
    let mut layout = LayoutRequest::new();
    for prop in props {
        let Some(spec) = floom_ui_ir::prop_spec(kind, &prop.name) else {
            continue;
        };
        match spec.class {
            floom_ui_ir::PropClass::Style => set_style_prop(&mut style, prop),
            floom_ui_ir::PropClass::Layout => set_layout_prop(&mut layout, prop),
            // Widget properties belong to the widget instance, which this
            // headless tree does not instantiate. Documented boundary.
            floom_ui_ir::PropClass::Widget => {}
        }
    }
    (style, layout)
}
