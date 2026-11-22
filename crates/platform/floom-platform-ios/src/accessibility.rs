// SPDX-License-Identifier: MIT
//! UIKit assistive-technology bridge.
//!
//! The semantics tree reaches UIKit as a flat, ordered list of elements: the
//! Swift host turns each record into a `UIAccessibilityElement` and assigns the
//! array to `view.accessibilityElements`. Element objects are UIKit's, so they
//! are created there; this module only decides *what* is accessible and *where*.
//!
//! Frames are in the layout's units, which are points — the same units the host
//! passes to `floom_ios_app_start`.

use std::ffi::{CString, c_char};
use std::sync::Mutex;

use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, AccessibilityFrame, SemanticsAction,
    SemanticsDiff, SemanticsRole, SemanticsState,
};
use floom_types::ElementId;

/// One accessible element. `#[repr(C)]` — mirrored by `FloomA11yNode` in the
/// Swift host; change both together.
#[repr(C)]
#[derive(Debug)]
pub struct A11yNode {
    /// NUL-terminated UTF-8, valid only for the duration of the sink call.
    pub label: *const c_char,
    /// See [`role_code`]; the host maps it to `UIAccessibilityTraits`.
    pub role: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub enabled: bool,
    pub selected: bool,
    pub focused: bool,
}

/// Receives the whole list after every change. Synchronous: the records and the
/// labels they point at die when it returns.
pub type Sink = unsafe extern "C" fn(nodes: *const A11yNode, count: usize);

static SINK: Mutex<Option<Sink>> = Mutex::new(None);

/// Called by the Swift host once, with a function that rebuilds its elements.
///
/// # Safety
/// `sink` must stay callable for as long as the process runs, and must not
/// retain anything it is handed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn floom_ios_set_a11y_sink(sink: Option<Sink>) {
    *SINK.lock().unwrap_or_else(|e| e.into_inner()) = sink;
}

/// Stable role codes shared with the Swift host.
pub fn role_code(role: SemanticsRole) -> u32 {
    match role {
        SemanticsRole::None => 0,
        SemanticsRole::Button => 1,
        SemanticsRole::CheckBox => 2,
        SemanticsRole::Radio => 3,
        SemanticsRole::Switch => 4,
        SemanticsRole::Slider => 5,
        SemanticsRole::TextField => 6,
        SemanticsRole::TextView => 7,
        SemanticsRole::Image => 8,
        SemanticsRole::ImageButton => 9,
        SemanticsRole::Link => 10,
        SemanticsRole::Heading => 11,
        SemanticsRole::StaticText => 12,
        SemanticsRole::List => 13,
        SemanticsRole::ListItem => 14,
        SemanticsRole::Group => 15,
        SemanticsRole::Scrollable => 16,
        SemanticsRole::ProgressBar => 17,
        SemanticsRole::Tab => 18,
        SemanticsRole::TabBar => 19,
        SemanticsRole::Dialog => 20,
        SemanticsRole::MenuItem => 21,
        SemanticsRole::Custom(_) => 100,
    }
}

/// Adapter that forwards the tree to the Swift host.
#[derive(Debug, Default)]
pub struct IosAccessibility {
    frame: AccessibilityFrame,
}

impl IosAccessibility {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_count(&self) -> usize {
        self.frame.node_count()
    }

    /// Rebuild the host's elements. Reading order is top-to-bottom, then
    /// left-to-right: hash-map order would be arbitrary, and VoiceOver reads in
    /// array order.
    fn publish(&self) {
        let sink = *SINK.lock().unwrap_or_else(|e| e.into_inner());
        let Some(sink) = sink else {
            return;
        };

        let mut nodes: Vec<_> = self
            .frame
            .nodes
            .iter()
            .filter(|(_, node)| {
                node.role != SemanticsRole::None && !node.state.contains(SemanticsState::HIDDEN)
            })
            .collect();
        nodes.sort_by(|(_, a), (_, b)| {
            a.bounds
                .y0
                .partial_cmp(&b.bounds.y0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.bounds
                        .x0
                        .partial_cmp(&b.bounds.x0)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        // The labels outlive the records below by exactly this scope.
        let labels: Vec<CString> = nodes
            .iter()
            .map(|(_, node)| {
                CString::new(node.label.as_str()).unwrap_or_else(|_| CString::default())
            })
            .collect();
        let records: Vec<A11yNode> = nodes
            .iter()
            .zip(&labels)
            .map(|((_, node), label)| A11yNode {
                label: label.as_ptr(),
                role: role_code(node.role),
                x: node.bounds.x0 as f32,
                y: node.bounds.y0 as f32,
                width: node.bounds.width() as f32,
                height: node.bounds.height() as f32,
                enabled: node.state.contains(SemanticsState::ENABLED),
                selected: node.state.contains(SemanticsState::SELECTED),
                focused: node.state.contains(SemanticsState::FOCUSED),
            })
            .collect();

        // SAFETY: the pointer and every label it points at are alive for the
        // whole call, and the count matches the slice.
        unsafe { sink(records.as_ptr(), records.len()) };
    }
}

impl AccessibilityAdapter for IosAccessibility {
    fn update(&mut self, diff: &SemanticsDiff) {
        for (id, node) in diff.added.iter().chain(&diff.updated) {
            self.frame.nodes.insert(*id, node.clone());
        }
        for id in &diff.removed {
            self.frame.nodes.remove(id);
        }
        self.publish();
    }

    fn capabilities(&self) -> AccessibilityCapabilities {
        // Actions are not delivered yet: UIKit would need to call back into the
        // element's handler, which no host hook exists for.
        AccessibilityCapabilities::SCREEN_READER | AccessibilityCapabilities::NATIVE_BRIDGE
    }

    fn perform_action(&mut self, _node: ElementId, _action: SemanticsAction) -> bool {
        false
    }

    fn reset(&mut self) {
        self.frame = AccessibilityFrame::default();
        self.publish();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex as StdMutex;

    use super::*;
    use floom_accessibility::SemanticsNode;
    use floom_types::Rect;

    static SEEN: StdMutex<Vec<(String, u32, f32, f32)>> = StdMutex::new(Vec::new());

    /// The sink is one process-wide slot; tests that install it must not
    /// interleave, or one clears what the other just recorded.
    static SINK_SLOT: StdMutex<()> = StdMutex::new(());

    fn slot() -> std::sync::MutexGuard<'static, ()> {
        SINK_SLOT.lock().unwrap_or_else(|e| e.into_inner())
    }

    unsafe extern "C" fn record(nodes: *const A11yNode, count: usize) {
        let mut seen = SEEN.lock().unwrap_or_else(|e| e.into_inner());
        seen.clear();
        for i in 0..count {
            // SAFETY: the adapter hands over `count` live records.
            let node = unsafe { &*nodes.add(i) };
            let label = unsafe { std::ffi::CStr::from_ptr(node.label) }
                .to_string_lossy()
                .into_owned();
            seen.push((label, node.role, node.x, node.y));
        }
    }

    fn node(role: SemanticsRole, label: &str, x: f64, y: f64, w: f64, h: f64) -> SemanticsNode {
        SemanticsNode::new(role)
            .with_label(label)
            .with_bounds(Rect {
                x0: x,
                y0: y,
                x1: x + w,
                y1: y + h,
            })
    }

    fn diff(nodes: Vec<(ElementId, SemanticsNode)>) -> SemanticsDiff {
        SemanticsDiff {
            added: nodes,
            ..Default::default()
        }
    }

    #[test]
    fn the_record_layout_is_what_the_swift_host_asserts() {
        // The Swift side hard-codes 32 and decodes with `assumingMemoryBound`;
        // a field added here without the mirror there reads garbage.
        assert_eq!(std::mem::size_of::<A11yNode>(), 32);
        assert_eq!(std::mem::align_of::<A11yNode>(), 8);
    }

    #[test]
    fn the_host_receives_nodes_in_reading_order() {
        let _guard = slot();
        // SAFETY: `record` is a plain function with a `'static` address.
        unsafe { floom_ios_set_a11y_sink(Some(record)) };
        let mut adapter = IosAccessibility::new();
        adapter.update(&diff(vec![
            (
                ElementId::new(3, 1),
                node(SemanticsRole::Button, "third", 0.0, 80.0, 72.0, 22.0),
            ),
            (
                ElementId::new(2, 1),
                node(SemanticsRole::StaticText, "second", 66.0, 40.0, 90.0, 18.0),
            ),
            (
                ElementId::new(1, 1),
                node(SemanticsRole::Heading, "first", 0.0, 40.0, 60.0, 18.0),
            ),
        ]));

        let seen = SEEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let labels: Vec<&str> = seen.iter().map(|(l, ..)| l.as_str()).collect();
        assert_eq!(labels, ["first", "second", "third"], "{seen:?}");
        assert_eq!(seen[2].1, role_code(SemanticsRole::Button));
        assert_eq!(
            (seen[2].2, seen[2].3),
            (0.0, 80.0),
            "frames travel as points"
        );
        // SAFETY: clearing the sink is always valid.
        unsafe { floom_ios_set_a11y_sink(None) };
    }

    #[test]
    fn hidden_and_anonymous_nodes_are_not_published() {
        let _guard = slot();
        // SAFETY: as above.
        unsafe { floom_ios_set_a11y_sink(Some(record)) };
        let mut adapter = IosAccessibility::new();
        adapter.update(&diff(vec![
            (
                ElementId::new(1, 1),
                node(SemanticsRole::Button, "visible", 0.0, 0.0, 10.0, 10.0),
            ),
            (
                ElementId::new(2, 1),
                node(SemanticsRole::Button, "hidden", 0.0, 0.0, 10.0, 10.0)
                    .with_state(SemanticsState::HIDDEN),
            ),
            (
                ElementId::new(3, 1),
                node(SemanticsRole::None, "decorative", 0.0, 0.0, 10.0, 10.0),
            ),
        ]));

        let seen = SEEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert_eq!(seen[0].0, "visible");
        // SAFETY: as above.
        unsafe { floom_ios_set_a11y_sink(None) };
    }

    #[test]
    fn a_removal_rebuilds_the_list_without_the_node() {
        let _guard = slot();
        // SAFETY: as above.
        unsafe { floom_ios_set_a11y_sink(Some(record)) };
        let mut adapter = IosAccessibility::new();
        adapter.update(&diff(vec![
            (
                ElementId::new(1, 1),
                node(SemanticsRole::Button, "kept", 0.0, 0.0, 10.0, 10.0),
            ),
            (
                ElementId::new(2, 1),
                node(SemanticsRole::Button, "gone", 0.0, 20.0, 10.0, 10.0),
            ),
        ]));
        adapter.update(&SemanticsDiff {
            removed: vec![ElementId::new(2, 1)],
            ..Default::default()
        });

        let seen = SEEN.lock().unwrap_or_else(|e| e.into_inner()).clone();
        assert_eq!(seen.len(), 1, "{seen:?}");
        assert_eq!(seen[0].0, "kept");
        assert_eq!(adapter.node_count(), 1);
        // SAFETY: as above.
        unsafe { floom_ios_set_a11y_sink(None) };
    }
}
