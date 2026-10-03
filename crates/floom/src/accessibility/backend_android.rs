//! Android accessibility backend (TalkBack).
//!
//! Unlike iOS (which subclasses `UIAccessibilityElement`) or the desktop
//! platforms (accesskit), Android exposes accessibility through an
//! [`android.view.accessibility.AccessibilityNodeProvider`] attached to a Java
//! `View`. Because that provider *must* be a Java object living in the view
//! hierarchy, this backend cannot be pure Rust: floom ships a tiny Java helper
//! (`FloomAccessibilityView.java`, package `dev.floom.android`) and injects an
//! instance of it into the Android `Activity`'s decor view from Rust (see
//! [`inject_a11y_view`]). The Java view is transparent, does not consume touch
//! events (so it never steals input from the rendering surface below it), and
//! is marked *not important for accessibility* — its only job is to vend an
//! `AccessibilityNodeProvider` whose callbacks are forwarded to the native
//! entry points in this module.
//!
//! Those native entry points read the shared neutral tree ([`ANDROID_A11Y`])
//! and translate it into `AccessibilityNodeInfo` objects on demand, and route
//! TalkBack actions (`ACTION_CLICK`, `ACTION_SET_TEXT`, `ACTION_SCROLL_*`, …)
//! back to Floom core through the **same** [`PendingAction`] channel the
//! desktop and iOS backends use.
//!
//! ## Tree sync
//!
//! `update_tree` receives a **full** neutral tree every frame (see
//! `bridge::build_access_tree_update`). It is applied to the shared cache; only
//! when the *topology* (root or any node's children) changes do we fire a
//! `TYPE_WINDOW_CONTENT_CHANGED` event on the host view so TalkBack re-requests
//! node infos — the per-node info is always rebuilt fresh on request, so
//! property changes (label/value/bounds/state) need no explicit notification.
//!
//! ## Safety
//!
//! Every native entry point reconstructs a `JNIEnv` from the raw pointer the
//! JVM passes, and converts any `Err` from the `jni` crate into a safe
//! null/empty/false return rather than panicking — a panic across the JNI
//! boundary would abort the app's JVM.

#![cfg(target_os = "android")]

use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::Mutex;

use jni::errors::Error;
use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::sys::{jboolean, jint, jobject, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use peniko::kurbo::{Point, Rect};

use crate::accessibility::{
    AccessAction, AccessNode, AccessNodeId, AccessRole, AccessState, AccessTree,
    AccessTreeUpdate, AccessibilityBackend, PendingAction,
};
use crate::platform::android_jni::AndroidJni;

/// The conventional virtual-view id for the host `View` itself (no node).
const HOST_VIEW_ID: jint = -1;

/// Cache of the `AccessibilityNodeInfo.ACTION_*` integer constants. Fetched
/// once (lazily, on first need) so we don't re-query the JVM on every node.
struct ActionConsts {
    click: jint,
    focus: jint,
    long_click: jint,
    scroll_forward: jint,
    scroll_backward: jint,
    increment: jint,
    decrement: jint,
    set_text: jint,
    expand: jint,
    collapse: jint,
    select: jint,
}

impl ActionConsts {
    fn load(env: &mut JNIEnv) -> Result<ActionConsts, Error> {
        let cls = "android/view/accessibility/AccessibilityNodeInfo";
        let mut c = |name: &str| -> Result<jint, Error> {
            Ok(env.get_static_field(cls, name, "I")?.i()?)
        };
        let click = c("ACTION_CLICK")?;
        let focus = c("ACTION_FOCUS")?;
        let long_click = c("ACTION_LONG_CLICK")?;
        let scroll_forward = c("ACTION_SCROLL_FORWARD")?;
        let scroll_backward = c("ACTION_SCROLL_BACKWARD")?;
        let increment = c("ACTION_INCREMENT")?;
        let decrement = c("ACTION_DECREMENT")?;
        let set_text = c("ACTION_SET_TEXT")?;
        let expand = c("ACTION_EXPAND")?;
        let collapse = c("ACTION_COLLAPSE")?;
        let select = c("ACTION_SELECT")?;
        Ok(ActionConsts {
            click,
            focus,
            long_click,
            scroll_forward,
            scroll_backward,
            increment,
            decrement,
            set_text,
            expand,
            collapse,
            select,
        })
    }
}

/// Shared, single-window a11y state. On Android there is exactly one Floom
/// window, so a module-level singleton (reachable from both the
/// `AccessibilityBackend` impl and the JNI entry points) is the natural shape.
struct AndroidA11yInner {
    tree: AccessTree,
    tx: mpsc::Sender<PendingAction>,
    rx: mpsc::Receiver<PendingAction>,
    /// Global ref to the Java `FloomAccessibilityView`, used to fire content
    /// changed events. `None` until the view is injected/registered.
    view: Option<GlobalRef>,
    root: Option<AccessNodeId>,
    children: HashMap<AccessNodeId, Vec<AccessNodeId>>,
    parent: HashMap<AccessNodeId, AccessNodeId>,
    actions: Option<ActionConsts>,
}

impl AndroidA11yInner {
    fn new(tx: mpsc::Sender<PendingAction>, rx: mpsc::Receiver<PendingAction>) -> Self {
        Self {
            tree: AccessTree::new(),
            tx,
            rx,
            view: None,
            root: None,
            children: HashMap::new(),
            parent: HashMap::new(),
            actions: None,
        }
    }
}

static ANDROID_A11Y: Mutex<Option<AndroidA11yInner>> = Mutex::new(None);

fn ensure_actions<'a>(
    inner: &'a mut AndroidA11yInner,
    env: &mut JNIEnv,
) -> Result<&'a ActionConsts, Error> {
    if inner.actions.is_none() {
        inner.actions = Some(ActionConsts::load(env)?);
    }
    Ok(inner.actions.as_ref().unwrap())
}

/// Android (TalkBack) accessibility backend.
///
/// A zero-sized marker that delegates all work to the shared [`ANDROID_A11Y`]
/// singleton, because the actual tree + action channel live there (so the JNI
/// entry points can reach them without a `self` pointer).
pub(crate) struct AndroidAccessibilityBackend;

impl AndroidAccessibilityBackend {
    /// Build the backend and (best-effort) inject the a11y host view into the
    /// running `Activity`. Returns `None` only if the singleton channel could
    /// not be created (never expected).
    pub(crate) fn new() -> Option<Self> {
        {
            let mut g = ANDROID_A11Y.lock().unwrap();
            if g.is_none() {
                let (tx, rx) = mpsc::channel();
                *g = Some(AndroidA11yInner::new(tx, rx));
            }
        }
        inject_a11y_view();
        Some(Self)
    }
}

impl AccessibilityBackend for AndroidAccessibilityBackend {
    fn update_tree(&mut self, update: AccessTreeUpdate) {
        let topology_changed = {
            let mut g = ANDROID_A11Y.lock().unwrap();
            let inner = match g.as_mut() {
                Some(i) => i,
                None => return,
            };
            inner.tree.apply(update);

            // Recompute the parent map (AccessTree stores only children).
            let mut parent = HashMap::new();
            for n in inner.tree.iter() {
                for c in &n.children {
                    parent.insert(*c, n.id);
                }
            }
            inner.parent = parent;

            let children_now: HashMap<AccessNodeId, Vec<AccessNodeId>> = inner
                .tree
                .iter()
                .map(|n| (n.id, n.children.clone()))
                .collect();
            let changed = inner.root != inner.tree.root || inner.children != children_now;
            if changed {
                inner.root = inner.tree.root;
                inner.children = children_now;
            }
            changed
        };
        if topology_changed {
            notify_changed();
        }
    }

    fn focused_node(&self) -> Option<AccessNodeId> {
        ANDROID_A11Y
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|i| i.tree.focus)
    }

    fn node_at_point(&self, point: Point) -> Option<AccessNodeId> {
        ANDROID_A11Y
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|i| i.tree.node_at_point(point))
    }

    fn poll_actions(&mut self) -> Vec<PendingAction> {
        let mut out = Vec::new();
        if let Some(inner) = ANDROID_A11Y.lock().unwrap().as_mut() {
            while let Ok(a) = inner.rx.try_recv() {
                out.push(a);
            }
        }
        out
    }
}

/// Fire `TYPE_WINDOW_CONTENT_CHANGED` on the host view (off the UI thread, via
/// `View.post`) so TalkBack re-requests node infos after a topology change.
fn notify_changed() {
    let view = ANDROID_A11Y
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|i| i.view.clone());
    if let Some(v) = view {
        AndroidJni::with_env(|env, _activity| {
            let _ = env.call_method(v.as_obj(), "requestA11yRefresh", "()V", &[]);
        });
    }
}

// ─────────────────────────────────────────────────────────────────────
// JNI entry points — called by `FloomAccessibilityView.java`.
//
// Symbol names follow the JNI auto-resolution convention
// `Java_<fully_qualified_class_with_underscores>_<method>`; the class is
// `dev.floom.android.FloomAccessibilityView`. Each function converts any
// `jni` error into a safe default (null / empty / false) instead of panicking.
// ─────────────────────────────────────────────────────────────────────

#[unsafe(no_mangle)]
pub extern "C" fn Java_dev_floom_android_FloomAccessibilityView_nativeRegisterView(
    env: *mut jni::sys::JNIEnv,
    this: jobject,
) {
    let mut env = match unsafe { JNIEnv::from_raw(env) } {
        Ok(e) => e,
        Err(_) => return,
    };
    let this = unsafe { JObject::from_raw(this) };
    let gref = match env.new_global_ref(this) {
        Ok(g) => g,
        Err(_) => return,
    };
    let mut g = ANDROID_A11Y.lock().unwrap();
    if let Some(inner) = g.as_mut() {
        inner.view = Some(gref);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_dev_floom_android_FloomAccessibilityView_nativeCreateNodeInfo(
    env: *mut jni::sys::JNIEnv,
    this: jobject,
    virtual_view_id: jint,
) -> jobject {
    let mut env = match unsafe { JNIEnv::from_raw(env) } {
        Ok(e) => e,
        Err(_) => return std::ptr::null_mut(),
    };
    let this = unsafe { JObject::from_raw(this) };
    let result = {
        let mut g = ANDROID_A11Y.lock().unwrap();
        match g.as_mut() {
            Some(inner) => create_node_info_locked(inner, &mut env, &this, virtual_view_id),
            None => Ok(None),
        }
    };
    match result {
        Ok(Some(info)) => info.into_raw(),
        _ => std::ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_dev_floom_android_FloomAccessibilityView_nativePerformAction(
    env: *mut jni::sys::JNIEnv,
    this: jobject,
    virtual_view_id: jint,
    action: jint,
    arguments: jobject,
) -> jboolean {
    let mut env = match unsafe { JNIEnv::from_raw(env) } {
        Ok(e) => e,
        Err(_) => return JNI_FALSE,
    };
    let this = unsafe { JObject::from_raw(this) };
    let args = unsafe { JObject::from_raw(arguments) };
    let result = {
        let mut g = ANDROID_A11Y.lock().unwrap();
        match g.as_mut() {
            Some(inner) => perform_action_locked(inner, &mut env, virtual_view_id, action, &args),
            None => Ok(false),
        }
    };
    if result.unwrap_or(false) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn Java_dev_floom_android_FloomAccessibilityView_nativeFindNodesByText(
    env: *mut jni::sys::JNIEnv,
    this: jobject,
    text: jni::sys::jstring,
    virtual_view_id: jint,
) -> jobject {
    let mut env = match unsafe { JNIEnv::from_raw(env) } {
        Ok(e) => e,
        Err(_) => return std::ptr::null_mut(),
    };
    let this = unsafe { JObject::from_raw(this) };
    let text = if text.is_null() {
        String::new()
    } else {
        let jtext = unsafe { JString::from_raw(text) };
        match env.get_string(&jtext) {
            Ok(s) => String::from(s),
            Err(_) => String::new(),
        }
    };
    let result = {
        let mut g = ANDROID_A11Y.lock().unwrap();
        match g.as_mut() {
            Some(inner) => find_nodes_locked(inner, &mut env, &this, &text, virtual_view_id),
            None => Ok(None),
        }
    };
    match result {
        Ok(Some(list)) => list.into_raw(),
        _ => std::ptr::null_mut(),
    }
}

// ─────────────────────────────────────────────────────────────────────
// Tree → AccessibilityNodeInfo translation.
// ─────────────────────────────────────────────────────────────────────

fn create_node_info_locked<'a>(
    inner: &mut AndroidA11yInner,
    env: &mut JNIEnv<'a>,
    this: &JObject,
    vid: jint,
) -> Result<Option<JObject<'a>>, Error> {
    if vid == HOST_VIEW_ID {
        return Ok(Some(build_host_info(inner, env, this)?));
    }
    let id = AccessNodeId(vid as u64);
    let node: AccessNode = match inner.tree.get(id) {
        Some(n) => n.clone(),
        None => return Ok(None),
    };

    ensure_actions(inner, env)?;
    let actions = inner.actions.as_ref().unwrap();

    let info = env
        .call_static_method(
            "android/view/accessibility/AccessibilityNodeInfo",
            "obtain",
            "(Landroid/view/View;I)Landroid/view/accessibility/AccessibilityNodeInfo;",
            &[JValue::Object(this), JValue::Int(vid)],
        )?
        .l()?;

    // Primary spoken text: label preferred, else value (e.g. text field content).
    if let Some(t) = node.label.clone().or(node.value.clone()) {
        set_charsequence(env, &info, "setText", &t)?;
    }
    if let Some(d) = &node.description {
        set_charsequence(env, &info, "setContentDescription", d)?;
    }

    // Class name drives TalkBack's speech pattern / sound.
    let cn = env.new_string(class_name_for(node.role))?;
    env.call_method(
        &info,
        "setClassName",
        "(Ljava/lang/CharSequence;)V",
        &[JValue::Object(&cn)],
    )?;

    // States.
    env.call_method(
        &info,
        "setEnabled",
        "(Z)V",
        &[JValue::from(!node.states.contains(AccessState::DISABLED))],
    )?;
    env.call_method(
        &info,
        "setSelected",
        "(Z)V",
        &[JValue::from(node.states.contains(AccessState::SELECTED))],
    )?;
    let checkable = matches!(
        node.role,
        AccessRole::CheckBox | AccessRole::RadioButton | AccessRole::Switch
    );
    env.call_method(&info, "setCheckable", "(Z)V", &[JValue::from(checkable)])?;
    if checkable {
        env.call_method(
            &info,
            "setChecked",
            "(Z)V",
            &[JValue::from(node.states.contains(AccessState::CHECKED))],
        )?;
    }

    // Geometry: window-local logical px → on-screen px.
    if let Some(b) = node.bounds {
        set_bounds_in_screen(env, this, &info, b)?;
    }

    // Parent: the root's parent is the host view; everything else points at its
    // computed parent.
    let parent_vid = match inner.parent.get(&id) {
        Some(p) => p.0 as jint,
        None => HOST_VIEW_ID,
    };
    env.call_method(
        &info,
        "setParent",
        "(Landroid/view/View;I)V",
        &[JValue::Object(this), JValue::Int(parent_vid)],
    )?;

    // Children.
    for c in &node.children {
        env.call_method(
            &info,
            "addChild",
            "(Landroid/view/View;I)V",
            &[JValue::Object(this), JValue::Int(c.0 as jint)],
        )?;
    }

    // Supported actions.
    for a in &node.actions {
        add_action(env, &info, actions, *a)?;
    }

    Ok(Some(info))
}

/// Build the host (container) node info for `HOST_VIEW_ID`, covering the whole
/// window and exposing the tree root as its single virtual child.
fn build_host_info<'a>(
    inner: &AndroidA11yInner,
    env: &mut JNIEnv<'a>,
    this: &JObject,
) -> Result<JObject<'a>, Error> {
    let info = env
        .call_static_method(
            "android/view/accessibility/AccessibilityNodeInfo",
            "obtain",
            "(Landroid/view/View;)Landroid/view/accessibility/AccessibilityNodeInfo;",
            &[JValue::Object(this)],
        )?
        .l()?;
    set_host_bounds(env, this, &info)?;
    if let Some(root) = inner.tree.root {
        env.call_method(
            &info,
            "addChild",
            "(Landroid/view/View;I)V",
            &[JValue::Object(this), JValue::Int(root.0 as jint)],
        )?;
    }
    Ok(info)
}

fn set_charsequence(env: &mut JNIEnv, obj: &JObject, method: &str, s: &str) -> Result<(), Error> {
    let j = env.new_string(s)?;
    env.call_method(obj, method, "(Ljava/lang/CharSequence;)V", &[JValue::Object(&j)])?;
    Ok(())
}

fn set_bounds_in_screen(
    env: &mut JNIEnv,
    this: &JObject,
    info: &JObject,
    b: Rect,
) -> Result<(), Error> {
    let (ox, oy, density) = view_offset_and_density(env, this)?;
    let l = (ox as f32 + b.x0 as f32 * density).round() as i32;
    let t = (oy as f32 + b.y0 as f32 * density).round() as i32;
    let r = (ox as f32 + b.x1 as f32 * density).round() as i32;
    let bo = (oy as f32 + b.y1 as f32 * density).round() as i32;
    let rect = env.new_object(
        "android/graphics/Rect",
        "(IIII)V",
        &[JValue::Int(l), JValue::Int(t), JValue::Int(r), JValue::Int(bo)],
    )?;
    env.call_method(
        info,
        "setBoundsInScreen",
        "(Landroid/graphics/Rect;)V",
        &[JValue::Object(&rect)],
    )?;
    Ok(())
}

fn set_host_bounds(env: &mut JNIEnv, this: &JObject, info: &JObject) -> Result<(), Error> {
    let (ox, oy, _density) = view_offset_and_density(env, this)?;
    let w = env.call_method(this, "getWidth", "()I", &[])?.i()?;
    let h = env.call_method(this, "getHeight", "()I", &[])?.i()?;
    let rect = env.new_object(
        "android/graphics/Rect",
        "(IIII)V",
        &[
            JValue::Int(ox),
            JValue::Int(oy),
            JValue::Int(ox + w),
            JValue::Int(oy + h),
        ],
    )?;
    env.call_method(
        info,
        "setBoundsInScreen",
        "(Landroid/graphics/Rect;)V",
        &[JValue::Object(&rect)],
    )?;
    Ok(())
}

/// View's top-left on screen (px) plus the display density, used to convert
/// window-local logical px into the screen px `AccessibilityNodeInfo` expects.
fn view_offset_and_density(env: &mut JNIEnv, this: &JObject) -> Result<(i32, i32, f32), Error> {
    let arr = env.new_int_array(2)?;
    env.call_method(this, "getLocationOnScreen", "([I)V", &[JValue::Object(&arr)])?;
    let mut loc = [0i32; 2];
    env.get_int_array_region(&arr, 0, &mut loc)?;
    let res = env
        .call_method(this, "getResources", "()Landroid/content/res/Resources;", &[])?
        .l()?;
    let dm = env
        .call_method(&res, "getDisplayMetrics", "()Landroid/util/DisplayMetrics;", &[])?
        .l()?;
    let density = env.get_field(&dm, "density", "F")?.f()?;
    Ok((loc[0], loc[1], density))
}

fn add_action(
    env: &mut JNIEnv,
    info: &JObject,
    c: &ActionConsts,
    a: AccessAction,
) -> Result<(), Error> {
    match a {
        AccessAction::Default => {
            env.call_method(info, "setClickable", "(Z)V", &[JValue::from(true)])?;
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.click)])?;
        }
        AccessAction::Focus => {
            env.call_method(info, "setFocusable", "(Z)V", &[JValue::from(true)])?;
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.focus)])?;
        }
        AccessAction::Increment => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.increment)])?;
        }
        AccessAction::Decrement => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.decrement)])?;
        }
        AccessAction::ScrollIntoView => {
            env.call_method(info, "setScrollable", "(Z)V", &[JValue::from(true)])?;
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.scroll_forward)])?;
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.scroll_backward)])?;
        }
        AccessAction::Expand => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.expand)])?;
        }
        AccessAction::Collapse => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.collapse)])?;
        }
        AccessAction::SetValue => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.set_text)])?;
        }
        AccessAction::Select => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.select)])?;
        }
        AccessAction::ShowContextMenu => {
            env.call_method(info, "addAction", "(I)V", &[JValue::Int(c.long_click)])?;
        }
    }
    Ok(())
}

fn perform_action_locked(
    inner: &mut AndroidA11yInner,
    env: &mut JNIEnv,
    vid: jint,
    action: jint,
    arguments: &JObject,
) -> Result<bool, Error> {
    let id = AccessNodeId(vid as u64);
    if inner.tree.get(id).is_none() {
        return Ok(false);
    }
    let c = ensure_actions(inner, env)?;
    let (a, payload) = if action == c.click {
        (AccessAction::Default, None)
    } else if action == c.long_click {
        (AccessAction::ShowContextMenu, None)
    } else if action == c.focus {
        (AccessAction::Focus, None)
    } else if action == c.scroll_forward || action == c.scroll_backward {
        (AccessAction::ScrollIntoView, None)
    } else if action == c.increment {
        (AccessAction::Increment, None)
    } else if action == c.decrement {
        (AccessAction::Decrement, None)
    } else if action == c.expand {
        (AccessAction::Expand, None)
    } else if action == c.collapse {
        (AccessAction::Collapse, None)
    } else if action == c.select {
        (AccessAction::Select, None)
    } else if action == c.set_text {
        let payload =
            read_bundle_string(env, arguments, "android.view.accessibility.extra.DATA_TEXT")?;
        (AccessAction::SetValue, payload)
    } else {
        return Ok(false);
    };
    let _ = inner.tx.send((id, a, payload));
    Ok(true)
}

fn read_bundle_string(
    env: &mut JNIEnv,
    bundle: &JObject,
    key: &str,
) -> Result<Option<String>, Error> {
    if bundle.as_raw().is_null() {
        return Ok(None);
    }
    let k = env.new_string(key)?;
    let v = env.call_method(
        bundle,
        "getString",
        "(Ljava/lang/String;)Ljava/lang/String;",
        &[JValue::Object(&k)],
    )?;
    let v = match v.l() {
        Ok(o) if !o.as_raw().is_null() => o,
        _ => return Ok(None),
    };
    let jstr = unsafe { JString::from_raw(v.as_raw() as jni::sys::jstring) };
    let s = env.get_string(&jstr)?;
    Ok(Some(String::from(s)))
}

fn find_nodes_locked<'a>(
    inner: &mut AndroidA11yInner,
    env: &mut JNIEnv<'a>,
    this: &JObject,
    text: &str,
    _vid: jint,
) -> Result<Option<JObject<'a>>, Error> {
    let list = env.new_object("java/util/ArrayList", "()V", &[])?;
    let lower = text.to_lowercase();
    // Collect matching ids first (immutable borrow), then build node infos
    // (mutable borrow of `inner`).
    let ids: Vec<AccessNodeId> = inner
        .tree
        .iter()
        .filter(|n| {
            let content = n
                .label
                .clone()
                .or_else(|| n.value.clone())
                .unwrap_or_default();
            let hay = format!(
                "{} {}",
                content,
                n.description.clone().unwrap_or_default()
            )
            .to_lowercase();
            hay.contains(&lower)
        })
        .map(|n| n.id)
        .collect();
    for id in ids {
        if let Ok(Some(info)) = create_node_info_locked(inner, env, this, id.0 as jint) {
            env.call_method(
                &list,
                "add",
                "(Ljava/lang/Object;)Z",
                &[JValue::Object(&info)],
            )?;
        }
    }
    Ok(Some(list))
}

/// Map a neutral role to the Android widget class name TalkBack uses for its
/// speech patterns. Containers fall back to `android.view.View`.
fn class_name_for(role: AccessRole) -> &'static str {
    match role {
        AccessRole::Button => "android.widget.Button",
        AccessRole::Link => "android.widget.TextView",
        AccessRole::Image => "android.widget.ImageView",
        AccessRole::Text
        | AccessRole::Heading
        | AccessRole::Paragraph
        | AccessRole::ListItem => "android.widget.TextView",
        AccessRole::TextField | AccessRole::SearchField => "android.widget.EditText",
        AccessRole::CheckBox => "android.widget.CheckBox",
        AccessRole::RadioButton => "android.widget.RadioButton",
        AccessRole::Switch => "android.widget.Switch",
        AccessRole::Slider | AccessRole::SpinButton => "android.widget.SeekBar",
        AccessRole::ProgressBar => "android.widget.ProgressBar",
        AccessRole::List => "android.widget.ListView",
        AccessRole::ScrollView => "android.widget.ScrollView",
        AccessRole::ScrollBar => "android.widget.ScrollBar",
        AccessRole::Tab => "android.widget.TabWidget",
        AccessRole::Dialog => "android.app.Dialog",
        AccessRole::Alert => "android.widget.Toast",
        _ => "android.view.View",
    }
}

/// Best-effort: inject a transparent, non-touch-consuming
/// `FloomAccessibilityView` into the running `Activity`'s decor view so
/// TalkBack can find an `AccessibilityNodeProvider`. If the app did not ship
/// the Java class (or the activity isn't ready), this silently no-ops — the
/// app still runs, it just won't expose an a11y tree.
fn inject_a11y_view() {
    AndroidJni::with_env(|env, activity| {
        // Already injected?
        {
            let g = ANDROID_A11Y.lock().unwrap();
            if g.as_ref().and_then(|i| i.view.as_ref()).is_some() {
                return;
            }
        }
        let view = match env.new_object(
            "dev/floom/android/FloomAccessibilityView",
            "(Landroid/content/Context;)V",
            &[JValue::Object(activity)],
        ) {
            Ok(v) => v,
            Err(_) => return,
        };
        let window = match env
            .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
            .and_then(|w| w.l())
        {
            Ok(w) => w,
            Err(_) => return,
        };
        let decor = match env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .and_then(|d| d.l())
        {
            Ok(d) => d,
            Err(_) => return,
        };
        let lp = match env.new_object(
            "android/view/ViewGroup$LayoutParams",
            "(II)V",
            &[JValue::Int(-1), JValue::Int(-1)],
        ) {
            Ok(l) => l,
            Err(_) => return,
        };
        let _ = env.call_method(
            &decor,
            "addView",
            "(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V",
            &[JValue::Object(&view), JValue::Object(&lp)],
        );
    });
}
