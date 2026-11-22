// SPDX-License-Identifier: MIT
//! Android accessibility: the semantics tree, shaped for `AccessibilityNodeProvider`.
//!
//! The JVM half lives in the host's `FloomSurfaceView`: it owns the
//! `AccessibilityNodeProvider`, turns what arrives here into
//! `AccessibilityNodeInfo`s, and sends a node's activation back. This half
//! decides *what* the tree looks like — reading order, labels, frames — and
//! hands it over in one call per change.
//!
//! On a device the publisher stores the encoded tree, and the host's
//! `dev.floom.android.FloomA11y` pulls it whenever `AccessibilityNodeProvider`
//! asks for children — pulling matches who decides *when* the tree is needed.
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

/// The tree as the JVM half reads it: one node per line, tab separated.
///
/// Stored rather than pushed so the host can pull on the accessibility
/// thread at the moment `AccessibilityNodeProvider` needs children, instead of
/// marshalling to the UI thread on every change.
#[cfg(target_os = "android")]
static PUBLISHED: Mutex<String> = Mutex::new(String::new());

/// `index`, `role`, `label`, `x`, `y`, `width`, `height`, then three flags.
///
/// Tabs and newlines inside a label become spaces: the format's separator is
/// the only thing the JVM half parses against.
// Only the JVM half reads the encoding, so on a host this exists for the test.
#[cfg(any(target_os = "android", test))]
fn encode(nodes: &[Node]) -> String {
    let mut out = String::with_capacity(nodes.len() * 64);
    for node in nodes {
        let label: String = node
            .label
            .chars()
            .map(|c| {
                if c == '\t' || c == '\n' || c == '\r' {
                    ' '
                } else {
                    c
                }
            })
            .collect();
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            node.index,
            node.role,
            label,
            node.bounds.x0,
            node.bounds.y0,
            node.bounds.width(),
            node.bounds.height(),
            u8::from(node.enabled),
            u8::from(node.selected),
            u8::from(node.focused),
        ));
    }
    out
}

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
            publisher: default_publisher(),
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

/// The publisher a device needs: store the encoded tree for the JVM half.
///
/// `None` on the host, where there is no JVM to pull it — and where a stored
/// tree would only make `NATIVE_BRIDGE` a lie.
fn default_publisher() -> Option<Publisher> {
    #[cfg(target_os = "android")]
    {
        Some(Box::new(store))
    }
    #[cfg(not(target_os = "android"))]
    {
        None
    }
}

/// Keep the latest tree where [`jvm::Java_dev_floom_android_FloomA11y_pull`]
/// can read it.
#[cfg(target_os = "android")]
fn store(nodes: &[Node]) {
    if let Ok(mut published) = PUBLISHED.lock() {
        published.clear();
        published.push_str(&encode(nodes));
    }
}

/// The JNI entries `dev.floom.android.FloomA11y` declares as `external fun`s.
///
/// Names are mangled by hand because the class is fixed: the scaffold ships it
/// as part of the Android project, so its path is not the app's to choose. The
/// first parameter is [`jni::EnvUnowned`], which is what JNI actually passes —
/// `jni::Env` is not FFI safe and taking it by value dereferences a bogus
/// pointer.
#[cfg(target_os = "android")]
mod jvm {
    use super::{PUBLISHED, activate};
    use jni::objects::{JClass, JString};
    use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jint};

    /// The tree the provider should show, or null when there is none yet.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_floom_android_FloomA11y_pull<'local>(
        mut unowned_env: jni::EnvUnowned<'local>,
        _class: JClass<'local>,
    ) -> JString<'local> {
        unowned_env
            .with_env(|env| {
                let payload = match PUBLISHED.lock() {
                    Ok(published) => published.clone(),
                    Err(_) => String::new(),
                };
                JString::from_str(env, payload)
            })
            .resolve::<jni::errors::ThrowRuntimeExAndDefault>()
    }

    /// `performAction(ACTION_CLICK)` for node `index`; false when nothing ran.
    #[unsafe(no_mangle)]
    pub extern "system" fn Java_dev_floom_android_FloomA11y_activate(
        _env: jni::EnvUnowned,
        _class: JClass,
        index: jint,
    ) -> jboolean {
        if index < 0 {
            return JNI_FALSE;
        }
        if activate(index as usize) {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
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
    fn the_encoded_tree_is_one_line_per_node() {
        let nodes = vec![Node {
            index: 0,
            label: "Increment\tx\ny".to_string(),
            role: "button",
            bounds: Rect {
                x0: 0.0,
                y0: 24.0,
                x1: 78.0,
                y1: 46.0,
            },
            enabled: true,
            selected: false,
            focused: false,
        }];
        // Tabs and newlines in a label are flattened: the line's own tabs are
        // the only separators the JVM half parses against.
        assert_eq!(
            encode(&nodes),
            "0\tbutton\tIncrement x y\t0\t24\t78\t22\t1\t0\t0\n"
        );
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
