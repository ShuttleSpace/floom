//! OpenHarmony accessibility backend.
//!
//! This is the OHOS counterpart of the desktop a11y backends
//! (`backend_nsaccessibility` / `backend_atspi` / `backend_uia`) and of the
//! iOS `backend_ios`. Like those, it implements [`AccessibilityBackend`] so
//! Floem core can hand it diffable [`AccessTreeUpdate`]s through
//! [`AccessibilityBackend::update_tree`].
//!
//! # Bridge status — IMPORTANT (#113 / #121)
//!
//! The current implementation serializes the neutral [`AccessTree`] into a JS
//! object and pushes it to ArkTS via `setAccessibilityTree` (the same shared
//! N-API registration used by the clipboard / file / external-url /
//! receive-files bridges — see `crate::platform::ohos_bridge`). That push is a
//! **debug/data feed only**: it does NOT expose the tree to the screen reader.
//!
//! OpenHarmony exposes a screen-reader tree to a native (winit) window ONLY
//! through the **native C** accessibility API
//! (`native_interface_accessibility.h`: `ArkUI_AccessibilityProvider` +
//! `ArkUI_AccessibilityElementInfo` + `ArkUI_Accessibility_ActionType`,
//! registered via `OH_ArkUI_AccessibilityProviderRegisterCallback`). ArkTS /
//! `@kit.AccessibilityKit` has no `registerProvider` and cannot register a
//! provider at all. Therefore the real exposure must be done from Rust by
//! calling that C API directly — this is tracked as task #121. Until then
//! `capability.accessibility` is claimed `true` but the OHOS screen reader is
//! NOT actually reachable.
//!
//! Actions flow back the other way: when the (future native) provider invokes
//! an action on a node, it should call [`on_accessibility_action`], which maps
//! the action name back to [`AccessAction`] and pushes it onto a channel that
//! [`AccessibilityBackend::poll_actions`] drains each frame.

#![cfg(target_env = "ohos")]

use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};

use napi::bindgen_prelude::JsFunction;
use napi::{Env, JsObject, Result as NapiResult};
use napi_derive::napi;

use crate::accessibility::{
    AccessAction, AccessNode, AccessNodeId, AccessRole, AccessTree, AccessTreeUpdate,
    AccessibilityBackend, PendingAction,
};
use crate::platform::ohos_bridge::call_bridge;

// Native C (ArkUI) accessibility provider — the real screen-reader path (#121).
mod backend_ohos_native;

// Entry points the embedder (winit fork) calls once it has a native provider
// pointer. The XComponent route (`register_native_provider_for_xcomponent`,
// exposed via the safe [`attach_xcomponent_provider`] wrapper) is the one wired
// by #122 (Route A): the fork already holds the `OH_NativeXComponent*`. The
// custom-node route (`try_create_and_register_provider`) is kept as a fallback.
#[allow(unused_imports)]
pub(crate) use backend_ohos_native::{
    register_native_provider, register_native_provider_for_xcomponent, try_create_and_register_provider,
};

/// Called once at window creation with the XComponent pointer obtained from
/// `floem_winit_ohos::WindowExtOhos::xcomponent_ptr`. Registers Floem's
/// accessibility callbacks on the XComponent's native provider so the OHOS
/// screen reader can traverse the neutral tree (#122, Route A).
///
/// Safe wrapper around the `unsafe` FFI registration. A null pointer (e.g.
/// surface not yet created) is a no-op.
#[cfg(target_env = "ohos")]
pub(crate) fn attach_xcomponent_provider(xcomponent: *mut std::ffi::c_void) {
    if xcomponent.is_null() {
        return;
    }
    unsafe {
        backend_ohos_native::register_native_provider_for_xcomponent(xcomponent);
    }
}

// ─────────────────────────────────────────────────────────────────────
// Reverse channel: OHOS screen-reader actions → Floem core.
//
// OHOS apps are effectively single-window, so a single global action sender
// is sufficient. The first backend instance registers its sender; any later
// instance (a second window) keeps the existing global and simply never
// receives actions — acceptable until multi-window OHOS support is needed.
// ─────────────────────────────────────────────────────────────────────
pub(crate) static ACTION_TX: OnceLock<Mutex<mpsc::Sender<PendingAction>>> = OnceLock::new();

/// Called by ArkTS when the OHOS accessibility service performs an action on a
/// node (e.g. a VoiceAssistant "activate"). Maps the string action name back
/// to the neutral [`AccessAction`] and forwards it to Floem core.
// TODO(OHOS): verify the `#[napi]` import path and that `cargo-ohos-app`
// registers this function alongside `register_platform_bridge`.
// `node_id` is `i64` at the boundary: ArkTS numbers are f64 and napi reads them
// as `i64`; napi 2.x implements `FromNapiValue` for `i64` but NOT `u64`, so the
// `AccessNodeId` (a `u64`) is reconstructed via `as u64` here. This matches the
// serialize side, which writes ids with `create_int64(id.0 as i64)`.
#[napi]
pub fn on_accessibility_action(node_id: i64, action: String, payload: Option<String>) {
    let Some(tx) = ACTION_TX.get() else {
        return;
    };
    let Ok(tx) = tx.lock() else {
        return;
    };
    let Some(action) = action_from_name(&action) else {
        return;
    };
    let _ = tx.send((AccessNodeId(node_id as u64), action, payload));
}

/// Serializes Floem's neutral accessibility tree and pushes it to ArkTS via the
/// shared N-API platform bridge (`setAccessibilityTree`). NOTE (#113/#121):
/// this is a debug/data feed only — the tree is NOT exposed to the screen
/// reader from ArkTS. The real exposure is the native C ArkUI_Accessibility
/// Provider (task #121).
pub(crate) struct OhosAccessibilityBackend {
    tree: AccessTree,
    action_rx: mpsc::Receiver<PendingAction>,
}

impl OhosAccessibilityBackend {
    /// Build the OHOS a11y backend. Returns `None` only if the action channel
    /// cannot be created (practically never).
    pub(crate) fn new() -> Option<Self> {
        let tree = AccessTree::new();
        let (tx, rx) = mpsc::channel();
        // Register the single action sender; ignore a second registration.
        let _ = ACTION_TX.set(Mutex::new(tx));
        Some(Self {
            tree,
            action_rx: rx,
        })
    }

    /// Serialize the current tree and push it to ArkTS via the platform bridge.
    fn push(&self) {
        let tree = &self.tree;
        call_bridge(|env, bridge| -> NapiResult<()> {
            let obj = serialize_tree(env, tree)?;
            let func: JsFunction = bridge.get_named_property("setAccessibilityTree")?;
            // `JsFunction::call<V>(this: Option<&JsObject>, args: &[V])`: pass the
            // serialized tree as the single argument via `JsUnknown` (which is
            // `NapiRaw`). The ArkTS return value is ignored.
            func.call(None, &[obj.into_unknown()])?;
            Ok(())
        });
    }
}

impl AccessibilityBackend for OhosAccessibilityBackend {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        self.tree.apply(update);
        // Feed the neutral tree to the native C provider (the real screen-reader
        // path, #121). The snapshot is read by the OHOS a11y thread callbacks.
        backend_ohos_native::update_snapshot(&self.tree);
        backend_ohos_native::notify_content_update();
        // Also push the tree to ArkTS as a debug/data feed (see module docs).
        self.push();
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        self.tree.focus
    }

    fn node_at_point(&self, point: peniko::kurbo::Point) -> Option<AccessNodeId> {
        self.tree.node_at_point(point)
    }

    fn poll_actions(&mut self) -> Vec<PendingAction> {
        self.action_rx.try_recv().into_iter().collect()
    }

    // `update_focus_state`: the neutral tree already carries the focused node
    // id, so no separate window-focus N-API call is needed for OHOS.
}

/// Serialize the neutral [`AccessTree`] into a JS object of the shape:
///
/// ```text
/// {
///   root:   number | null,
///   focus:  number | null,
///   nodes:  Array<{
///     id, role, label, value, description,
///     states: number,            // AccessState bitmask
///     actions: string[],         // AccessAction names
///     bounds: { x0, y0, x1, y1 } | null,
///     children: number[],
///   }>
/// }
/// ```
///
/// `null` is used for optional/absent fields so the ArkTS side has a stable
/// schema.
fn serialize_tree(env: Env, tree: &AccessTree) -> NapiResult<JsObject> {
    let mut obj = env.create_object()?;
    match tree.root {
        Some(id) => obj.set_named_property("root", env.create_int64(id.0 as i64)?)?,
        None => obj.set_named_property("root", env.get_null()?)?,
    }
    match tree.focus {
        Some(id) => obj.set_named_property("focus", env.create_int64(id.0 as i64)?)?,
        None => obj.set_named_property("focus", env.get_null()?)?,
    }

    let mut nodes = env.create_array_with_length(tree.len())?;
    let mut i = 0u32;
    for node in tree.iter() {
        let n = serialize_node(env, node)?;
        nodes.set_element(i, n)?;
        i += 1;
    }
    obj.set_named_property("nodes", nodes)?;
    Ok(obj)
}

fn serialize_node(env: Env, node: &AccessNode) -> NapiResult<JsObject> {
    let mut n = env.create_object()?;
    n.set_named_property("id", env.create_int64(node.id.0 as i64)?)?;
    n.set_named_property("role", env.create_string(role_name(node.role))?)?;

    match &node.label {
        Some(s) => n.set_named_property("label", env.create_string(s)?)?,
        None => n.set_named_property("label", env.get_null()?)?,
    }
    match &node.value {
        Some(s) => n.set_named_property("value", env.create_string(s)?)?,
        None => n.set_named_property("value", env.get_null()?)?,
    }
    match &node.description {
        Some(s) => n.set_named_property("description", env.create_string(s)?)?,
        None => n.set_named_property("description", env.get_null()?)?,
    }

    n.set_named_property("states", env.create_int64(node.states.bits() as i64)?)?;

    let mut actions = env.create_array_with_length(node.actions.len())?;
    for (j, a) in node.actions.iter().enumerate() {
        actions.set_element(j as u32, env.create_string(action_name(*a))?)?;
    }
    n.set_named_property("actions", actions)?;

    match node.bounds {
        Some(r) => {
            let mut b = env.create_object()?;
            b.set_named_property("x0", env.create_double(r.x0)?)?;
            b.set_named_property("y0", env.create_double(r.y0)?)?;
            b.set_named_property("x1", env.create_double(r.x1)?)?;
            b.set_named_property("y1", env.create_double(r.y1)?)?;
            n.set_named_property("bounds", b)?;
        }
        None => n.set_named_property("bounds", env.get_null()?)?,
    }

    let mut children = env.create_array_with_length(node.children.len())?;
    for (j, c) in node.children.iter().enumerate() {
        children.set_element(j as u32, env.create_int64(c.0 as i64)?)?;
    }
    n.set_named_property("children", children)?;

    Ok(n)
}

/// Map a neutral [`AccessRole`] to a stable, human-readable name string that
/// the ArkTS side can map onto OHOS accessibility role constants.
pub(crate) fn role_name(role: AccessRole) -> &'static str {
    match role {
        AccessRole::Unknown => "unknown",
        AccessRole::None => "none",
        AccessRole::Window => "window",
        AccessRole::Application => "application",
        AccessRole::Dialog => "dialog",
        AccessRole::Group => "group",
        AccessRole::Panel => "panel",
        AccessRole::Divider => "divider",
        AccessRole::Text => "text",
        AccessRole::Heading => "heading",
        AccessRole::Paragraph => "paragraph",
        AccessRole::Link => "link",
        AccessRole::Image => "image",
        AccessRole::Button => "button",
        AccessRole::CheckBox => "checkbox",
        AccessRole::RadioButton => "radioButton",
        AccessRole::Switch => "switch",
        AccessRole::Slider => "slider",
        AccessRole::SpinButton => "spinButton",
        AccessRole::ProgressBar => "progressBar",
        AccessRole::TextField => "textField",
        AccessRole::SearchField => "searchField",
        AccessRole::List => "list",
        AccessRole::ListItem => "listItem",
        AccessRole::Grid => "grid",
        AccessRole::Cell => "cell",
        AccessRole::Tab => "tab",
        AccessRole::TabList => "tabList",
        AccessRole::TabPanel => "tabPanel",
        AccessRole::Menu => "menu",
        AccessRole::MenuBar => "menuBar",
        AccessRole::MenuItem => "menuItem",
        AccessRole::ScrollView => "scrollView",
        AccessRole::ScrollBar => "scrollBar",
        AccessRole::Alert => "alert",
        AccessRole::Status => "status",
        AccessRole::Tooltip => "tooltip",
    }
}

/// Map a neutral [`AccessAction`] to the name string ArkTS sends back. Must
/// stay in sync with [`action_from_name`].
fn action_name(action: AccessAction) -> &'static str {
    match action {
        AccessAction::Focus => "focus",
        AccessAction::Default => "default",
        AccessAction::Increment => "increment",
        AccessAction::Decrement => "decrement",
        AccessAction::ShowContextMenu => "showContextMenu",
        AccessAction::ScrollIntoView => "scrollIntoView",
        AccessAction::Expand => "expand",
        AccessAction::Collapse => "collapse",
        AccessAction::Select => "select",
        AccessAction::SetValue => "setValue",
    }
}

/// Inverse of [`action_name`]. Returns `None` for unknown action names so the
/// backend does not propagate a bogus action into Floem core.
fn action_from_name(name: &str) -> Option<AccessAction> {
    Some(match name {
        "focus" => AccessAction::Focus,
        "default" => AccessAction::Default,
        "increment" => AccessAction::Increment,
        "decrement" => AccessAction::Decrement,
        "showContextMenu" => AccessAction::ShowContextMenu,
        "scrollIntoView" => AccessAction::ScrollIntoView,
        "expand" => AccessAction::Expand,
        "collapse" => AccessAction::Collapse,
        "select" => AccessAction::Select,
        "setValue" => AccessAction::SetValue,
        _ => return None,
    })
}
