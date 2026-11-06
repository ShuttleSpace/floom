// SPDX-License-Identifier: MIT
//! Nodes, keys, slots and events — the structure half of the schema.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ui_ir::value::PropValue;

/// The slot every container has unless it says otherwise.
pub const DEFAULT_SLOT: &str = "default";

/// A native participant kind — mirror of `floom_types::NativeViewKind`.
///
/// The IR keeps its own copy so a document is readable by tools that never link
/// the compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NativeKind {
    Web,
    Video,
    Map,
    TextInput,
}

/// What a node is.
///
/// This is a closed vocabulary, not a `Custom(String)` escape hatch: an unknown
/// kind is a document that cannot be rendered, and failing at load is far
/// cheaper than discovering it when a widget materializes as nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    // Primitives
    Text,
    Icon,
    Image,
    Container,
    Spacer,
    Divider,
    // Controls
    Button,
    Checkbox,
    Radio,
    Switch,
    Slider,
    Progress,
    Select,
    TextField,
    // Composition
    Stack,
    Grid,
    List,
    Conditional,
    Overlay,
    Dialog,
    Popup,
    Tabs,
    NavigationBar,
    NavigationRail,
    NavigationDrawer,
    /// A platform-composited participant (Phase 16/17 vocabulary).
    Native(NativeKind),
}

impl NodeKind {
    /// Every kind, for exhaustive tests and table audits.
    pub const ALL: &'static [NodeKind] = &[
        Self::Text,
        Self::Icon,
        Self::Image,
        Self::Container,
        Self::Spacer,
        Self::Divider,
        Self::Button,
        Self::Checkbox,
        Self::Radio,
        Self::Switch,
        Self::Slider,
        Self::Progress,
        Self::Select,
        Self::TextField,
        Self::Stack,
        Self::Grid,
        Self::List,
        Self::Conditional,
        Self::Overlay,
        Self::Dialog,
        Self::Popup,
        Self::Tabs,
        Self::NavigationBar,
        Self::NavigationRail,
        Self::NavigationDrawer,
        Self::Native(NativeKind::Web),
        Self::Native(NativeKind::Video),
        Self::Native(NativeKind::Map),
        Self::Native(NativeKind::TextInput),
    ];

    /// Whether this kind can hold children.
    pub fn is_container(self) -> bool {
        matches!(
            self,
            Self::Stack
                | Self::Grid
                | Self::List
                | Self::Conditional
                | Self::Overlay
                | Self::Dialog
                | Self::Popup
                | Self::Tabs
                | Self::Container
                | Self::NavigationBar
                | Self::NavigationRail
                | Self::NavigationDrawer
        )
    }
}

impl fmt::Display for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Native(kind) => write!(f, "Native({kind:?})"),
            other => write!(f, "{other:?}"),
        }
    }
}

/// An event a node can emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventName {
    Click,
    Change,
    Input,
    Submit,
    Focus,
    Blur,
    Toggle,
    Scroll,
    Select,
    Dismiss,
    Hover,
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// A node's identity, stable across edits and reloads.
///
/// A key rather than an index: hot reload matches old and new nodes by key, so
/// state survives an edit that inserts a sibling. Indices would shift and every
/// following node would look new.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NodeKey(String);

impl NodeKey {
    /// A key is `segment(.segment)*`, each segment starting with a letter or
    /// `_` — dotted so a key reads as the path it usually is.
    pub fn is_valid(key: &str) -> bool {
        !key.is_empty()
            && key.split('.').all(|seg| {
                let mut chars = seg.chars();
                match chars.next() {
                    Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
                    _ => return false,
                }
                chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            })
    }

    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NodeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for NodeKey {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<String> for NodeKey {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// One property assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prop {
    pub name: String,
    pub value: PropValue,
}

impl Prop {
    pub fn new(name: impl Into<String>, value: PropValue) -> Self {
        Self {
            name: name.into(),
            value,
        }
    }
}

/// A named list of children.
///
/// Named rather than positional so a kind can have more than one (`Tabs` wants a
/// strip and a body), and so a lowering step can put a child somewhere specific
/// without counting positions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slot {
    pub name: String,
    pub children: Vec<NodeKey>,
}

impl Slot {
    pub fn new(name: impl Into<String>, children: Vec<NodeKey>) -> Self {
        Self {
            name: name.into(),
            children,
        }
    }

    pub fn default_slot(children: Vec<NodeKey>) -> Self {
        Self::new(DEFAULT_SLOT, children)
    }
}

/// An event wired to a handler.
///
/// There are two ways a handler arrives:
///
/// - *reference* — [`EventBinding::new`] ties the event to a named binding in the
///   document's binding table (`onClick={onPress}`). `handler` is that binding's
///   name and [`body`] is `None`.
/// - *inline* — [`EventBinding::inline`] carries the handler's Rust source
///   directly (`onClick={() => setCount(count() + 1)}`). The compiler lowers the
///   JS arrow into Rust, so there is no binding name to look up and [`body`] holds
///   the code. [`UiDocument::validate`](crate::ui_ir::UiDocument::validate) skips the binding lookup when [`body`] is
///   set.
///
/// [`body`]: EventBinding::body
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventBinding {
    pub event: EventName,
    /// The binding name for a *reference* handler, or empty for an *inline* one.
    pub handler: String,
    /// Inline Rust source for the handler body, set by the TSX compiler. `None`
    /// for a reference handler.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

impl EventBinding {
    /// Wire the event to a named binding (`onClick={onPress}`).
    pub fn new(event: EventName, handler: impl Into<String>) -> Self {
        Self {
            event,
            handler: handler.into(),
            body: None,
        }
    }

    /// Wire the event to an inline handler whose Rust body is already translated.
    pub fn inline(event: EventName, body: impl Into<String>) -> Self {
        Self {
            event,
            handler: String::new(),
            body: Some(body.into()),
        }
    }
}

/// One node of the document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiNode {
    pub key: NodeKey,
    pub kind: NodeKind,
    pub props: Vec<Prop>,
    pub slots: Vec<Slot>,
    pub events: Vec<EventBinding>,
}

impl UiNode {
    pub fn new(key: impl Into<NodeKey>, kind: NodeKind) -> Self {
        Self {
            key: key.into(),
            kind,
            props: Vec::new(),
            slots: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn with_props(mut self, props: impl IntoIterator<Item = Prop>) -> Self {
        self.props.extend(props);
        self
    }

    pub fn with_slot(mut self, slot: Slot) -> Self {
        self.slots.push(slot);
        self
    }

    pub fn with_events(mut self, events: impl IntoIterator<Item = EventBinding>) -> Self {
        self.events.extend(events);
        self
    }

    /// The value set for `name`, if any.
    pub fn prop(&self, name: &str) -> Option<&PropValue> {
        self.props.iter().find(|p| p.name == name).map(|p| &p.value)
    }

    /// Every child key, in slot order.
    pub fn children(&self) -> impl Iterator<Item = &NodeKey> {
        self.slots.iter().flat_map(|s| s.children.iter())
    }
}

/// The spelling rules a text front end shares with the IR.
///
/// `parse` and `name` must round-trip (`parse(name(k)) == Some(k)`), and a test
/// audits that for every kind: two tables — one for reading, one for writing —
/// is exactly the drift this pair exists to avoid.
fn normalize(word: &str) -> String {
    word.chars()
        .filter(|c| *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

impl NodeKind {
    /// The canonical spelling: the variant name in `snake_case`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Icon => "icon",
            Self::Image => "image",
            Self::Container => "container",
            Self::Spacer => "spacer",
            Self::Divider => "divider",
            Self::Button => "button",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Switch => "switch",
            Self::Slider => "slider",
            Self::Progress => "progress",
            Self::Select => "select",
            Self::TextField => "text_field",
            Self::Stack => "stack",
            Self::Grid => "grid",
            Self::List => "list",
            Self::Conditional => "if",
            Self::Overlay => "overlay",
            Self::Dialog => "dialog",
            Self::Popup => "popup",
            Self::Tabs => "tabs",
            Self::NavigationBar => "navigation_bar",
            Self::NavigationRail => "navigation_rail",
            Self::NavigationDrawer => "navigation_drawer",
            Self::Native(NativeKind::Web) => "web",
            Self::Native(NativeKind::Video) => "video",
            Self::Native(NativeKind::Map) => "map",
            Self::Native(NativeKind::TextInput) => "native_text_input",
        }
    }

    /// Read a kind, ignoring case and separators, so `text_field`,
    /// `TextField` and `textfield` are the same kind.
    pub fn parse(word: &str) -> Option<Self> {
        let normalized = normalize(word);
        Some(match normalized.as_str() {
            "text" => Self::Text,
            "icon" => Self::Icon,
            "image" => Self::Image,
            "container" => Self::Container,
            "spacer" => Self::Spacer,
            "divider" => Self::Divider,
            "button" => Self::Button,
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            "switch" => Self::Switch,
            "slider" => Self::Slider,
            "progress" => Self::Progress,
            "select" => Self::Select,
            "textfield" | "input" => Self::TextField,
            "stack" => Self::Stack,
            "grid" => Self::Grid,
            "list" => Self::List,
            "if" | "conditional" => Self::Conditional,
            "overlay" => Self::Overlay,
            "dialog" => Self::Dialog,
            "popup" => Self::Popup,
            "tabs" => Self::Tabs,
            "navigationbar" | "navbar" => Self::NavigationBar,
            "navigationrail" | "navrail" => Self::NavigationRail,
            "navigationdrawer" | "navdrawer" => Self::NavigationDrawer,
            "web" => Self::Native(NativeKind::Web),
            "video" => Self::Native(NativeKind::Video),
            "map" => Self::Native(NativeKind::Map),
            "nativetextinput" | "textinput" => Self::Native(NativeKind::TextInput),
            _ => return None,
        })
    }
}

impl EventName {
    /// The canonical spelling: the variant name in `snake_case`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Click => "click",
            Self::Change => "change",
            Self::Input => "input",
            Self::Submit => "submit",
            Self::Focus => "focus",
            Self::Blur => "blur",
            Self::Toggle => "toggle",
            Self::Scroll => "scroll",
            Self::Select => "select",
            Self::Dismiss => "dismiss",
            Self::Hover => "hover",
        }
    }

    pub fn all() -> &'static [EventName] {
        &[
            Self::Click,
            Self::Change,
            Self::Input,
            Self::Submit,
            Self::Focus,
            Self::Blur,
            Self::Toggle,
            Self::Scroll,
            Self::Select,
            Self::Dismiss,
            Self::Hover,
        ]
    }

    pub fn parse(word: &str) -> Option<Self> {
        Self::all().iter().copied().find(|e| e.name() == word)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips_through_its_spelling() {
        // The audit that keeps `parse` and `name` from drifting apart. A kind
        // added with only one half would be unreadable or unwritable.
        for kind in NodeKind::ALL {
            let name = kind.name();
            assert_eq!(
                NodeKind::parse(name),
                Some(*kind),
                "`{name}` did not parse back to {kind}"
            );
        }
    }

    #[test]
    fn spelling_is_forgiving_about_case_and_separators() {
        assert_eq!(NodeKind::parse("TextField"), Some(NodeKind::TextField));
        assert_eq!(NodeKind::parse("text-field"), Some(NodeKind::TextField));
        assert_eq!(NodeKind::parse("TEXT_FIELD"), Some(NodeKind::TextField));
        assert_eq!(NodeKind::parse("carousel"), None);
    }

    #[test]
    fn the_conditional_reads_as_if() {
        // The DSL spells it `if`, so `conditional` has to keep working too.
        assert_eq!(NodeKind::Conditional.name(), "if");
        assert_eq!(NodeKind::parse("if"), Some(NodeKind::Conditional));
        assert_eq!(NodeKind::parse("conditional"), Some(NodeKind::Conditional));
    }

    #[test]
    fn every_event_round_trips_through_its_spelling() {
        for event in EventName::all() {
            assert_eq!(EventName::parse(event.name()), Some(*event));
        }
        assert_eq!(EventName::parse("Nope"), None);
    }

    #[test]
    fn a_key_rejects_what_would_be_ambiguous() {
        assert!(NodeKey::is_valid("app.root"));
        assert!(NodeKey::is_valid("app.save-button"));
        assert!(!NodeKey::is_valid(""));
        assert!(!NodeKey::is_valid("app..root"));
        assert!(!NodeKey::is_valid("1st"));
        assert!(!NodeKey::is_valid("app.root!"));
    }
}
