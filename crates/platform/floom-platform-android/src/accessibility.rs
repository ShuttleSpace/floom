// SPDX-License-Identifier: MIT
//! Android accessibility: the semantics tree, shaped for `AccessibilityNodeProvider`.
//!
//! The JVM half lives in the host's `FloomSurfaceView`: it owns the
//! `AccessibilityNodeProvider`, turns what arrives here into
//! `AccessibilityNodeInfo`s, and sends a node's activation back. This half
//! decides *what* the tree looks like — reading order, labels, frames — and
//! hands it over in one call per change.
//!
//! Without a publisher the adapter still retains the tree (tooling and tests
//! read it), but [`AccessibilityAdapter::perform_action`] reports `false` and
//! `NATIVE_BRIDGE` stays unset: there is no JVM to deliver to.

use std::sync::Mutex;

use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, AccessibilityFrame, SemanticsAction,
    SemanticsDiff, SemanticsNode, SemanticsRole, SemanticsState,
};
use floom_types::{ElementId, Rect};

/// One node, in the shape the JVM consumes.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Position in the published list; the JVM sends this back on activation.
    pub index: usize,
    pub label: String,
    /// Role name the JVM maps to an `AccessibilityNodeInfo` class name.
    pub role: &'static str,
    pub bounds: Rect,
    pub enabled: bool,
    pub selected: bool,
    pub focused: bool,
}

/// Receives the whole tree whenever it changes.
pub type Publisher = Box<dyn FnMut(&[Node])>;

type Activate = Box<dyn Fn(ElementId) -> bool + Send>;

/// Published order, so an activation index can be turned back into a node.
static ORDER: Mutex<Vec<ElementId>> = Mutex::new(Vec::new());

static ACTIVATE: Mutex<Option<Activate>> = Mutex::new(None);

/// Registers the host's activation handler — the only thing that holds the tree.
pub fn set_activate_sink(sink: Activate) {
    if let Ok(mut slot) = ACTIVATE.lock() {
        *slot = Some(sink);
    }
}

/// Routes the JVM's `nativeA11yAction(index)` to the widget behind that node.
pub fn activate(index: usize) -> bool {
    let node = ORDER
        .lock()
        .ok()
        .and_then(|order| order.get(index).copied());
    let Some(node) = node else {
        return false;
    };
    ACTIVATE
        .lock()
        .ok()
        .and_then(|sink| sink.as_ref().map(|sink| sink(node)))
        .unwrap_or(false)
}

/// Role name for the JVM's `AccessibilityNodeInfo` class name.
pub fn role_name(role: SemanticsRole) -> &'static str {
    match role {
        SemanticsRole::None => "none",
        SemanticsRole::Button => "button",
        SemanticsRole::CheckBox => "check_box",
        SemanticsRole::Radio => "radio",
        SemanticsRole::Switch => "switch",
        SemanticsRole::Slider => "slider",
        SemanticsRole::TextField => "text_field",
        SemanticsRole::TextView => "text_view",
        SemanticsRole::Image => "image",
        SemanticsRole::ImageButton => "image_button",
        SemanticsRole::Link => "link",
        SemanticsRole::Heading => "heading",
        SemanticsRole::StaticText => "static_text",
        SemanticsRole::List => "list",
        SemanticsRole::ListItem => "list_item",
        SemanticsRole::Group => "group",
        SemanticsRole::Scrollable => "scrollable",
        SemanticsRole::ProgressBar => "progress_bar",
        SemanticsRole::Tab => "tab",
        SemanticsRole::TabBar => "tab_bar",
        SemanticsRole::Dialog => "dialog",
        SemanticsRole::MenuItem => "menu_item",
        SemanticsRole::Custom(_) => "custom",
    }
}

/// The Android adapter.
pub struct AndroidAccessibility {
    frame: AccessibilityFrame,
    publisher: Option<Publisher>,
}

impl AndroidAccessibility {
    pub fn new() -> Self {
        Self {
            frame: AccessibilityFrame::default(),
            publisher: None,
        }
    }

    /// An adapter that hands the tree to `publisher` on every change.
    pub fn with_publisher(publisher: Publisher) -> Self {
        Self {
            frame: AccessibilityFrame::default(),
            publisher: Some(publisher),
        }
    }

    /// The tree as currently published.
    pub fn frame(&self) -> &AccessibilityFrame {
        &self.frame
    }

    fn publish(&mut self) {
        // Reading order is top to bottom, then left to right. A map iteration
        // order is arbitrary, and TalkBack would read a form sideways.
        let mut listed: Vec<(&ElementId, &SemanticsNode)> = self
            .frame
            .nodes
            .iter()
            .filter(|(_, node)| node.role != SemanticsRole::None)
            .filter(|(_, node)| !node.state.contains(SemanticsState::HIDDEN))
            .collect();
        listed.sort_by(|a, b| {
            a.1.bounds
                .y0
                .total_cmp(&b.1.bounds.y0)
                .then(a.1.bounds.x0.total_cmp(&b.1.bounds.x0))
        });

        let mut order = Vec::with_capacity(listed.len());
        let mut nodes = Vec::with_capacity(listed.len());
        for (id, node) in listed {
            order.push(*id);
            nodes.push(Node {
                index: nodes.len(),
                label: node.label.clone(),
                role: role_name(node.role),
                bounds: node.bounds,
                enabled: !node.state.contains(SemanticsState::DISABLED),
                selected: node.state.contains(SemanticsState::SELECTED),
                focused: node.state.contains(SemanticsState::FOCUSED),
            });
        }

        if let Ok(mut slot) = ORDER.lock() {
            *slot = order;
        }
        if let Some(publisher) = self.publisher.as_mut() {
            publisher(&nodes);
        }
    }
}

impl Default for AndroidAccessibility {
    fn default() -> Self {
        Self::new()
    }
}

impl AccessibilityAdapter for AndroidAccessibility {
    fn update(&mut self, diff: &SemanticsDiff) {
        for (id, node) in diff.added.iter().chain(diff.updated.iter()) {
            self.frame.nodes.insert(*id, node.clone());
        }
        for id in &diff.removed {
            self.frame.nodes.remove(id);
        }
        self.publish();
    }

    fn capabilities(&self) -> AccessibilityCapabilities {
        let base =
            AccessibilityCapabilities::SCREEN_READER | AccessibilityCapabilities::ACTION_INVOKE;
        // The bridge bit is a claim about a delivery path, so it is set only
        // when there is one.
        if self.publisher.is_some() {
            base | AccessibilityCapabilities::NATIVE_BRIDGE
        } else {
            base
        }
    }

    fn perform_action(&mut self, node: ElementId, action: SemanticsAction) -> bool {
        // No publisher means no JVM half: an action that has nowhere to be
        // delivered from cannot have been delivered.
        if self.publisher.is_none() {
            return false;
        }
        match action {
            SemanticsAction::Activate => ACTIVATE
                .lock()
                .ok()
                .and_then(|sink| sink.as_ref().map(|sink| sink(node)))
                .unwrap_or(false),
            // Reporting success on the others would make TalkBack's menu claim
            // gestures that move nothing.
            _ => false,
        }
    }

    fn reset(&mut self) {
        self.frame = AccessibilityFrame::default();
        self.publish();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::{Arc, Mutex};

    use super::*;
    use floom_types::Point;

    /// Two tests below touch the same process-wide order/sink, so they take
    /// turns rather than fight over it.
    static SLOT: Mutex<()> = Mutex::new(());

    fn node(role: SemanticsRole, label: &str, x: f64, y: f64) -> SemanticsNode {
        SemanticsNode::new(role)
            .with_label(label)
            .with_bounds(Rect::from_origin_size(
                Point::new(x, y),
                floom_types::Size::new(80.0, 24.0),
            ))
    }

    fn diff(nodes: Vec<(u32, SemanticsNode)>) -> SemanticsDiff {
        SemanticsDiff {
            added: nodes
                .into_iter()
                .map(|(i, n)| (ElementId::new(i, 1), n))
                .collect(),
            ..SemanticsDiff::default()
        }
    }

    #[test]
    fn publishes_in_reading_order_with_frames() {
        let _guard = SLOT.lock().unwrap_or_else(|e| e.into_inner());
        let seen: Rc<RefCell<Vec<Node>>> = Rc::new(RefCell::new(Vec::new()));
        let sink = Rc::clone(&seen);
        let mut adapter = AndroidAccessibility::with_publisher(Box::new(move |nodes| {
            sink.borrow_mut().extend_from_slice(nodes)
        }));
        adapter.update(&diff(vec![
            (2, node(SemanticsRole::Button, "Second", 0.0, 60.0)),
            (1, node(SemanticsRole::Button, "First", 0.0, 10.0)),
        ]));

        let published = seen.borrow();
        assert_eq!(
            published
                .iter()
                .map(|n| n.label.as_str())
                .collect::<Vec<_>>(),
            vec!["First", "Second"],
            "top to bottom, not map order"
        );
        assert_eq!(published[0].bounds.y0, 10.0, "the frame comes from layout");
        assert_eq!(published[0].role, "button");
        assert!(
            adapter
                .capabilities()
                .contains(AccessibilityCapabilities::NATIVE_BRIDGE)
        );
    }

    #[test]
    fn hidden_and_anonymous_nodes_stay_off_the_screen_reader() {
        let _guard = SLOT.lock().unwrap_or_else(|e| e.into_inner());
        let seen: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
        let sink = Rc::clone(&seen);
        let mut adapter = AndroidAccessibility::with_publisher(Box::new(move |nodes| {
            *sink.borrow_mut() = nodes.len();
        }));
        let hidden = SemanticsNode::new(SemanticsRole::Button)
            .with_label("gone")
            .with_state(SemanticsState::HIDDEN);
        adapter.update(&diff(vec![
            (1, hidden),
            (2, SemanticsNode::new(SemanticsRole::None)),
        ]));
        assert_eq!(*seen.borrow(), 0);
    }

    #[test]
    fn activation_from_the_jvm_lands_on_the_node_it_was_given() {
        let _guard = SLOT.lock().unwrap_or_else(|e| e.into_inner());
        let hit: Arc<Mutex<Vec<u32>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&hit);
        set_activate_sink(Box::new(move |id| {
            if let Ok(mut hits) = sink.lock() {
                hits.push(id.index());
            }
            true
        }));

        let mut adapter = AndroidAccessibility::new();
        adapter.update(&diff(vec![
            (7, node(SemanticsRole::Button, "Below", 0.0, 90.0)),
            (9, node(SemanticsRole::Button, "Above", 0.0, 30.0)),
        ]));

        assert!(activate(0), "index 0 is the topmost node");
        assert!(activate(1));
        assert!(!activate(2), "an index that was never published");
        assert_eq!(*hit.lock().unwrap(), vec![9, 7]);
    }

    #[test]
    fn without_a_bridge_the_tree_is_retained_but_actions_do_not_claim_success() {
        let _guard = SLOT.lock().unwrap_or_else(|e| e.into_inner());
        let mut adapter = AndroidAccessibility::new();
        adapter.update(&diff(vec![(
            1,
            node(SemanticsRole::Button, "OK", 0.0, 0.0),
        )]));
        assert_eq!(adapter.frame().node_count(), 1);
        assert!(
            !adapter
                .capabilities()
                .contains(AccessibilityCapabilities::NATIVE_BRIDGE)
        );
        assert!(!adapter.perform_action(ElementId::new(1, 1), SemanticsAction::Activate));
        assert!(
            !adapter.perform_action(ElementId::new(1, 1), SemanticsAction::SetValue("x".into()))
        );
    }
}
