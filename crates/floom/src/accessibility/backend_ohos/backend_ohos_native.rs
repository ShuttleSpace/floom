//! OpenHarmony **native C** accessibility provider — the real screen-reader
//! exposure path (#121).
//!
//! This module binds the C API declared in
//! `arkui/native_interface_accessibility.h` (shipped with the OHOS NDK,
//! implemented by `libace_ndk.z.so`) and registers an
//! `ArkUI_AccessibilityProvider` whose callbacks serve Floom's neutral
//! `AccessTree` to the OpenHarmony accessibility service.
//!
//! # Why native C (not ArkTS) (#113)
//!
//! `@kit.AccessibilityKit` / ArkTS has **no** `registerProvider` and cannot
//! register an accessibility provider at all. The only way to expose a screen
//! reader tree to a native (winit) window is this native C provider. See
//! `backend_ohos.rs` for the full root-cause write-up.
//!
//! # Threading model
//!
//! The OpenHarmony accessibility service invokes the registered callbacks on
//! its **own system thread**, not Floom's main thread. We therefore keep a
//! thread-safe [`Snapshot`] of the current `AccessTree` (plus a bidirectional
//! id map between Floom `AccessNodeId` and the `int32` ids the C API requires)
//! behind a global `Mutex`. `update_snapshot` (called from
//! [`crate::accessibility::backend_ohos::OhosAccessibilityBackend::update_tree`]
//! on the main thread) writes it; the callbacks read it.
//!
//! # Provider source / remaining integration gap (#122)
//!
//! This header only **registers** callbacks on an existing
//! `ArkUI_AccessibilityProvider*`; it provides no constructor. The provider is
//! obtained from an ArkUI custom node via
//! `OH_ArkUI_NativeModule_GetNativeAccessibilityProvider`
//! (API 23, requires an `ARKUI_NODE_CUSTOM` handle). For a winit native window
//! that custom node must be mounted into the window's ArkUI tree — that glue
//! lives in the embedder / winit fork and is tracked as #122. Until then the
//! screen reader is not reachable even though this module is correct;
//! [`register_native_provider`] is the entry point the embedder calls once it
//! has a provider pointer.

#![cfg(target_env = "ohos")]
#![allow(non_snake_case, non_camel_case_types, clippy::missing_safety_doc)]
#![allow(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::os::raw::c_double;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::accessibility::{
    AccessAction, AccessNode, AccessNodeId, AccessRole, AccessState, AccessTree,
};

// ─────────────────────────────────────────────────────────────────────
// Link against the OHOS accessibility implementation.
// ─────────────────────────────────────────────────────────────────────
#[link(name = "ace_ndk.z")]
unsafe extern "C" {
    // Provider registration.
    pub fn OH_ArkUI_AccessibilityProviderRegisterCallback(
        provider: *mut ArkUI_AccessibilityProvider,
        callbacks: *mut ArkUI_AccessibilityProviderCallbacks,
    ) -> c_int;

    // Element info lifecycle.
    pub fn OH_ArkUI_CreateAccessibilityElementInfo() -> *mut ArkUI_AccessibilityElementInfo;
    pub fn OH_ArkUI_DestoryAccessibilityElementInfo(info: *mut ArkUI_AccessibilityElementInfo);
    pub fn OH_ArkUI_AddAndGetAccessibilityElementInfo(
        list: *mut ArkUI_AccessibilityElementInfoList,
    ) -> *mut ArkUI_AccessibilityElementInfo;

    // Element info setters.
    pub fn OH_ArkUI_AccessibilityElementInfoSetElementId(
        info: *mut ArkUI_AccessibilityElementInfo,
        elementId: c_int,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetParentId(
        info: *mut ArkUI_AccessibilityElementInfo,
        parentId: c_int,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetComponentType(
        info: *mut ArkUI_AccessibilityElementInfo,
        componentType: *const c_char,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetContents(
        info: *mut ArkUI_AccessibilityElementInfo,
        contents: *const c_char,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetAccessibilityText(
        info: *mut ArkUI_AccessibilityElementInfo,
        accessibilityText: *const c_char,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetAccessibilityDescription(
        info: *mut ArkUI_AccessibilityElementInfo,
        accessibilityDescription: *const c_char,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetChildNodeIds(
        info: *mut ArkUI_AccessibilityElementInfo,
        childCount: c_int,
        childNodeIds: *const i64,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetOperationActions(
        info: *mut ArkUI_AccessibilityElementInfo,
        operationCount: c_int,
        operationActions: *const ArkUI_AccessibleAction,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetScreenRect(
        info: *mut ArkUI_AccessibilityElementInfo,
        screenRect: *const ArkUI_AccessibleRect,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetCheckable(
        info: *mut ArkUI_AccessibilityElementInfo,
        checkable: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetChecked(
        info: *mut ArkUI_AccessibilityElementInfo,
        checked: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetFocusable(
        info: *mut ArkUI_AccessibilityElementInfo,
        focusable: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetFocused(
        info: *mut ArkUI_AccessibilityElementInfo,
        isFocused: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetVisible(
        info: *mut ArkUI_AccessibilityElementInfo,
        isVisible: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetAccessibilityFocused(
        info: *mut ArkUI_AccessibilityElementInfo,
        accessibilityFocused: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetSelected(
        info: *mut ArkUI_AccessibilityElementInfo,
        selected: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetClickable(
        info: *mut ArkUI_AccessibilityElementInfo,
        clickable: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetLongClickable(
        info: *mut ArkUI_AccessibilityElementInfo,
        longClickable: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetEnabled(
        info: *mut ArkUI_AccessibilityElementInfo,
        isEnabled: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetScrollable(
        info: *mut ArkUI_AccessibilityElementInfo,
        scrollable: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetEditable(
        info: *mut ArkUI_AccessibilityElementInfo,
        editable: bool,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityElementInfoSetRangeInfo(
        info: *mut ArkUI_AccessibilityElementInfo,
        rangeInfo: *const ArkUI_AccessibleRangeInfo,
    ) -> c_int;

    // Event reporting.
    pub fn OH_ArkUI_CreateAccessibilityEventInfo() -> *mut ArkUI_AccessibilityEventInfo;
    pub fn OH_ArkUI_DestoryAccessibilityEventInfo(info: *mut ArkUI_AccessibilityEventInfo);
    pub fn OH_ArkUI_AccessibilityEventSetEventType(
        info: *mut ArkUI_AccessibilityEventInfo,
        eventType: c_int,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityEventSetRequestFocusId(
        info: *mut ArkUI_AccessibilityEventInfo,
        requestFocusId: c_int,
    ) -> c_int;
    pub fn OH_ArkUI_AccessibilityEventSetElementInfo(
        info: *mut ArkUI_AccessibilityEventInfo,
        elementInfo: *mut ArkUI_AccessibilityElementInfo,
    ) -> c_int;
    pub fn OH_ArkUI_SendAccessibilityAsyncEvent(
        provider: *mut ArkUI_AccessibilityProvider,
        eventInfo: *mut ArkUI_AccessibilityEventInfo,
        callback: Option<unsafe extern "C" fn(errorCode: c_int)>,
    );

    // Action argument lookup.
    pub fn OH_ArkUI_FindAccessibilityActionArgumentByKey(
        arguments: *mut ArkUI_AccessibilityActionArguments,
        key: *const c_char,
        value: *mut *mut c_char,
    ) -> c_int;

    // Provider source (requires an ArkUI custom node; API 23). Route B — kept
    // as a fallback behind `try_create_and_register_provider`. See #122.
    pub fn OH_ArkUI_NativeModule_GetNativeAccessibilityProvider(
        node: *mut *mut ArkUI_Node,
        provider: *mut *mut ArkUI_AccessibilityProvider,
    ) -> c_int;
    pub fn OH_ArkUI_CreateNode(nodeType: c_int) -> *mut ArkUI_Node;

    // Provider source straight from the XComponent (API 13). Route A — the
    // minimal, broadly-supported path used by #122: the fork already holds the
    // `OH_NativeXComponent*` (via `WindowExtOhos::xcomponent_ptr`), so no custom
    // ArkUI node or window-tree mount is required.
    pub fn OH_NativeXComponent_GetNativeAccessibilityProvider(
        component: *mut OH_NativeXComponent,
        handle: *mut *mut ArkUI_AccessibilityProvider,
    ) -> c_int;
}

// ─────────────────────────────────────────────────────────────────────
// Opaque C types.
// ─────────────────────────────────────────────────────────────────────
pub enum ArkUI_AccessibilityProvider {}
pub enum ArkUI_AccessibilityElementInfo {}
pub enum ArkUI_AccessibilityElementInfoList {}
pub enum ArkUI_AccessibilityEventInfo {}
pub enum ArkUI_AccessibilityActionArguments {}
pub enum ArkUI_Node {}

/// Opaque OHOS XComponent (from the NDK `ace/xcomponent/native_interface_xcomponent.h`).
/// `floom-winit-ohos` hands us this as a raw `*mut c_void` via
/// `WindowExtOhos::xcomponent_ptr`; we cast it back here to call the NDK.
pub enum OH_NativeXComponent {}

// ─────────────────────────────────────────────────────────────────────
// Value structs (must match the C `#[repr(C)]` layout).
// ─────────────────────────────────────────────────────────────────────
#[repr(C)]
pub struct ArkUI_AccessibleRect {
    pub leftTopX: c_int,
    pub leftTopY: c_int,
    pub rightBottomX: c_int,
    pub rightBottomY: c_int,
}

#[repr(C)]
pub struct ArkUI_AccessibleAction {
    pub actionType: c_int,
    pub description: *const c_char,
}

#[repr(C)]
pub struct ArkUI_AccessibleRangeInfo {
    pub min: c_double,
    pub max: c_double,
    pub current: c_double,
}

#[repr(C)]
pub struct ArkUI_AccessibilityProviderCallbacks {
    pub find_accessibility_node_infos_by_id:
        Option<unsafe extern "C" fn(elementId: i64, mode: c_int, requestId: c_int, list: *mut ArkUI_AccessibilityElementInfoList) -> c_int>,
    pub find_accessibility_node_infos_by_text:
        Option<unsafe extern "C" fn(elementId: i64, text: *const c_char, requestId: c_int, list: *mut ArkUI_AccessibilityElementInfoList) -> c_int>,
    pub find_focused_accessibility_node:
        Option<unsafe extern "C" fn(elementId: i64, focusType: c_int, requestId: c_int, info: *mut ArkUI_AccessibilityElementInfo) -> c_int>,
    pub find_next_focus_accessibility_node:
        Option<unsafe extern "C" fn(elementId: i64, direction: c_int, requestId: c_int, info: *mut ArkUI_AccessibilityElementInfo) -> c_int>,
    pub execute_accessibility_action:
        Option<unsafe extern "C" fn(elementId: i64, action: c_int, arguments: *mut ArkUI_AccessibilityActionArguments, requestId: c_int) -> c_int>,
    pub clear_focused_focus_accessibility_node: Option<unsafe extern "C" fn() -> c_int>,
    pub get_accessibility_node_cursor_position:
        Option<unsafe extern "C" fn(elementId: i64, requestId: c_int, index: *mut c_int) -> c_int>,
}

// ─────────────────────────────────────────────────────────────────────
// Constant discriminants (C enums are `int` → `c_int`).
// ─────────────────────────────────────────────────────────────────────
const RESULT_SUCCESSFUL: c_int = 0;
const RESULT_FAILED: c_int = -1;

// ArkUI_Accessibility_ActionType
const ACTION_CLICK: c_int = 0x10;
const ACTION_LONG_CLICK: c_int = 0x20;
const ACTION_GAIN_ACCESSIBILITY_FOCUS: c_int = 0x40;
const ACTION_CLEAR_ACCESSIBILITY_FOCUS: c_int = 0x80;
const ACTION_SCROLL_FORWARD: c_int = 0x100;
const ACTION_SCROLL_BACKWARD: c_int = 0x200;
const ACTION_SELECT_TEXT: c_int = 0x2000;
const ACTION_SET_TEXT: c_int = 0x4000;

// ArkUI_AccessibilitySearchMode
const SEARCH_MODE_CURRENT: c_int = 0;
const SEARCH_MODE_PREDECESSORS: c_int = 1 << 0;
const SEARCH_MODE_SIBLINGS: c_int = 1 << 1;
const SEARCH_MODE_CHILDREN: c_int = 1 << 2;
const SEARCH_MODE_RECURSIVE_CHILDREN: c_int = 1 << 3;

// ArkUI_AccessibilityFocusType
#[allow(dead_code)]
const FOCUS_TYPE_INPUT: c_int = 1 << 0;
#[allow(dead_code)]
const FOCUS_TYPE_ACCESSIBILITY: c_int = 1 << 1;

// ArkUI_AccessibilityFocusMoveDirection
const DIRECTION_FORWARD: c_int = 0x10;
const DIRECTION_BACKWARD: c_int = 0x20;

// ArkUI_AccessibilityEventType
const EVENT_PAGE_CONTENT_UPDATE: c_int = 0x800;

const ARKUI_NODE_CUSTOM: c_int = 0;

// ─────────────────────────────────────────────────────────────────────
// Thread-safe snapshot shared with the a11y service thread.
// ─────────────────────────────────────────────────────────────────────
struct Snapshot {
    tree: AccessTree,
    /// floom id → native `int32` id (stable across updates; never reused).
    to_native: HashMap<AccessNodeId, c_int>,
    /// native `int32` id → floom id.
    from_native: HashMap<c_int, AccessNodeId>,
    /// monotonic allocator for native ids.
    next_native: c_int,
    /// parent map (rebuilt on every update).
    parent: HashMap<AccessNodeId, AccessNodeId>,
}

static SNAPSHOT: OnceLock<Mutex<Snapshot>> = OnceLock::new();
static PROVIDER: AtomicPtr<ArkUI_AccessibilityProvider> = AtomicPtr::new(ptr::null_mut());
static CALLBACKS: OnceLock<ArkUI_AccessibilityProviderCallbacks> = OnceLock::new();

fn snapshot() -> &'static Mutex<Snapshot> {
    SNAPSHOT.get_or_init(|| {
        Mutex::new(Snapshot {
            tree: AccessTree::new(),
            to_native: HashMap::new(),
            from_native: HashMap::new(),
            next_native: 1,
            parent: HashMap::new(),
        })
    })
}

/// Replace the shared snapshot with a clone of the current tree, assigning
/// fresh native ids to any nodes not yet seen. Called from the main thread.
pub(crate) fn update_snapshot(tree: &AccessTree) {
    let mut snap = match snapshot().lock() {
        Ok(s) => s,
        Err(_) => return,
    };

    let mut parent = HashMap::new();
    for node in tree.iter() {
        for &child in &node.children {
            parent.insert(child, node.id);
        }
    }

    // Assign a stable native id to every node we have not mapped before.
    for node in tree.iter() {
        if !snap.to_native.contains_key(&node.id) {
            let id = snap.next_native;
            snap.next_native += 1;
            snap.to_native.insert(node.id, id);
            snap.from_native.insert(id, node.id);
        }
    }

    snap.tree = tree.clone();
    snap.parent = parent;
}

// ─────────────────────────────────────────────────────────────────────
// Node → C element info translation.
// ─────────────────────────────────────────────────────────────────────
fn collect_descendants(tree: &AccessTree, id: AccessNodeId, out: &mut Vec<AccessNodeId>) {
    if let Some(node) = tree.get(id) {
        for &child in &node.children {
            out.push(child);
            collect_descendants(tree, child, out);
        }
    }
}

/// Append one C element info (built from `node`) to `list`.
unsafe fn build_info(list: *mut ArkUI_AccessibilityElementInfoList, node: &AccessNode, snap: &Snapshot) {
    if list.is_null() {
        return;
    }
    let info = OH_ArkUI_AddAndGetAccessibilityElementInfo(list);
    if info.is_null() {
        return;
    }
    fill_info(info, node, snap);
}

unsafe fn fill_info(info: *mut ArkUI_AccessibilityElementInfo, node: &AccessNode, snap: &Snapshot) {
    let native = snap.to_native.get(&node.id).copied().unwrap_or(0);
    let _ = OH_ArkUI_AccessibilityElementInfoSetElementId(info, native);

    if let Some(&p) = snap.parent.get(&node.id) {
        if let Some(&pn) = snap.to_native.get(&p) {
            let _ = OH_ArkUI_AccessibilityElementInfoSetParentId(info, pn);
        }
    }

    let ctype = CString::new(super::role_name(node.role)).unwrap_or_default();
    let _ = OH_ArkUI_AccessibilityElementInfoSetComponentType(info, ctype.as_ptr());

    // Primary readable content: value first (text fields), else label.
    let content = node.value.as_deref().or(node.label.as_deref()).unwrap_or("");
    let c_content = CString::new(content).unwrap_or_default();
    let _ = OH_ArkUI_AccessibilityElementInfoSetContents(info, c_content.as_ptr());

    if let Some(label) = &node.label {
        let c = CString::new(label.as_str()).unwrap_or_default();
        let _ = OH_ArkUI_AccessibilityElementInfoSetAccessibilityText(info, c.as_ptr());
    }
    if let Some(desc) = &node.description {
        let c = CString::new(desc.as_str()).unwrap_or_default();
        let _ = OH_ArkUI_AccessibilityElementInfoSetAccessibilityDescription(info, c.as_ptr());
    }

    // Child node ids (native i32 → i64 array).
    if !node.children.is_empty() {
        let ids: Vec<i64> = node
            .children
            .iter()
            .map(|c| snap.to_native.get(c).copied().unwrap_or(0) as i64)
            .collect();
        let _ = OH_ArkUI_AccessibilityElementInfoSetChildNodeIds(
            info,
            ids.len() as c_int,
            ids.as_ptr(),
        );
    }

    // Advertised operations (action name + description).
    if !node.actions.is_empty() {
        let mut cstrs: Vec<CString> = Vec::new();
        let mut acts: Vec<ArkUI_AccessibleAction> = node
            .actions
            .iter()
            .map(|a| {
                let (at, desc) = action_to_native(*a);
                let c = CString::new(desc).unwrap_or_default();
                let ptr = c.as_ptr();
                cstrs.push(c);
                ArkUI_AccessibleAction {
                    actionType: at,
                    description: ptr,
                }
            })
            .collect();
        let _ = OH_ArkUI_AccessibilityElementInfoSetOperationActions(
            info,
            acts.len() as c_int,
            acts.as_ptr(),
        );
        // `cstrs` (and thus the description pointers) stay alive until here.
        let _ = &cstrs;
    }

    // Geometry (logical px → int32 screen rect).
    if let Some(r) = node.bounds {
        let rect = ArkUI_AccessibleRect {
            leftTopX: r.x0 as c_int,
            leftTopY: r.y0 as c_int,
            rightBottomX: r.x1 as c_int,
            rightBottomY: r.y1 as c_int,
        };
        let _ = OH_ArkUI_AccessibilityElementInfoSetScreenRect(info, &rect as *const _);
    }

    // States.
    let s = node.states;
    let checkable_role = matches!(
        node.role,
        AccessRole::CheckBox | AccessRole::RadioButton | AccessRole::Switch
    );
    let _ = OH_ArkUI_AccessibilityElementInfoSetCheckable(info, checkable_role);
    let _ = OH_ArkUI_AccessibilityElementInfoSetChecked(info, s.contains(AccessState::CHECKED));
    let _ = OH_ArkUI_AccessibilityElementInfoSetFocusable(info, true);
    let _ = OH_ArkUI_AccessibilityElementInfoSetFocused(info, s.contains(AccessState::FOCUSED));
    let _ = OH_ArkUI_AccessibilityElementInfoSetVisible(info, !s.contains(AccessState::HIDDEN));
    let _ = OH_ArkUI_AccessibilityElementInfoSetAccessibilityFocused(
        info,
        s.contains(AccessState::FOCUSED),
    );
    let _ = OH_ArkUI_AccessibilityElementInfoSetSelected(info, s.contains(AccessState::SELECTED));
    let clickable = matches!(node.role, AccessRole::Button) || node.actions.contains(&AccessAction::Default);
    let _ = OH_ArkUI_AccessibilityElementInfoSetClickable(info, clickable);
    let _ = OH_ArkUI_AccessibilityElementInfoSetEnabled(info, !s.contains(AccessState::DISABLED));
    let _ = OH_ArkUI_AccessibilityElementInfoSetLongClickable(
        info,
        node.actions.contains(&AccessAction::ShowContextMenu),
    );
    let scrollable = matches!(
        node.role,
        AccessRole::ScrollView | AccessRole::ScrollBar | AccessRole::List | AccessRole::Grid
    );
    let _ = OH_ArkUI_AccessibilityElementInfoSetScrollable(info, scrollable);
    let editable = matches!(node.role, AccessRole::TextField | AccessRole::SearchField);
    let _ = OH_ArkUI_AccessibilityElementInfoSetEditable(info, editable);

    // Range for value widgets (best-effort: min/max default to 0..1).
    if matches!(
        node.role,
        AccessRole::Slider | AccessRole::SpinButton | AccessRole::ProgressBar
    ) {
        let current: c_double = node
            .value
            .as_deref()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        let range = ArkUI_AccessibleRangeInfo {
            min: 0.0,
            max: 1.0,
            current,
        };
        let _ = OH_ArkUI_AccessibilityElementInfoSetRangeInfo(info, &range as *const _);
    }
}

fn action_to_native(action: AccessAction) -> (c_int, &'static str) {
    match action {
        AccessAction::Focus => (ACTION_GAIN_ACCESSIBILITY_FOCUS, "focus"),
        AccessAction::Default => (ACTION_CLICK, "default"),
        AccessAction::Increment => (ACTION_SCROLL_FORWARD, "increment"),
        AccessAction::Decrement => (ACTION_SCROLL_BACKWARD, "decrement"),
        AccessAction::ShowContextMenu => (ACTION_LONG_CLICK, "showContextMenu"),
        AccessAction::ScrollIntoView => (ACTION_SCROLL_FORWARD, "scrollIntoView"),
        // No dedicated native op for expand/collapse; CLICK is the closest verb.
        AccessAction::Expand => (ACTION_CLICK, "expand"),
        AccessAction::Collapse => (ACTION_CLICK, "collapse"),
        AccessAction::Select => (ACTION_SELECT_TEXT, "select"),
        AccessAction::SetValue => (ACTION_SET_TEXT, "setValue"),
    }
}

#[allow(dead_code)]
fn native_action_to_access(a: c_int) -> Option<AccessAction> {
    Some(match a {
        ACTION_CLICK => AccessAction::Default,
        ACTION_LONG_CLICK => AccessAction::ShowContextMenu,
        ACTION_GAIN_ACCESSIBILITY_FOCUS => AccessAction::Focus,
        ACTION_CLEAR_ACCESSIBILITY_FOCUS => AccessAction::Focus,
        ACTION_SCROLL_FORWARD => AccessAction::Increment,
        ACTION_SCROLL_BACKWARD => AccessAction::Decrement,
        ACTION_SELECT_TEXT => AccessAction::Select,
        ACTION_SET_TEXT => AccessAction::SetValue,
        _ => return None,
    })
}

// ─────────────────────────────────────────────────────────────────────
// Provider callbacks (run on the OHOS a11y service thread).
// ─────────────────────────────────────────────────────────────────────
unsafe extern "C" fn cb_find_by_id(
    elementId: i64,
    mode: c_int,
    _requestId: c_int,
    list: *mut ArkUI_AccessibilityElementInfoList,
) -> c_int {
    let Ok(snap) = snapshot().lock() else {
        return RESULT_FAILED;
    };
    let id = match snap.from_native.get(&(elementId as c_int)).copied() {
        Some(id) => id,
        None => return RESULT_FAILED,
    };

    let mut collected: Vec<AccessNodeId> = Vec::new();
    match mode {
        SEARCH_MODE_CURRENT => collected.push(id),
        SEARCH_MODE_CHILDREN => {
            if let Some(n) = snap.tree.get(id) {
                collected.extend(n.children.iter().copied());
            }
        }
        SEARCH_MODE_RECURSIVE_CHILDREN => collect_descendants(&snap.tree, id, &mut collected),
        SEARCH_MODE_PREDECESSORS => {
            let mut cur = id;
            while let Some(&p) = snap.parent.get(&cur) {
                collected.push(p);
                cur = p;
            }
        }
        SEARCH_MODE_SIBLINGS => {
            if let Some(&p) = snap.parent.get(&id) {
                if let Some(n) = snap.tree.get(p) {
                    for &c in &n.children {
                        if c != id {
                            collected.push(c);
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for nid in collected {
        if let Some(node) = snap.tree.get(nid) {
            build_info(list, node, &snap);
        }
    }
    RESULT_SUCCESSFUL
}

unsafe extern "C" fn cb_find_by_text(
    elementId: i64,
    text: *const c_char,
    _requestId: c_int,
    list: *mut ArkUI_AccessibilityElementInfoList,
) -> c_int {
    let needle = if text.is_null() {
        return RESULT_FAILED;
    } else {
        match CStr::from_ptr(text).to_str() {
            Ok(s) => s.to_ascii_lowercase(),
            Err(_) => return RESULT_FAILED,
        }
    };
    let Ok(snap) = snapshot().lock() else {
        return RESULT_FAILED;
    };
    // Anchor the search at the requested node's subtree when it is known.
    let mut scope: Vec<AccessNodeId> = Vec::new();
    match snap.from_native.get(&(elementId as c_int)).copied() {
        Some(id) => collect_descendants(&snap.tree, id, &mut scope),
        None => {
            for n in snap.tree.iter() {
                scope.push(n.id);
            }
        }
    }
    for nid in scope {
        if let Some(node) = snap.tree.get(nid) {
            let content = node
                .value
                .as_deref()
                .or(node.label.as_deref())
                .unwrap_or("")
                .to_ascii_lowercase();
            if content.contains(&needle) {
                build_info(list, node, &snap);
            }
        }
    }
    RESULT_SUCCESSFUL
}

unsafe extern "C" fn cb_find_focused(
    _elementId: i64,
    _focusType: c_int,
    _requestId: c_int,
    info: *mut ArkUI_AccessibilityElementInfo,
) -> c_int {
    if info.is_null() {
        return RESULT_FAILED;
    }
    let Ok(snap) = snapshot().lock() else {
        return RESULT_FAILED;
    };
    let id = match snap.tree.focus {
        Some(id) => id,
        None => return RESULT_FAILED,
    };
    match snap.tree.get(id) {
        Some(node) => {
            fill_info(info, node, &snap);
            RESULT_SUCCESSFUL
        }
        None => RESULT_FAILED,
    }
}

unsafe extern "C" fn cb_find_next_focus(
    elementId: i64,
    direction: c_int,
    _requestId: c_int,
    info: *mut ArkUI_AccessibilityElementInfo,
) -> c_int {
    if info.is_null() {
        return RESULT_FAILED;
    }
    // We have no spatial model; treat FORWARD/BACKWARD as next/prev in
    // pre-order traversal. Other directions are unsupported (best-effort).
    if direction != DIRECTION_FORWARD && direction != DIRECTION_BACKWARD {
        return RESULT_FAILED;
    }
    let Ok(snap) = snapshot().lock() else {
        return RESULT_FAILED;
    };
    let root = match snap.tree.root {
        Some(r) => r,
        None => return RESULT_FAILED,
    };
    let mut order: Vec<AccessNodeId> = Vec::new();
    collect_descendants(&snap.tree, root, &mut order);
    order.insert(0, root);
    let cur = snap.from_native.get(&(elementId as c_int)).copied().unwrap_or(root);
    let idx = order.iter().position(|&n| n == cur);
    let next = match (direction, idx) {
        (DIRECTION_FORWARD, Some(i)) if i + 1 < order.len() => Some(order[i + 1]),
        (DIRECTION_BACKWARD, Some(i)) if i > 0 => Some(order[i - 1]),
        _ => None,
    };
    match next.and_then(|n| snap.tree.get(n)) {
        Some(node) => {
            fill_info(info, node, &snap);
            RESULT_SUCCESSFUL
        }
        None => RESULT_FAILED,
    }
}

unsafe extern "C" fn cb_execute_action(
    elementId: i64,
    action: c_int,
    arguments: *mut ArkUI_AccessibilityActionArguments,
    _requestId: c_int,
) -> c_int {
    let native = elementId as c_int;
    let floom_id = {
        let Ok(snap) = snapshot().lock() else {
            return RESULT_FAILED;
        };
        match snap.from_native.get(&native).copied() {
            Some(id) => id,
            None => return RESULT_FAILED,
        }
    };

    let access_action = match native_action_to_access(action) {
        Some(a) => a,
        None => return RESULT_FAILED,
    };

    // Extract the SET_TEXT payload from the action arguments (owned by the
    // framework — do NOT free it).
    let mut payload: Option<String> = None;
    if action == ACTION_SET_TEXT && !arguments.is_null() {
        let mut val: *mut c_char = ptr::null_mut();
        let key = CString::new("text").unwrap_or_default();
        if OH_ArkUI_FindAccessibilityActionArgumentByKey(arguments, key.as_ptr(), &mut val)
            == RESULT_SUCCESSFUL
            && !val.is_null()
        {
            payload = CStr::from_ptr(val).to_str().ok().map(|s| s.to_string());
        }
    }

    if let Some(tx) = crate::accessibility::backend_ohos::ACTION_TX.get() {
        if let Ok(tx) = tx.lock() {
            let _ = tx.send((floom_id, access_action, payload));
        }
    }
    RESULT_SUCCESSFUL
}

unsafe extern "C" fn cb_clear_focus() -> c_int {
    // No neutral "clear focus" action exists; the embedder can extend this
    // later. Acknowledge success so the service does not retry.
    RESULT_SUCCESSFUL
}

unsafe extern "C" fn cb_cursor_position(
    _elementId: i64,
    _requestId: c_int,
    index: *mut c_int,
) -> c_int {
    if index.is_null() {
        return RESULT_FAILED;
    }
    // We do not track a caret position for text fields today.
    *index = 0;
    RESULT_SUCCESSFUL
}

// ─────────────────────────────────────────────────────────────────────
// Registration entry points.
// ─────────────────────────────────────────────────────────────────────
/// Register the Floom accessibility callbacks on an existing native provider.
///
/// `provider` must be a valid `ArkUI_AccessibilityProvider*` obtained by the
/// embedder (e.g. via [`try_create_and_register_provider`] / the custom-node
/// path described in #122). Returns `true` on success.
///
/// # Safety
/// `provider` must be a non-null provider pointer the framework still owns.
pub(crate) unsafe fn register_native_provider(provider: *mut ArkUI_AccessibilityProvider) -> bool {
    if provider.is_null() {
        return false;
    }
    let callbacks = CALLBACKS.get_or_init(|| ArkUI_AccessibilityProviderCallbacks {
        find_accessibility_node_infos_by_id: Some(cb_find_by_id),
        find_accessibility_node_infos_by_text: Some(cb_find_by_text),
        find_focused_accessibility_node: Some(cb_find_focused),
        find_next_focus_accessibility_node: Some(cb_find_next_focus),
        execute_accessibility_action: Some(cb_execute_action),
        clear_focused_focus_accessibility_node: Some(cb_clear_focus),
        get_accessibility_node_cursor_position: Some(cb_cursor_position),
    });
    let ret = OH_ArkUI_AccessibilityProviderRegisterCallback(
        provider,
        callbacks as *const _ as *mut _,
    );
    if ret == RESULT_SUCCESSFUL {
        PROVIDER.store(provider, Ordering::Release);
        true
    } else {
        false
    }
}

/// Convenience helper that creates an `ARKUI_NODE_CUSTOM` node and obtains its
/// provider, then registers Floom's callbacks.
///
/// **Integration gap (#122):** the created node is NOT mounted into the
/// window's ArkUI tree here — without a mounting point the accessibility
/// service will not discover it. The embedder (winit fork) must mount the node
/// and call [`register_native_provider`] with the live provider. This function
/// is provided for that wiring and is intentionally not auto-invoked.
#[allow(dead_code)]
pub(crate) unsafe fn try_create_and_register_provider() -> bool {
    let mut node = OH_ArkUI_CreateNode(ARKUI_NODE_CUSTOM);
    if node.is_null() {
        return false;
    }
    let mut provider: *mut ArkUI_AccessibilityProvider = ptr::null_mut();
    let ret = OH_ArkUI_NativeModule_GetNativeAccessibilityProvider(
        &mut node as *mut *mut ArkUI_Node,
        &mut provider as *mut *mut ArkUI_AccessibilityProvider,
    );
    if ret != RESULT_SUCCESSFUL || provider.is_null() {
        return false;
    }
    register_native_provider(provider)
}

/// Register Floom's accessibility callbacks on the XComponent's native
/// provider. This is the **#122 Route A** entry point: the embedder passes the
/// `OH_NativeXComponent*` it already holds (from
/// `floom_winit_ohos::WindowExtOhos::xcomponent_ptr`), and we obtain the
/// provider directly via `OH_NativeXComponent_GetNativeAccessibilityProvider`
/// (API 13) — no custom ArkUI node or window-tree mount needed. Returns `true`
/// on success.
///
/// # Safety
/// `component` must be a non-null `OH_NativeXComponent*` the framework owns for
/// the lifetime of the window (single-window OHOS).
pub(crate) unsafe fn register_native_provider_for_xcomponent(
    component: *mut c_void,
) -> bool {
    if component.is_null() {
        return false;
    }
    let mut provider: *mut ArkUI_AccessibilityProvider = ptr::null_mut();
    let ret = OH_NativeXComponent_GetNativeAccessibilityProvider(
        component as *mut OH_NativeXComponent,
        &mut provider as *mut *mut ArkUI_AccessibilityProvider,
    );
    if ret != RESULT_SUCCESSFUL || provider.is_null() {
        return false;
    }
    register_native_provider(provider)
}

/// Notify the accessibility service that the tree content changed, prompting
/// it to re-query. No-op until a provider is registered. Safe to call on every
/// `update_tree`.
pub(crate) fn notify_content_update() {
    let provider = PROVIDER.load(Ordering::Acquire);
    if provider.is_null() {
        return;
    }
    unsafe {
        let event = OH_ArkUI_CreateAccessibilityEventInfo();
        if event.is_null() {
            return;
        }
        let _ = OH_ArkUI_AccessibilityEventSetEventType(event, EVENT_PAGE_CONTENT_UPDATE);
        if let Ok(snap) = snapshot().lock() {
            if let Some(root) = snap.tree.root {
                if let Some(native_root) = snap.to_native.get(&root).copied() {
                    let elem = OH_ArkUI_CreateAccessibilityElementInfo();
                    if !elem.is_null() {
                        let _ = OH_ArkUI_AccessibilityElementInfoSetElementId(elem, native_root);
                        let _ = OH_ArkUI_AccessibilityEventSetElementInfo(event, elem);
                        OH_ArkUI_DestoryAccessibilityElementInfo(elem);
                    }
                }
            }
        }
        // The framework copies the event synchronously (the callback carries no
        // userdata, so it cannot return our pointer later) — safe to destroy.
        OH_ArkUI_SendAccessibilityAsyncEvent(provider, event, None);
        OH_ArkUI_DestoryAccessibilityEventInfo(event);
    }
}
