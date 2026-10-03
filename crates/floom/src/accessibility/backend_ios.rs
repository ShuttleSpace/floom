//! iOS accessibility backend (VoiceOver).
//!
//! Unlike macOS/Windows, iOS has **no `accesskit` adapter** — `accesskit`
//! only covers the desktop platforms. So this backend speaks the iOS
//! `UIAccessibility` protocol directly: it subclasses `UIAccessibilityElement`
//! once per Floom a11y node, mirrors the neutral tree under the window's
//! `UIView` (which acts as the root accessibility container), and routes
//! VoiceOver actions (activate / increment / decrement / scroll) back to core
//! through the same `PendingAction` channel the desktop backends use.
//!
//! Everything here runs on the main thread (UIKit and winit's
//! `UIApplicationMain` both live there), which is why the Objective-C objects
//! are kept inside this `#[cfg(target_os = "ios")]`-only module.
//!
//! ## Tree sync
//!
//! `update_tree` receives a **full** neutral tree every frame (see
//! `bridge::build_access_tree_update`). To avoid churning VoiceOver element
//! identity, we keep an `elements` map keyed by stable [`AccessNodeId`] and
//! reconcile incrementally: new ids get a fresh element, gone ids are dropped,
//! and every surviving element's properties (label/value/hint/traits/frame)
//! are refreshed in place. The native `accessibilityElements` arrays (the
//! container hierarchy VoiceOver traverses) and the
//! `UIAccessibilityLayoutChangedNotification` are only recomputed/posted when
//! the tree *topology* (root or any node's children) actually changes.

// This entire module is unsafe FFI against UIKit; every function here is
// already marked `unsafe fn`, so the in-body `unsafe {}` block requirement
// (which the current toolchain warns about for `unsafe fn`) is noise here.
#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::mpsc;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_foundation::{NSArray, NSObjectProtocol, NSString, NSPoint, NSRect, NSSize};
use objc2_ui_kit::{
    UIAccessibilityConvertFrameToScreenCoordinates, UIAccessibilityElement,
    UIAccessibilityLayoutChangedNotification, UIAccessibilityPostNotification,
    UIAccessibilityScrollDirection, UIAccessibilityTraitAdjustable, UIAccessibilityTraitButton,
    UIAccessibilityTraitHeader, UIAccessibilityTraitImage, UIAccessibilityTraitLink,
    UIAccessibilityTraitNotEnabled, UIAccessibilityTraitSelected, UIAccessibilityTraitStaticText,
    UIAccessibilityTraits, UIView,
};
use peniko::kurbo::Point;

use crate::accessibility::{
    AccessAction, AccessNode, AccessNodeId, AccessRole, AccessState, AccessTree,
    AccessTreeUpdate, AccessibilityBackend, PendingAction,
};

/// Bit flags packing the `UIAccessibilityElement`-mappable actions a node
/// supports. Stored in the element's ivars so VoiceOver's action callbacks can
/// decide whether to forward the action to core.
const ACTION_DEFAULT: u32 = 1 << 0;
const ACTION_INCREMENT: u32 = 1 << 1;
const ACTION_DECREMENT: u32 = 1 << 2;
const ACTION_SCROLL: u32 = 1 << 3;

/// Ivars for the `FloomA11yElement` subclass.
///
/// `tx` is a raw pointer to the backend's action `Sender` (owned by the
/// backend that owns this element, so it outlives the element). We store a raw
/// pointer rather than the `Sender` directly because `mpsc::Sender` is not
/// `Encode` and therefore cannot live in a `define_class!` ivar struct.
struct A11yIvars {
    node_id: u64,
    actions: u32,
    tx: *const mpsc::Sender<PendingAction>,
}

define_class!(
    #[unsafe(super(UIAccessibilityElement))]
    #[thread_kind = MainThreadOnly]
    #[ivars = A11yIvars]
    #[name = "FloomA11yElement"]
    struct FloomA11yElement;

    unsafe impl NSObjectProtocol for FloomA11yElement {}

    impl FloomA11yElement {
        /// VoiceOver "activate" (double-tap / Enter) → our `Default` action
        /// (e.g. click a button).
        #[unsafe(method(accessibilityActivate))]
        fn accessibility_activate(&self) -> bool {
            let iv = self.ivars();
            if iv.actions & ACTION_DEFAULT != 0 {
                unsafe { send_action(iv.tx, iv.node_id, AccessAction::Default, None) };
                true
            } else {
                false
            }
        }

        /// VoiceOver "increment" gesture → our `Increment` action.
        #[unsafe(method(accessibilityIncrement))]
        fn accessibility_increment(&self) {
            let iv = self.ivars();
            if iv.actions & ACTION_INCREMENT != 0 {
                unsafe { send_action(iv.tx, iv.node_id, AccessAction::Increment, None) };
            }
        }

        /// VoiceOver "decrement" gesture → our `Decrement` action.
        #[unsafe(method(accessibilityDecrement))]
        fn accessibility_decrement(&self) {
            let iv = self.ivars();
            if iv.actions & ACTION_DECREMENT != 0 {
                unsafe { send_action(iv.tx, iv.node_id, AccessAction::Decrement, None) };
            }
        }

        /// VoiceOver scroll gesture → our `ScrollIntoView` action.
        #[unsafe(method(accessibilityScroll:))]
        fn accessibility_scroll(&self, _direction: UIAccessibilityScrollDirection) -> bool {
            let iv = self.ivars();
            if iv.actions & ACTION_SCROLL != 0 {
                unsafe { send_action(iv.tx, iv.node_id, AccessAction::ScrollIntoView, None) };
                true
            } else {
                false
            }
        }
    }
);

impl FloomA11yElement {
    /// Allocate + init a `FloomA11yElement` anchored to `container` (the
    /// window's `UIView`).
    fn new(
        mtm: MainThreadMarker,
        container: &AnyObject,
        node_id: u64,
        actions: u32,
        tx: *const mpsc::Sender<PendingAction>,
    ) -> Retained<Self> {
        unsafe {
            let obj = Self::alloc(mtm).set_ivars(A11yIvars {
                node_id,
                actions,
                tx,
            });
            let el: Retained<Self> =
                msg_send![super(obj, UIAccessibilityElement::class()), initWithAccessibilityContainer: container];
            el
        }
    }
}

/// Reinterpret an Objective-C object reference (the underlying `NSObject`, not
/// a `Retained` wrapper) as `&AnyObject` so it can be passed where the root
/// class type is expected.
///
/// # Safety
/// `obj` must be a valid message-receiving object pointer.
unsafe fn as_any<T: ?Sized>(obj: &T) -> &AnyObject {
    &*(obj as *const T as *const AnyObject)
}

/// Forward a VoiceOver action to Floom core via the backend's channel.
///
/// # Safety
/// `tx` must be a valid, live pointer to the backend's `mpsc::Sender`
/// (the backend owns both the sender and the element, and outlives it).
unsafe fn send_action(
    tx: *const mpsc::Sender<PendingAction>,
    node: u64,
    action: AccessAction,
    payload: Option<String>,
) {
    if tx.is_null() {
        return;
    }
    let _ = unsafe { (*tx).send((AccessNodeId(node), action, payload)) };
}

/// Bridges Floom's neutral accessibility tree to iOS VoiceOver.
pub(crate) struct IosAccessibilityBackend {
    /// The window's `UIView` — the root accessibility container VoiceOver
    /// starts traversal from.
    container: Retained<UIView>,
    /// One native element per live neutral node, keyed by stable id.
    elements: HashMap<AccessNodeId, Retained<FloomA11yElement>>,
    /// Neutral cache powering `focused_node` / `node_at_point`.
    tree: AccessTree,
    /// Action channel: elements → core (drained by `WindowState`).
    tx: mpsc::Sender<PendingAction>,
    rx: mpsc::Receiver<PendingAction>,
    /// Last topology we pushed, to detect when a layout-changed notification
    /// is actually warranted.
    prev_root: Option<AccessNodeId>,
    prev_children: HashMap<AccessNodeId, Vec<AccessNodeId>>,
}

impl IosAccessibilityBackend {
    /// Build the backend from the raw window `UIView` pointer.
    ///
    /// Returns `None` when `ui_view` is null (e.g. headless). Must be called on
    /// the main thread (UIKit requirement); bails safely otherwise.
    pub(crate) fn new(ui_view: *mut c_void) -> Option<Self> {
        if ui_view.is_null() {
            return None;
        }
        let mtm = MainThreadMarker::new()?;
        // Retain the view so it stays alive for the lifetime of the backend;
        // winit/UIKit still owns the original reference.
        let container: Retained<UIView> = unsafe { Retained::retain(ui_view as *mut UIView) }?;
        let (tx, rx) = mpsc::channel();
        let _ = mtm;
        Some(Self {
            container,
            elements: HashMap::new(),
            tree: AccessTree::new(),
            tx,
            rx,
            prev_root: None,
            prev_children: HashMap::new(),
        })
    }
}

impl AccessibilityBackend for IosAccessibilityBackend {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        // VoiceOver / UIKit calls must happen on the main thread.
        let mtm = match MainThreadMarker::new() {
            Some(mtm) => mtm,
            // Off-thread: still cache the tree so queries stay correct, but
            // skip touching any Objective-C objects.
            None => {
                self.tree.apply(update);
                return;
            }
        };

        let old_ids: std::collections::HashSet<AccessNodeId> =
            self.tree.iter().map(|n| n.id).collect();

        // Detect added/removed node ids from the new tree after applying.
        self.tree.apply(update);
        let new_ids: std::collections::HashSet<AccessNodeId> =
            self.tree.iter().map(|n| n.id).collect();

        // Create native elements for freshly-appeared nodes.
        let tx_ptr = &self.tx as *const mpsc::Sender<PendingAction>;
        let container_any = unsafe { as_any(&*self.container) };
        for id in new_ids.difference(&old_ids) {
            if let Some(node) = self.tree.get(*id) {
                let actions = action_bits(&node.actions);
                let el = FloomA11yElement::new(mtm, container_any, id.0, actions, tx_ptr);
                self.elements.insert(*id, el);
            }
        }

        // Drop native elements for removed nodes.
        for id in old_ids.difference(&new_ids) {
            self.elements.remove(id);
        }

        // Refresh properties of every surviving element in place.
        for id in new_ids.iter() {
            if let (Some(node), Some(el)) = (self.tree.get(*id), self.elements.get(id)) {
                unsafe { update_element(el, node, &*self.container) };
            }
        }

        // Only rebuild the container hierarchy + notify VoiceOver when the
        // topology actually changed (root or any node's children), so we don't
        // spam layout-changed notifications every frame.
        let root = self.tree.root;
        let children_now: HashMap<AccessNodeId, Vec<AccessNodeId>> = self
            .tree
            .iter()
            .map(|n| (n.id, n.children.clone()))
            .collect();
        let topology_changed = root != self.prev_root || children_now != self.prev_children;

        if topology_changed {
            unsafe { self.rebuild_containers() };
            let container_any = unsafe { as_any(&*self.container) };
            unsafe {
                UIAccessibilityPostNotification(
                    UIAccessibilityLayoutChangedNotification,
                    Some(container_any),
                );
            }
            self.prev_root = root;
            self.prev_children = children_now;
        }
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        self.tree.focus
    }

    fn node_at_point(&self, point: Point) -> Option<AccessNodeId> {
        self.tree.node_at_point(point)
    }

    fn poll_actions(&mut self) -> Vec<PendingAction> {
        // Drain every pending action (VoiceOver may queue several between
        // frames); the desktop backends only take one, but draining all is
        // strictly safer.
        let mut out = Vec::new();
        while let Ok(action) = self.rx.try_recv() {
            out.push(action);
        }
        out
    }
}

impl IosAccessibilityBackend {
    /// Re-wire the native `accessibilityElements` arrays so VoiceOver traverses
    /// the tree the same way the neutral tree is shaped:
    ///
    /// `UIView` → root element → each node's children, recursively.
    ///
    /// # Safety
    /// Must be called on the main thread (touches UIKit objects).
    unsafe fn rebuild_containers(&self) {
        let container_any = as_any(&*self.container);

        // The root element is contained by the `UIView`; the view's
        // `accessibilityElements` is just that root element.
        if let Some(root) = self.tree.root {
            if let Some(root_el) = self.elements.get(&root) {
                root_el.setAccessibilityContainer(Some(container_any));
            }
            if let Some(root_el) = self.elements.get(&root) {
                let arr: Retained<NSArray<FloomA11yElement>> =
                    NSArray::from_retained_slice(&[root_el.clone()]);
                let _: () = msg_send![container_any, setAccessibilityElements: Some(&*arr)];
            }
        }

        // For every node that has children, expose them as the node's own
        // `accessibilityElements` and point each child's container back at the
        // node.
        for node in self.tree.iter() {
            let Some(el) = self.elements.get(&node.id) else {
                continue;
            };
            if node.children.is_empty() {
                continue;
            }
            let child_any = as_any(&**el);
            let child_els: Vec<Retained<FloomA11yElement>> = node
                .children
                .iter()
                .filter_map(|c| self.elements.get(c).cloned())
                .collect();
            if child_els.is_empty() {
                continue;
            }
            let arr = NSArray::from_retained_slice(&child_els);
            let _: () = msg_send![child_any, setAccessibilityElements: Some(&*arr)];
            for ce in &child_els {
                ce.setAccessibilityContainer(Some(child_any));
            }
        }
    }
}

/// Map the neutral supported actions to the `UIAccessibilityElement`-mappable
/// bit flags.
fn action_bits(actions: &[AccessAction]) -> u32 {
    let mut bits = 0u32;
    for a in actions {
        bits |= match a {
            AccessAction::Default => ACTION_DEFAULT,
            AccessAction::Increment => ACTION_INCREMENT,
            AccessAction::Decrement => ACTION_DECREMENT,
            AccessAction::ScrollIntoView => ACTION_SCROLL,
            _ => 0,
        };
    }
    bits
}

/// Whether a node should be an accessibility *element* (something VoiceOver
/// reads/activates) versus a pure container.
fn is_element(role: AccessRole) -> bool {
    !matches!(
        role,
        AccessRole::Window
            | AccessRole::Application
            | AccessRole::Group
            | AccessRole::Panel
            | AccessRole::None
            | AccessRole::Unknown
    )
}

/// Translate a neutral node's role + states into `UIAccessibilityTraits`.
///
/// # Safety
/// Reads `extern static` trait constants; callers must be in an unsafe
/// context.
unsafe fn traits_for(node: &AccessNode) -> UIAccessibilityTraits {
    let mut t: UIAccessibilityTraits = 0;
    t |= match node.role {
        AccessRole::Button => UIAccessibilityTraitButton,
        AccessRole::Link => UIAccessibilityTraitLink,
        AccessRole::Image => UIAccessibilityTraitImage,
        AccessRole::Heading => UIAccessibilityTraitHeader,
        AccessRole::Text | AccessRole::Paragraph | AccessRole::List | AccessRole::ListItem => {
            UIAccessibilityTraitStaticText
        }
        AccessRole::Slider | AccessRole::SpinButton => UIAccessibilityTraitAdjustable,
        _ => 0,
    };
    if node.states.contains(AccessState::SELECTED) {
        t |= UIAccessibilityTraitSelected;
    }
    if node.states.contains(AccessState::DISABLED) {
        t |= UIAccessibilityTraitNotEnabled;
    }
    t
}

/// Push a neutral node's properties onto its native element.
///
/// # Safety
/// Must be called on the main thread (touches UIKit objects; reads trait
/// constants).
unsafe fn update_element(el: &FloomA11yElement, node: &AccessNode, container: &UIView) {
    // Label / value / hint.
    let label: Option<Retained<NSString>> = node.label.as_deref().map(NSString::from_str);
    let _: () = msg_send![el, setAccessibilityLabel: label.as_deref()];
    let value: Option<Retained<NSString>> = node.value.as_deref().map(NSString::from_str);
    let _: () = msg_send![el, setAccessibilityValue: value.as_deref()];
    let hint: Option<Retained<NSString>> = node.description.as_deref().map(NSString::from_str);
    let _: () = msg_send![el, setAccessibilityHint: hint.as_deref()];

    // Role + state → traits.
    let _: () = msg_send![el, setAccessibilityTraits: traits_for(node)];

    // Element vs container.
    let _: () = msg_send![el, setIsAccessibilityElement: is_element(node.role)];

    // Geometry: convert window-local logical points to screen coordinates.
    if let Some(b) = node.bounds {
        let local = NSRect::new(
            NSPoint::new(b.x0 as f64, b.y0 as f64),
            NSSize::new((b.x1 - b.x0) as f64, (b.y1 - b.y0) as f64),
        );
        let screen = UIAccessibilityConvertFrameToScreenCoordinates(local, container);
        let _: () = msg_send![el, setAccessibilityFrame: screen];
    }
}
