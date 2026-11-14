// SPDX-License-Identifier: MIT
//! Document to a Rust builder program.
//!
//! Two things happen here that are worth stating plainly, because both are
//! losses and neither is silent.
//!
//! **Node keys are not materialized.** The generated program builds a static
//! tree; the builder keys its children by position. The document's `NodeKey`s
//! exist for the *reload* side of this phase — the differ in
//! `floom-hot-reload` matches two documents by key and a live tree by
//! `ElementId` — and a program compiled ahead of time has no reload cycle to
//! consume them. Emitting them anyway would mean inventing a string-to-`u64`
//! numbering that nothing else in the pipeline shares, and it would make every
//! otherwise identical subtree textually distinct, which is the opposite of
//! what the next paragraph is for.
//!
//! **Structural sharing is exact because keys are gone.** Two subtrees that
//! differ only by key emit the same text, so they can be emitted once. A
//! subtree occurring `repeat_threshold` times becomes one `fn`, and a *run* of
//! `repeat_threshold` identical siblings becomes a loop over that function
//! instead of that many call sites. The second mechanism is what makes the cost
//! independent of the repetition count rather than merely smaller: a document
//! with a hundred copies of a row costs almost what ten cost, while the same
//! document with a hundred *distinct* rows costs about ten times what ten cost.
//! Both are asserted, because the flat measurement alone would only prove that
//! the fixture never varied the thing being measured.
//!
//! What the program cannot express — a kind with no builder equivalent, a
//! property with no seat on its widget, a handler that is a document binding
//! rather than a function — is collected in [`Emitted::unmapped`] rather than
//! dropped quietly, and [`EmitOptions::strict`] turns any of it into an error
//! for callers who need fidelity rather than a compiling artifact.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use crate::ui_ir::{
    BindingKind, IrError, NativeKind, NodeKind, PropValue, Rgba, UiDocument, UiNode, ValueType,
};

/// Which widget set the emitted program calls into.
///
/// The emitter's job is to turn a design-system-neutral document into Rust
/// that builds it, and the *last* step of that needs concrete types. Which
/// types those are is a decision of the **application**, not of the DSL: an
/// app that depends only on `floom-widgets` has the primitives, and an app
/// that also depends on `floom-ui-composites` has the composite controls.
///
/// The emitter used to assume one widget set, which was true when every
/// control lived in `floom-widgets` and stopped being true when the
/// composites moved to the satellite workspace. Assuming still would have
/// produced a program that does not compile, so the assumption became a
/// parameter.
///
/// # The root workspace cannot see the satellite
///
/// `crates/ecosystem/floom-ui` is a separate workspace that the root one
/// excludes, so `floom-dsl` cannot name `floom_ui_composites` in its own
/// source. That is why this is a choice the *caller* makes: the caller's
/// crate is the one that has the dependency, and the emitted program is
/// compiled in the caller's crate.
///
/// # A kind the chosen set cannot express
///
/// Degrades to a placeholder and a **note**, never a silent substitution:
/// the same rule the renderers follow for a filter they cannot honour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WidgetSource {
    /// Only the primitives in `floom_widgets`.
    ///
    /// The composite controls — checkbox, radio, select, slider, text field —
    /// are not reachable from here, so a document that uses them gets
    /// placeholders and notes.
    #[default]
    Primitives,
    /// The primitives plus the composites in `floom_ui_composites`.
    ///
    /// The caller's crate must depend on `floom-ui-composites`; the emitted
    /// program imports it.
    WithComposites,
}

impl WidgetSource {
    /// Whether this source can express `kind`.
    ///
    /// The composite controls are the only kinds whose home moved.
    pub fn has(self, kind: NodeKind) -> bool {
        match kind {
            NodeKind::Checkbox
            | NodeKind::Radio
            | NodeKind::Select
            | NodeKind::Slider
            | NodeKind::TextField
            | NodeKind::Calendar
            | NodeKind::DatePicker
            | NodeKind::TimePicker
            | NodeKind::ColorPicker
            | NodeKind::Sparkline => matches!(self, Self::WithComposites),
            _ => true,
        }
    }

    /// The composite controls this source adds.
    fn composites(self) -> &'static [&'static str] {
        match self {
            Self::Primitives => &[],
            Self::WithComposites => &[
                "Checkbox",
                "Radio",
                "Select",
                "Slider",
                "TextField",
                "Calendar",
                "DatePicker",
                "TimePicker",
                "ColorPicker",
                "Sparkline",
            ],
        }
    }
}

/// How to emit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOptions {
    /// Refuse the document instead of substituting a placeholder for anything
    /// the widget set cannot express.
    pub strict: bool,
    /// How many times a subtree must occur before it is emitted once, and how
    /// long a run of identical siblings must be before it collapses to a loop.
    pub repeat_threshold: usize,
    /// Which widget set the emitted program calls into.
    pub widgets: WidgetSource,
    /// Emit signal-backed properties as reactive closures.
    ///
    /// When a property value is a `PropValue::Ref(name)`, the emitted program
    /// declares `name` as a `floom_signal::SyncBaseSignal` at the top of
    /// `build()` and reads it with `.get()` inside the per-child `push(move ||
    /// …)` closure — Floom re-runs that closure when the signal changes. This
    /// is the closure-capture seam: reactivity lives in the builder tree, not in
    /// a runtime host lookup.
    ///
    /// Off by default so a program with no signals keeps its static form and
    /// needs no `floom_signal` dependency.
    pub reactive: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        // Four is where an extracted body is comfortably cheaper than four
        // inline copies at the sizes this DSL describes.
        Self {
            strict: false,
            repeat_threshold: 4,
            widgets: WidgetSource::default(),
            reactive: false,
        }
    }
}

/// The imports the emitted program needs.
///
/// Built from the widget source rather than fixed, because a fixed list is
/// exactly what broke when the composite controls moved: the list named
/// types the program could no longer name, and the program did not compile.
///
/// The primitives are imported unconditionally and silenced — computing the
/// used subset would mean a second traversal whose only job is to make the
/// output prettier, and a stale `allow` is harmless where a missing import
/// is a compile error.
fn header(source: WidgetSource) -> String {
    let mut out = String::from(
        "// @generated by floom-dsl. Do not edit.\n\
         #![allow(unused_imports, clippy::all)]\n\
         \n\
         use floom_core::{Align, Axis, LayoutRequest, WidgetAny};\n\
         use floom_types::{\n\
         \x20   Color, InputMode, Insets, NativeCompositionMode, NativeViewKind,\n\
         };\n\
         use floom_widgets::{\n\
         \x20   Button, CircularProgress, Conditional, Container, Divider, Icon, Indeterminate,\n\
         \x20   LinearProgress, NativeSurface, Progress, Spacer, Stack, Switch, Text,\n\
         };\n",
    );
    let composites = source.composites();
    if !composites.is_empty() {
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!("use floom_ui_composites::{{{}}};\n", composites.join(", ")),
        );
    }
    out
}

/// A construct the generated program cannot express faithfully.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unmapped {
    /// The document node it belongs to.
    pub key: String,
    /// The kind of that node, spelled as in the IR.
    pub kind: String,
    /// What was approximated, and how.
    pub reason: String,
}

impl fmt::Display for Unmapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): {}", self.key, self.kind, self.reason)
    }
}

/// A generated program, and everything it could not express.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Emitted {
    pub code: String,
    /// Sorted by key, so a report reads like the document.
    pub unmapped: Vec<Unmapped>,
}

/// Why a document could not be emitted.
#[derive(Debug, Clone, PartialEq)]
pub enum EmitError {
    /// The document does not validate, so there is nothing well-defined to
    /// emit. The walk below assumes a tree — it recurses into children — and a
    /// document with a cycle would not terminate.
    Invalid(IrError),
    /// `strict` was set and the document needs a placeholder.
    Unmapped(Unmapped),
}

impl fmt::Display for EmitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(error) => write!(f, "cannot emit an invalid document: {error}"),
            Self::Unmapped(unmapped) => write!(f, "no faithful mapping for {unmapped}"),
        }
    }
}

impl std::error::Error for EmitError {}

/// What a kind builds into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Seat {
    /// No child slot in the builder API.
    Leaf,
    /// `Stack`, which appends children one at a time.
    Stack,
    /// `Conditional`, which holds exactly one child behind a flag.
    OneChild,
}

fn seat(kind: NodeKind, source: WidgetSource) -> Option<Seat> {
    // A kind the chosen widget set cannot name has no seat: it becomes a
    // placeholder with a note, the same way a kind with no builder
    // equivalent does. The check comes first because it is the one that
    // decides whether the type exists at all.
    if !source.has(kind) {
        return None;
    }
    Some(match kind {
        NodeKind::Text
        | NodeKind::Button
        | NodeKind::Checkbox
        | NodeKind::Radio
        | NodeKind::Switch
        | NodeKind::Slider
        | NodeKind::Progress
        | NodeKind::ProgressCircular
        | NodeKind::ProgressLinear
        | NodeKind::Select
        | NodeKind::TextField
        | NodeKind::Calendar
        | NodeKind::DatePicker
        | NodeKind::TimePicker
        | NodeKind::ColorPicker
        | NodeKind::Sparkline
        | NodeKind::Container
        | NodeKind::Spacer
        | NodeKind::Divider
        | NodeKind::Icon
        | NodeKind::Native(_) => Seat::Leaf,
        NodeKind::Stack => Seat::Stack,
        NodeKind::Conditional => Seat::OneChild,
        // `Grid`/`List` are generic over the item type and take closures that
        // produce items; `Overlay`/`Dialog`/`Popup`/`Tabs` take a `Child` built
        // around a key and a `LayoutRequest`; the navigation kinds are hosts
        // rather than boxes. None of these can be reconstructed from a static
        // document without inventing state, so they become placeholders that
        // keep their children.
        NodeKind::Grid
        | NodeKind::List
        | NodeKind::Overlay
        | NodeKind::Dialog
        | NodeKind::Popup
        | NodeKind::Tabs
        | NodeKind::NavigationBar
        | NodeKind::NavigationRail
        | NodeKind::NavigationDrawer
        | NodeKind::Image => return None,
    })
}

/// Emit a Rust builder program for `document`.
pub fn emit(document: &UiDocument, options: &EmitOptions) -> Result<Emitted, EmitError> {
    document.validate().map_err(EmitError::Invalid)?;

    let mut emitter = Emitter::new(
        document,
        options.repeat_threshold,
        options.widgets,
        options.reactive,
    );
    let code = emitter.run(document);
    let unmapped = emitter.unmapped;
    if options.strict
        && let Some(first) = unmapped.first()
    {
        return Err(EmitError::Unmapped(first.clone()));
    }
    Ok(Emitted { code, unmapped })
}

struct Emitter<'a> {
    nodes: BTreeMap<&'a str, &'a UiNode>,
    /// Document order, for reports.
    order: Vec<&'a UiNode>,
    /// Subtree fingerprint per node key.
    ///
    /// Owned keys rather than borrowed ones: the map is written while the
    /// document is being walked, and borrowing the keys would tie every helper
    /// to the document's lifetime for no gain.
    fingerprints: HashMap<String, String>,
    /// Fingerprint to generated function name, for subtrees that repeat.
    shared: BTreeMap<String, String>,
    /// `(node key, property name)` of every property the program carries.
    handled: HashSet<(String, String)>,
    /// `(node key, event name)` of every event the program wires.
    wired: HashSet<(String, String)>,
    unmapped: Vec<Unmapped>,
    threshold: usize,
    /// Which widget set the emitted program calls into.
    source: WidgetSource,
    /// Emit signal-backed properties as reactive closures.
    reactive: bool,
    /// Signal names (the value bindings), so each reactive `push` closure can own
    /// a clone. A signal is shared across closures via `Arc`, so a read (label)
    /// and a write (handler) can both capture it.
    signals: Vec<String>,
}

impl<'a> Emitter<'a> {
    fn new(
        document: &'a UiDocument,
        threshold: usize,
        source: WidgetSource,
        reactive: bool,
    ) -> Self {
        let nodes: BTreeMap<&str, &UiNode> = document
            .nodes
            .iter()
            .map(|node| (node.key.as_str(), node))
            .collect();
        let signals = if reactive {
            document
                .bindings
                .iter()
                .filter(|b| matches!(b.kind, BindingKind::Value(_)))
                .map(|b| b.name.clone())
                .collect()
        } else {
            Vec::new()
        };
        Self {
            nodes,
            order: document.nodes.iter().collect(),
            fingerprints: HashMap::new(),
            shared: BTreeMap::new(),
            handled: HashSet::new(),
            wired: HashSet::new(),
            unmapped: Vec::new(),
            threshold: threshold.max(2),
            source,
            reactive,
            signals,
        }
    }

    fn run(&mut self, document: &UiDocument) -> String {
        // Copied out so the loops below can call `&mut self` methods: `order`
        // holds `&UiNode`, which does not borrow from `self`.
        let order = self.order.clone();

        // Structural sharing extracts a repeated subtree into a top-level `fn`,
        // but a reactive subtree must read its signal from its closure's scope,
        // so a top-level `fn` cannot capture it. Sharing is therefore disabled
        // in reactive mode: every subtree stays inline in its `push` closure.
        let bodies: Vec<(String, String)> = if self.reactive {
            Vec::new()
        } else {
            // Fingerprint everything first: a node can only be shared once the
            // whole document is known.
            let mut counts: HashMap<String, usize> = HashMap::new();
            for node in &order {
                let fingerprint = self.fingerprint(node.key.as_str());
                *counts.entry(fingerprint).or_default() += 1;
            }
            let repeated: Vec<String> = {
                let mut repeated: Vec<String> = counts
                    .into_iter()
                    .filter(|(_, count)| *count >= self.threshold)
                    .map(|(fingerprint, _)| fingerprint)
                    .collect();
                // Sorted, so function numbering does not depend on hash iteration.
                repeated.sort();
                repeated
            };
            for (index, fingerprint) in repeated.into_iter().enumerate() {
                self.shared.insert(fingerprint, format!("shared_{index}"));
            }

            // Each function's body is the fingerprint's representative: the first
            // node in document order, so the output does not depend on which
            // occurrence a walk happened to reach first.
            let mut representatives: BTreeMap<String, String> = BTreeMap::new();
            for node in &order {
                let fingerprint = self.fingerprint(node.key.as_str());
                if self.shared.contains_key(&fingerprint) {
                    representatives
                        .entry(fingerprint)
                        .or_insert_with(|| node.key.to_string());
                }
            }
            // Copied out for the same reason as `order`, and sorted by name so the
            // emitted functions appear in call order.
            let mut definitions: Vec<(String, String)> = self
                .shared
                .iter()
                .map(|(fingerprint, name)| {
                    let key = representatives
                        .get(fingerprint)
                        .expect("a shared fingerprint has a representative node")
                        .clone();
                    (name.clone(), key)
                })
                .collect();
            definitions.sort();
            definitions
                .into_iter()
                .map(|(name, key)| {
                    let node = *self.nodes.get(key.as_str()).expect("indexed node");
                    // `build`, not `resolve`: the representative must not become a
                    // call to the function being defined.
                    let body = self.build(node);
                    (name, body)
                })
                .collect()
        };

        let mut out = header(self.source);
        if self.reactive {
            // Signals are read with `.get()` and written with `.set()` /
            // `.update()`; `SyncBaseSignal` is the `Send + Sync` signal a `push`
            // closure can capture. Each closure owns an `Arc` clone of the signal
            // (so a read and a write can both capture it), hence `Arc` and the
            // update trait in scope.
            out.push_str("use std::sync::Arc;\n");
            out.push_str(
                "use floom_signal::{Effect, SignalGet, SignalUpdate, SyncBaseSignal};\n\n",
            );
        }
        for (name, body) in bodies {
            let _ = std::fmt::Write::write_fmt(
                &mut out,
                format_args!("fn {name}() -> Box<dyn WidgetAny> {{\n    {body}\n}}\n\n"),
            );
        }

        let root_expr = match self.nodes.get(document.root.as_str()) {
            Some(node) => self.resolve(node),
            // Validation guarantees the root exists; this arm keeps the
            // function total rather than because it is reachable.
            None => String::from("Box::new(Container::new())"),
        };
        let signal_decls = self.signal_decls(document);
        let _ = std::fmt::Write::write_fmt(
            &mut out,
            format_args!(
                "pub fn build() -> Box<dyn WidgetAny> {{\n    {signal_decls}{root_expr}\n}}\n"
            ),
        );

        self.report_leftovers();
        out
    }

    // --- sharing -----------------------------------------------------------

    /// A canonical, key-free description of a node's whole subtree.
    ///
    /// Key-free is what makes sharing possible at all (see the module docs),
    /// and canonical — properties sorted, events sorted — is what makes two
    /// structurally equal subtrees produce one string rather than two that
    /// happen to differ by authoring order.
    ///
    /// A node's fingerprint *contains* its children's, so a proper descendant
    /// can never share its ancestor's fingerprint (the string would have to
    /// contain itself) and two fingerprints can never contain each other. The
    /// containment is therefore a strict order, which is why the generated
    /// functions cannot be mutually recursive — a property of the
    /// representation rather than a check that has to be maintained.
    fn fingerprint(&mut self, key: &str) -> String {
        if let Some(fingerprint) = self.fingerprints.get(key) {
            return fingerprint.clone();
        }
        let Some(node) = self.nodes.get(key).copied() else {
            return String::from("?");
        };
        let mut fingerprint = String::new();
        let _ =
            std::fmt::Write::write_fmt(&mut fingerprint, format_args!("{}{{", node.kind.name()));
        let mut props: Vec<(&str, String)> = node
            .props
            .iter()
            .map(|prop| (prop.name.as_str(), render_value(&prop.value)))
            .collect();
        props.sort();
        for (name, value) in &props {
            let _ = std::fmt::Write::write_fmt(&mut fingerprint, format_args!("{name}={value},"));
        }
        fingerprint.push('|');
        let mut events: Vec<(String, String)> = node
            .events
            .iter()
            .map(|event| {
                (
                    event.event.name().to_string(),
                    event.body.clone().unwrap_or_else(|| event.handler.clone()),
                )
            })
            .collect();
        events.sort();
        for (event, handler) in &events {
            let _ =
                std::fmt::Write::write_fmt(&mut fingerprint, format_args!("{event}:{handler},"));
        }
        fingerprint.push('|');
        for child in node.children() {
            let child_fingerprint = self.fingerprint(child.as_str());
            let _ =
                std::fmt::Write::write_fmt(&mut fingerprint, format_args!("{child_fingerprint};"));
        }
        fingerprint.push(')');
        self.fingerprints
            .insert(key.to_string(), fingerprint.clone());
        fingerprint
    }

    /// The node's subtree: a call when it is shared, inline otherwise.
    fn resolve(&mut self, node: &UiNode) -> String {
        let fingerprint = self.fingerprint(node.key.as_str());
        if let Some(name) = self.shared.get(&fingerprint) {
            return format!("{name}()");
        }
        self.build(node)
    }

    // --- building ----------------------------------------------------------

    fn build(&mut self, node: &UiNode) -> String {
        let children: Vec<&'a UiNode> = node
            .children()
            .filter_map(|key| self.nodes.get(key.as_str()).copied())
            .collect();

        match seat(node.kind, self.source) {
            Some(Seat::Stack) => {
                let base = self.base_expr(node);
                if children.is_empty() {
                    format!("Box::new({base})")
                } else {
                    self.container(node, base, &children, Seat::Stack)
                }
            }
            Some(Seat::OneChild) => self.build_one_child(node, &children),
            Some(Seat::Leaf) => {
                let base = self.base_expr(node);
                if children.is_empty() {
                    format!("Box::new({base})")
                } else {
                    // The IR allows children on any node; the widget behind a
                    // leaf has no child slot. Wrapping keeps them.
                    self.note(
                        node,
                        format!(
                            "{} has no child slot; its children are wrapped in a Stack",
                            node.kind
                        ),
                    );
                    self.container(node, base, &children, Seat::Leaf)
                }
            }
            None => {
                let base = String::from("Stack::new(Axis::Vertical)");
                if children.is_empty() {
                    self.note(
                        node,
                        format!(
                            "{} has no builder equivalent; substituted a Container",
                            node.kind
                        ),
                    );
                    String::from("Box::new(Container::new())")
                } else {
                    self.note(
                        node,
                        format!(
                            "{} has no builder equivalent; substituted a Stack that keeps its children",
                            node.kind
                        ),
                    );
                    self.container(node, base, &children, Seat::Stack)
                }
            }
        }
    }

    /// A container built from a base expression and a child list.
    fn container(
        &mut self,
        node: &UiNode,
        base: String,
        children: &[&'a UiNode],
        _seat: Seat,
    ) -> String {
        let mut out = String::from("{\n        let mut v = ");
        out.push_str(&base);
        out.push_str(";\n");
        for (run, child) in self.runs(children) {
            match run {
                Some((count, fingerprint)) => {
                    let name = self.shared.get(&fingerprint).cloned().expect("shared");
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!("        for _ in 0..{count} {{ v = v.push({name}); }}\n"),
                    );
                }
                None => {
                    let layout = self.layout_request(child);
                    let child_expr = self.resolve(child);
                    let build = self.closure_block(&child_expr);
                    match layout {
                        Some(layout) => {
                            let _ = std::fmt::Write::write_fmt(
                                &mut out,
                                format_args!("        v = v.push_layout({layout}, {build});\n"),
                            );
                        }
                        None => {
                            let _ = std::fmt::Write::write_fmt(
                                &mut out,
                                format_args!("        v = v.push({build});\n"),
                            );
                        }
                    }
                }
            }
        }
        let _ = node;
        out.push_str("        Box::new(v)\n    }");
        out
    }

    /// The children of a container, grouped: `Some((count, fingerprint))` for a
    /// run that collapses to a loop, `None` for a child emitted on its own.
    ///
    /// A run only collapses when its element is already shared, because the
    /// loop body is a call to a function and there would be nothing to call.
    fn runs(&mut self, children: &[&'a UiNode]) -> Vec<(Option<(usize, String)>, &'a UiNode)> {
        let mut out = Vec::new();
        let mut index = 0;
        while index < children.len() {
            let fingerprint = self.fingerprint(children[index].key.as_str());
            let shareable = self.shared.contains_key(&fingerprint);
            let mut end = index + 1;
            if shareable {
                while end < children.len()
                    && self.fingerprint(children[end].key.as_str()) == fingerprint
                {
                    end += 1;
                }
            }
            let count = end - index;
            if shareable && count >= self.threshold {
                out.push((Some((count, fingerprint)), children[index]));
            } else {
                for child in &children[index..end] {
                    out.push((None, child));
                }
            }
            index = end;
        }
        out
    }

    fn build_one_child(&mut self, node: &UiNode, children: &[&'a UiNode]) -> String {
        let mut base = self.base_expr(node);
        if let Some(child) = children.first() {
            let layout = self
                .layout_request(child)
                .unwrap_or_else(|| String::from("LayoutRequest::new()"));
            let child_expr = self.resolve(child);
            let build = self.closure_block(&child_expr);
            let _ = std::fmt::Write::write_fmt(
                &mut base,
                format_args!(".show_with(0usize, {layout}, {build})"),
            );
        }
        if children.len() > 1 {
            self.note(
                node,
                format!(
                    "{} shows one child; {} further child(ren) were dropped",
                    node.kind,
                    children.len() - 1
                ),
            );
        }
        format!("Box::new({base})")
    }

    /// The expression for a node without its children.
    ///
    /// Every property it reads is marked handled, so [`Self::report_leftovers`]
    /// can name exactly what the program does not carry.
    fn base_expr(&mut self, node: &UiNode) -> String {
        let mut out = match node.kind {
            NodeKind::Text => {
                let label = self.label_expr(node);
                let mut out = format!("Text::new({label})");
                // `size` is Text's own font size; `font_size` is the shared
                // style property. Either satisfies it, and the specific one
                // wins — so read `font_size` only when `size` is absent, or the
                // presence of `size` would silently report `font_size` as
                // carried while ignoring it.
                let size = match self.num(node, "size") {
                    Some(size) => Some(size),
                    None => self.num(node, "font_size"),
                };
                if let Some(size) = size {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".size({})", num_lit(size)),
                    );
                }
                if let Some(color) = self.color(node, "foreground") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".color({})", color_lit(color)),
                    );
                }
                out
            }
            NodeKind::Icon => {
                let glyph = self.text(node, "glyph");
                match glyph.as_deref().and_then(single_char) {
                    Some(glyph) => {
                        let mut out = format!("Icon::new({})", char_lit(glyph));
                        if let Some(color) = self.color(node, "foreground") {
                            let _ = std::fmt::Write::write_fmt(
                                &mut out,
                                format_args!(".color({})", color_lit(color)),
                            );
                        }
                        if let Some(size) = self.num(node, "size") {
                            let _ = std::fmt::Write::write_fmt(
                                &mut out,
                                format_args!(".size({})", num_lit(size)),
                            );
                        }
                        out
                    }
                    None => {
                        if let Some(glyph) = glyph {
                            self.note(
                                node,
                                format!("Icon::new takes one character; `{glyph}` is not one"),
                            );
                        }
                        String::from("Container::new()")
                    }
                }
            }
            NodeKind::Button => {
                let label = self.label_expr(node);
                let mut out = format!("Button::new({label})");
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                if let Some(color) = self.color(node, "foreground") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".color({})", color_lit(color)),
                    );
                }
                if let Some(handler) = self.event(node, "click") {
                    let closure = self.handler_closure("", &handler);
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".on_click({closure})"));
                }
                out
            }
            NodeKind::Checkbox => {
                let label = self.text(node, "label").unwrap_or_default();
                let mut out = format!("Checkbox::new({})", lit(&label));
                let checked = if self.reactive {
                    // A signal-backed `checked` reads it through `.get()`; fall
                    // back to a literal for the static case.
                    self.ref_prop(node, "checked")
                        .or_else(|| self.boolean(node, "checked").map(|on| on.to_string()))
                } else {
                    self.boolean(node, "checked").map(|on| on.to_string())
                };
                if let Some(checked) = checked {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".checked({checked})"));
                }
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                if let Some(handler) = self.event(node, "toggle") {
                    let closure = self.handler_closure("_v", &handler);
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".on_toggle({closure})"));
                }
                out
            }
            NodeKind::Radio => {
                let label = self.text(node, "label").unwrap_or_default();
                let mut out = format!("Radio::new({})", lit(&label));
                let selected = if self.reactive {
                    self.ref_prop(node, "checked")
                        .or_else(|| self.boolean(node, "checked").map(|on| on.to_string()))
                } else {
                    self.boolean(node, "checked").map(|on| on.to_string())
                };
                if let Some(selected) = selected {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".selected({selected})"));
                }
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                if let Some(handler) = self.event(node, "toggle") {
                    let closure = self.handler_closure("", &handler);
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".on_select({closure})"));
                }
                out
            }
            NodeKind::Switch => {
                // `label_value` already returns a Rust-ready label: a quoted
                // literal for static text, or an inlined read/computed expression
                // when reactive — so it is dropped in verbatim, never re-quoted.
                let label = self.label_value(node);
                let mut out = format!("Switch::new({label})");
                let checked = if self.reactive {
                    if let Some(read) = self.ref_prop(node, "checked") {
                        Some(read)
                    } else {
                        self.boolean(node, "checked").map(|on| on.to_string())
                    }
                } else {
                    self.boolean(node, "checked").map(|on| on.to_string())
                };
                if let Some(checked) = checked {
                    let _ = std::fmt::Write::write_fmt(&mut out, format_args!(".on({checked})"));
                }
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                if let Some(handler) = self.event(node, "toggle") {
                    let closure = self.handler_closure("_v", &handler);
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".on_toggle({closure})"));
                }
                out
            }
            NodeKind::Slider => {
                let value = if self.reactive {
                    if let Some(read) = self.ref_num(node, "value") {
                        // `Slider::new` takes `f64`; a signal declared as
                        // `Int`/`Number`/`f32` all reach it through `as f64`.
                        format!("({read}) as f64")
                    } else {
                        match self.num(node, "value") {
                            Some(value) => num_lit(value),
                            None => "0.0".to_string(),
                        }
                    }
                } else {
                    match self.num(node, "value") {
                        Some(value) => num_lit(value),
                        None => "0.0".to_string(),
                    }
                };
                let min = match self.num(node, "min") {
                    Some(value) => num_lit(value),
                    None => "0.0".to_string(),
                };
                let max = match self.num(node, "max") {
                    Some(value) => num_lit(value),
                    None => "1.0".to_string(),
                };
                let mut out = format!("Slider::new({}, {}, {})", value, min, max);
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                if let Some(handler) = self.event(node, "change") {
                    let closure = self.handler_closure("_v", &handler);
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".on_change({closure})"));
                }
                out
            }
            NodeKind::Progress => {
                let value = self.progress_fraction(node);
                let mut out = format!("Progress::determinate({value})");
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                out
            }
            NodeKind::ProgressCircular => {
                // The ring takes a fraction in `0..=1`, the same domain the
                // generic bar is normalised into.
                let mut out = if self.boolean(node, "indeterminate") == Some(true) {
                    String::from("CircularProgress::indeterminate()")
                } else {
                    let value = self.progress_fraction(node);
                    format!("CircularProgress::determinate({value})")
                };
                if let Some(size) = self.num(node, "size") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".size({})", num_lit(size)),
                    );
                }
                out
            }
            NodeKind::ProgressLinear => {
                // The bar takes a percentage in `0..=100`, so the normalised
                // fraction is scaled on the way in.
                let value = self.progress_fraction(node);
                let mut out = format!("LinearProgress::new(({value} * 100.0) as f32)");
                if self.boolean(node, "indeterminate") == Some(true) {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".indeterminate_mode(Indeterminate::SlidingSegment)"),
                    );
                }
                out
            }
            NodeKind::Calendar => String::from("Calendar::new()"),
            NodeKind::DatePicker => String::from("DatePicker::new()"),
            NodeKind::TimePicker => String::from("TimePicker::new()"),
            NodeKind::ColorPicker => String::from("ColorPicker::new()"),
            NodeKind::Sparkline => String::from("Sparkline::new()"),
            NodeKind::TextField => {
                let mut out = String::from("TextField::new()");
                if let Some(value) = self.text(node, "value") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".value({})", lit(&value)),
                    );
                }
                if let Some(placeholder) = self.text(node, "placeholder") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".placeholder({})", lit(&placeholder)),
                    );
                }
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                for (event, setter) in [("input", "on_change"), ("submit", "on_submit")] {
                    if let Some(handler) = self.event(node, event) {
                        let closure = self.handler_closure("_v", &handler);
                        let _ = std::fmt::Write::write_fmt(
                            &mut out,
                            format_args!(".{setter}({closure})"),
                        );
                    }
                }
                out
            }
            NodeKind::Select => {
                // The IR carries the chosen value, not a choice list; a
                // one-option Select is the closest the document can justify.
                let options = match self.text(node, "value") {
                    Some(value) => format!("[{}]", lit(&value)),
                    None => String::from("Vec::<String>::new()"),
                };
                let mut out = format!("Select::new({options})");
                if let Some(disabled) = self.boolean(node, "disabled") {
                    let _ =
                        std::fmt::Write::write_fmt(&mut out, format_args!(".disabled({disabled})"));
                }
                if let Some(handler) = self.event(node, "change") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".on_change(|_v| {{ {handler} }})"),
                    );
                }
                if self.text(node, "placeholder").is_some() {
                    // Read and discarded on purpose: `Select` has no
                    // placeholder seat, so recording it keeps the report
                    // accurate rather than silently swallowing it.
                    self.note(node, "`placeholder` has no seat on Select".into());
                }
                out
            }
            NodeKind::Container => {
                let mut out = String::from("Container::new()");
                if let Some(color) = self.color(node, "background") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".background({})", color_lit(color)),
                    );
                }
                if let Some(radius) = self.num(node, "radius") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".radius({})", num_lit_f32(radius)),
                    );
                }
                if let Some(padding) = self.num(node, "padding") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".padding(Insets::uniform({}))", num_lit(padding)),
                    );
                }
                out
            }
            NodeKind::Spacer => {
                // The IR has no extent property, so `flexible` is the only
                // construction the document can justify.
                String::from("Spacer::flexible()")
            }
            NodeKind::Divider => {
                let mut out = String::from("Divider::new()");
                if let Some(color) = self.color(node, "foreground") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".color({})", color_lit(color)),
                    );
                }
                out
            }
            NodeKind::Stack => {
                let axis = self
                    .ident(node, "axis")
                    .unwrap_or_else(|| "vertical".into());
                let mut out = format!("Stack::new({})", axis_variant(&axis));
                if let Some(gap) = self.num(node, "gap") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".gap({})", num_lit_f32(gap)),
                    );
                }
                // The builder sets both axes in one call, so a document that
                // names only one of them is reported rather than completed with
                // an assumption about the other axis' default.
                let main = node.prop("main_align").is_some();
                let cross = node.prop("cross_align").is_some();
                if main && cross {
                    let main = self.ident(node, "main_align").expect("present");
                    let cross = self.ident(node, "cross_align").expect("present");
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(
                            ".align({}, {})",
                            align_variant(&main),
                            align_variant(&cross)
                        ),
                    );
                }
                out
            }
            NodeKind::Conditional => {
                let when = self.boolean(node, "when").unwrap_or(false);
                format!("Conditional::new({when})")
            }
            NodeKind::Native(kind) => {
                let mut out = format!(
                    "NativeSurface::new(NativeViewKind::{})",
                    native_variant(kind)
                );
                if let Some(mode) = self.ident(node, "input_mode") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(".input(InputMode::{})", input_variant(&mode)),
                    );
                }
                if let Some(composition) = self.ident(node, "composition") {
                    let _ = std::fmt::Write::write_fmt(
                        &mut out,
                        format_args!(
                            ".modes(&[NativeCompositionMode::{}])",
                            composition_variant(&composition)
                        ),
                    );
                }
                out
            }
            // `seat` returns `None` for the remaining kinds, so `build` never
            // reaches `base_expr` for them; the arm keeps the match total.
            _ => String::from("Container::new()"),
        };

        // `radius` is shared style vocabulary, but only the kinds that can
        // paint a corner read it above.
        if !matches!(node.kind, NodeKind::Container) && node.prop("radius").is_some() {
            self.note(node, "`radius` has no seat on this widget".into());
        }

        let _ = &mut out;
        out
    }

    /// Signal declarations prepended to `build()` in reactive mode.
    ///
    /// One `SyncBaseSignal` per value binding, seeded with a type default — the
    /// IR carries a binding's type but not its TSX initializer, so a signal
    /// starts at `0` / `""` / `false` rather than the value in `createSignal(0)`.
    /// The trailing newline+indent is included so this splices directly before
    /// the root expression.
    ///
    /// Each signal also gets a rebuild effect: it reads the signal (registering
    /// the dependency) and, whenever the read reports a change, asks the host to
    /// re-roll the root composite's children — the closure-capture seam.
    fn signal_decls(&self, document: &UiDocument) -> String {
        if !self.reactive {
            return String::new();
        }
        let mut out = String::new();
        for binding in &document.bindings {
            if let BindingKind::Value(ty) = binding.kind {
                // Seed the signal with the author's literal when the source
                // carried one (`createSignal("hi")`); fall back to the type's
                // default for DSL bindings that only declare a type. A label
                // bound to an unseeded signal paints empty until first write,
                // which reads as "the binding does not work".
                let init = match &binding.init {
                    Some(value) => seed_lit(value, ty),
                    None => default_lit(ty),
                };
                let _ = std::fmt::Write::write_fmt(
                    &mut out,
                    format_args!(
                        "    let {} = Arc::new(SyncBaseSignal::new({}));\n    ",
                        binding.name, init
                    ),
                );
                let _ = std::fmt::Write::write_fmt(
                    &mut out,
                    format_args!(
                        "    Effect::new({{\n        let {name} = {name}.clone();\n        move |prev: Option<{ty}>| -> {ty} {{\n            if prev.is_some() {{\n                floom_core::request_rebuild_root();\n            }}\n            {name}.get()\n        }}\n    }});\n\n",
                        ty = rust_type(ty),
                        name = binding.name,
                    ),
                );
            }
        }
        out
    }

    /// The Rust expression that reads a property's value, if it is reactive:
    /// a `Ref` becomes `name.get()`, a computed expression is inlined verbatim
    /// (it already references its signals via `.get()`). Returns `None` for a
    /// literal, where the caller falls back to its typed reader.
    fn ref_prop(&mut self, node: &UiNode, name: &str) -> Option<String> {
        match self.prop(node, name) {
            Some(PropValue::Ref(reference)) => Some(format!("{reference}.get()")),
            Some(PropValue::Computed(expr)) => Some(expr.clone()),
            _ => None,
        }
    }

    /// The Rust expression that reads a numeric property's value, if it is
    /// reactive. See [`Emitter::ref_prop`].
    fn ref_num(&mut self, node: &UiNode, name: &str) -> Option<String> {
        match self.prop(node, name) {
            Some(PropValue::Ref(reference)) => Some(format!("{reference}.get()")),
            Some(PropValue::Computed(expr)) => Some(expr.clone()),
            _ => None,
        }
    }

    /// The label argument for a `Text`/`Button`/`Switch`: a string literal, or a
    /// reactive read/`Computed` expression when reactive.
    fn label_value(&mut self, node: &UiNode) -> String {
        if self.reactive {
            match self.prop(node, "label") {
                Some(PropValue::Ref(name)) => return format!("format!(\"{{}}\", {name}.get())"),
                Some(PropValue::Computed(expr)) => return expr.clone(),
                _ => {}
            }
        }
        lit(&self.text(node, "label").unwrap_or_default())
    }

    /// The `Text`/`Button` label argument: a string literal, or a signal read
    /// when reactive.
    fn label_expr(&mut self, node: &UiNode) -> String {
        self.label_value(node)
    }

    /// The `push` build argument: a closure that builds the child. In reactive
    /// mode it is a `move` closure wrapped in a block that first clones the
    /// signals the body reads, so the closure owns an `Arc` handle rather than
    /// moving the caller's binding out from under the closures around it.
    ///
    /// A signal that is both read (a label) and written (an inline handler) is
    /// used by two closures; each must hold its own handle, so each captures an
    /// `Arc` clone rather than the single binding.
    fn closure_block(&self, child_expr: &str) -> String {
        if !self.reactive {
            return format!("|| {{ {child_expr} }}");
        }
        let mut clones = String::new();
        for name in &self.signals {
            // Only clone signals this body actually names, so unused ones are not
            // moved into the closure.
            if mentions_ident(child_expr, name.as_str()) {
                let _ = std::fmt::Write::write_fmt(
                    &mut clones,
                    format_args!("let {name} = {name}.clone(); "),
                );
            }
        }
        format!("{{ {clones}move || {{ {child_expr} }} }}")
    }

    /// The argument to an event setter (`.on_click(…)`, `.on_toggle(…)`), which
    /// is a `'static` closure and so must own the signals its body touches. In
    /// reactive mode it captures an `Arc` clone of each signal it names, rather
    /// than borrowing the enclosing closure's binding.
    fn handler_closure(&self, param: &str, body: &str) -> String {
        if !self.reactive {
            return format!("|{param}| {{ {body} }}");
        }
        let mut clones = String::new();
        for name in &self.signals {
            if mentions_ident(body, name.as_str()) {
                let _ = std::fmt::Write::write_fmt(
                    &mut clones,
                    format_args!("let {name} = {name}.clone(); "),
                );
            }
        }
        format!("{{ {clones}move |{param}| {{ {body} }} }}")
    }

    // --- property reads ----------------------------------------------------
    //
    // Each marks the property as carried, so a read that happens and a report
    // of "no seat" can never both be true for the same property.

    fn text(&mut self, node: &UiNode, name: &str) -> Option<String> {
        match self.prop(node, name) {
            Some(PropValue::Text(text)) => Some(text.clone()),
            _ => None,
        }
    }

    fn num(&mut self, node: &UiNode, name: &str) -> Option<f64> {
        match self.prop(node, name) {
            Some(PropValue::Number(number)) => Some(*number),
            Some(PropValue::Int(int)) => Some(*int as f64),
            _ => None,
        }
    }

    fn boolean(&mut self, node: &UiNode, name: &str) -> Option<bool> {
        match self.prop(node, name) {
            Some(PropValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    /// A progress value as a fraction in `0..=1`, as a Rust expression.
    ///
    /// `Progress::determinate` takes a fraction, while the IR carries a value
    /// in `min..=max`. Reading `value` alone made a `value=64 min=0 max=100`
    /// bar paint full — the domain was silently thrown away. Normalise here,
    /// at compile time for a literal and at run time for a signal.
    fn progress_fraction(&mut self, node: &UiNode) -> String {
        let min = self.num(node, "min").unwrap_or(0.0);
        let max = self.num(node, "max").unwrap_or(100.0);
        let span = if max > min { max - min } else { 1.0 };
        if self.reactive {
            if let Some(read) = self.ref_num(node, "value") {
                return format!("({read} as f64 - {}) / {}", num_lit(min), num_lit(span));
            }
        }
        match self.num(node, "value") {
            Some(value) => num_lit(((value - min) / span).clamp(0.0, 1.0)),
            None => "0.0".to_string(),
        }
    }

    fn color(&mut self, node: &UiNode, name: &str) -> Option<Rgba> {
        match self.prop(node, name) {
            Some(PropValue::Color(color)) => Some(*color),
            _ => None,
        }
    }

    fn ident(&mut self, node: &UiNode, name: &str) -> Option<String> {
        match self.prop(node, name) {
            Some(PropValue::Ident(word)) => Some(word.clone()),
            _ => None,
        }
    }

    fn prop<'n>(&mut self, node: &'n UiNode, name: &str) -> Option<&'n PropValue> {
        let found = node.prop(name);
        if found.is_some() {
            self.handled
                .insert((node.key.to_string(), name.to_string()));
        }
        found
    }

    /// Look up an event handler and mark the event as wired, returning the Rust
    /// code for the closure body.
    ///
    /// An *inline* handler (the TSX compiler lowered `onClick={() => …}` to Rust)
    /// already carries its body, so it is returned verbatim. A *reference*
    /// handler is a name in the document's binding table for which no function
    /// exists in a generated program, so an empty closure is wired and the
    /// substitution is reported as a note.
    fn event(&mut self, node: &UiNode, name: &str) -> Option<String> {
        let binding = node
            .events
            .iter()
            .find(|binding| binding.event.name() == name)?;
        self.wired.insert((node.key.to_string(), name.to_string()));
        if let Some(body) = &binding.body {
            return Some(body.clone());
        }
        let handler = binding.handler.clone();
        self.note(
            node,
            format!(
                "the `{name}` handler `{handler}` is a document binding; the program wires an empty closure"
            ),
        );
        Some(String::new())
    }

    /// A `LayoutRequest` for the child, if it asks for one.
    ///
    /// The child carries its layout because that is where the builder API puts
    /// it (`push_layout`), and `width`/`height`/`grow` are the three the
    /// document can state. `position` and the insets are absolute placement,
    /// which needs a containing box the IR does not describe.
    fn layout_request(&mut self, node: &UiNode) -> Option<String> {
        let mut request = String::from("LayoutRequest::new()");
        let mut any = false;
        for (prop, setter) in [("width", "width"), ("height", "height"), ("grow", "grow")] {
            if let Some(value) = self.num(node, prop) {
                let _ = std::fmt::Write::write_fmt(
                    &mut request,
                    format_args!(".{setter}({})", num_lit_f32(value)),
                );
                any = true;
            }
        }
        any.then_some(request)
    }

    fn note(&mut self, node: &UiNode, reason: String) {
        let unmapped = Unmapped {
            key: node.key.to_string(),
            kind: node.kind.name().to_string(),
            reason,
        };
        if !self.unmapped.contains(&unmapped) {
            self.unmapped.push(unmapped);
        }
    }

    /// Every property and event the program did not carry, in document order.
    fn report_leftovers(&mut self) {
        let mut leftovers: Vec<Unmapped> = Vec::new();
        for node in &self.order {
            for prop in &node.props {
                if self
                    .handled
                    .contains(&(node.key.to_string(), prop.name.clone()))
                {
                    continue;
                }
                leftovers.push(Unmapped {
                    key: node.key.to_string(),
                    kind: node.kind.name().to_string(),
                    reason: format!("`{}` has no seat in the generated program", prop.name),
                });
            }
            for binding in &node.events {
                if self
                    .wired
                    .contains(&(node.key.to_string(), binding.event.name().to_string()))
                {
                    continue;
                }
                leftovers.push(Unmapped {
                    key: node.key.to_string(),
                    kind: node.kind.name().to_string(),
                    reason: format!("`{}` has no seat on this widget", binding.event.name()),
                });
            }
        }
        for leftover in leftovers {
            if !self.unmapped.contains(&leftover) {
                self.unmapped.push(leftover);
            }
        }
        self.unmapped.sort_by(|a, b| {
            (a.key.as_str(), a.reason.as_str()).cmp(&(b.key.as_str(), b.reason.as_str()))
        });
        self.unmapped.dedup();
    }
}

// --- small spellings -------------------------------------------------------

fn single_char(text: &str) -> Option<char> {
    let mut chars = text.chars();
    let first = chars.next()?;
    chars.next().is_none().then_some(first)
}

fn render_value(value: &PropValue) -> String {
    match value {
        PropValue::Bool(b) => format!("bool:{b}"),
        PropValue::Int(i) => format!("int:{i}"),
        PropValue::Number(n) => format!("num:{n}"),
        PropValue::Text(t) => format!("text:{}", lit(t)),
        PropValue::Color(c) => format!("color:{}", c.to_hex()),
        PropValue::Ident(i) => format!("ident:{i}"),
        PropValue::List(items) => {
            let inner: Vec<String> = items.iter().map(render_value).collect();
            format!("list:[{}]", inner.join(","))
        }
        PropValue::Ref(name) => format!("ref:{name}"),
        PropValue::Computed(expr) => format!("computed:{expr}"),
    }
}

/// A type-appropriate default literal for a signal's initial value.
///
/// The IR records a binding's type but not its TSX initializer, so a signal is
/// seeded from its type's zero value rather than `createSignal(..)`'s argument.
fn default_lit(ty: ValueType) -> String {
    match ty {
        ValueType::Bool => "false".to_string(),
        ValueType::Int => "0i64".to_string(),
        ValueType::Number => "0.0f64".to_string(),
        ValueType::Text | ValueType::Ident => "String::new()".to_string(),
        ValueType::Color => "floom_types::Color::BLACK".to_string(),
        ValueType::List => "Vec::new()".to_string(),
    }
}

/// The Rust type a signal of `ty` holds.
fn rust_type(ty: ValueType) -> &'static str {
    match ty {
        ValueType::Bool => "bool",
        ValueType::Int => "i64",
        ValueType::Number => "f64",
        ValueType::Text | ValueType::Ident => "String",
        ValueType::Color => "floom_types::Color",
        ValueType::List => "Vec<floom_types::Color>",
    }
}

/// The Rust literal a signal is seeded with, given its declared type.
///
/// The payload is coerced to the declared type rather than trusted: a TSX
/// `createSignal(0)` infers `Int`, but a signal used by a `Number` prop must
/// still start at `0.0f64` or the generated `SyncBaseSignal<i64>` will not
/// match the `.get()` the prop expects.
fn seed_lit(value: &PropValue, ty: ValueType) -> String {
    match ty {
        ValueType::Bool => match value {
            PropValue::Bool(b) => b.to_string(),
            _ => default_lit(ty),
        },
        ValueType::Int => match value {
            PropValue::Int(i) => format!("{i}i64"),
            PropValue::Number(n) => format!("{}i64", *n as i64),
            _ => default_lit(ty),
        },
        ValueType::Number => match value {
            PropValue::Number(n) => num_lit(*n),
            PropValue::Int(i) => num_lit(*i as f64),
            _ => default_lit(ty),
        },
        ValueType::Text | ValueType::Ident => match value {
            PropValue::Text(text) => format!("{}.to_string()", lit(text)),
            PropValue::Ident(text) => format!("{}.to_string()", lit(text)),
            _ => default_lit(ty),
        },
        ValueType::Color => match value {
            PropValue::Color(rgba) => color_lit(*rgba),
            _ => default_lit(ty),
        },
        ValueType::List => default_lit(ty),
    }
}

/// A Rust string literal.
fn lit(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

/// True when `haystack` names `needle` as a whole Rust identifier, outside a
/// string or char literal.
///
/// A plain `contains` would treat the word "progress" inside the label
/// `"drag the slider to move the progress bar"` as a use of the `progress`
/// signal, cloning it into a closure that never reads it. Cloning an `Arc` is
/// cheap but the unused binding is a warning in every generated program. Skipping
/// string/char literals is what makes the word a *label* rather than a read.
fn mentions_ident(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    let chars: Vec<char> = haystack.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Skip a string literal, including its quotes and escapes.
        if c == '"' {
            i += 1;
            while i < chars.len() {
                match chars[i] {
                    '\\' => i += 2,
                    '"' => {
                        i += 1;
                        break;
                    }
                    _ => i += 1,
                }
            }
            continue;
        }
        // Skip a char literal; `'a'`, `'\n'`. A lifetime is not a literal, so
        // only skip when a closing quote is within reach.
        if c == '\''
            && let Some(close) = chars[i + 1..].iter().position(|&ch| ch == '\'')
        {
            i += close + 2;
            continue;
        }
        if c == '_' || c.is_alphabetic() {
            let start = i;
            while i < chars.len() && ident(chars[i]) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            if word == needle {
                return true;
            }
            continue;
        }
        i += 1;
    }
    false
}

fn char_lit(c: char) -> String {
    match c {
        '\'' => String::from("'\\''"),
        '\\' => String::from("'\\\\'"),
        other => format!("'{other}'"),
    }
}

/// A numeric literal for an `f64` seat.
fn num_lit(value: f64) -> String {
    if value.is_finite() && value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        format!("{value}")
    }
}

/// A numeric literal for an `f32` seat.
///
/// `8.0` alone would infer, but `8.0` in a generic position that also accepts
/// `f64` would not; the suffix pins it.
fn num_lit_f32(value: f64) -> String {
    format!("{}f32", num_lit(value))
}

fn color_lit(color: Rgba) -> String {
    format!(
        "Color::rgba({}, {}, {}, {})",
        color.r, color.g, color.b, color.a
    )
}

fn axis_variant(axis: &str) -> &'static str {
    match axis {
        "horizontal" => "Axis::Horizontal",
        _ => "Axis::Vertical",
    }
}

fn native_variant(kind: NativeKind) -> &'static str {
    match kind {
        NativeKind::Web => "Web",
        NativeKind::Video => "Video",
        NativeKind::Map => "Map",
        NativeKind::TextInput => "TextInput",
    }
}

fn input_variant(mode: &str) -> &'static str {
    match mode {
        "capture" => "Capture",
        _ => "Passthrough",
    }
}

fn composition_variant(mode: &str) -> &'static str {
    match mode {
        "texture" => "Texture",
        "surface" => "Surface",
        "offscreen" => "Offscreen",
        _ => "Overlay",
    }
}

fn align_variant(word: &str) -> &'static str {
    match word {
        "start" => "Align::Start",
        "center" => "Align::Center",
        "end" => "Align::End",
        _ => "Align::Stretch",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_ir::{Binding, EventBinding, NodeKey, Prop, Slot, ValueType};

    fn doc(nodes: Vec<UiNode>, root: &str) -> UiDocument {
        UiDocument {
            version: crate::ui_ir::IrVersion::CURRENT,
            root: root.into(),
            bindings: vec![
                Binding::handler("save"),
                Binding::value("title", ValueType::Text),
            ],
            nodes,
        }
    }

    fn stack(key: &str, children: Vec<&str>) -> UiNode {
        UiNode {
            key: NodeKey::new(key),
            kind: NodeKind::Stack,
            props: vec![],
            slots: if children.is_empty() {
                vec![]
            } else {
                vec![Slot::default_slot(
                    children.into_iter().map(NodeKey::from).collect(),
                )]
            },
            events: vec![],
        }
    }

    fn text(key: &str, label: &str) -> UiNode {
        UiNode::new(NodeKey::new(key), NodeKind::Text)
            .with_props([Prop::new("label", PropValue::Text(label.into()))])
    }

    fn options() -> EmitOptions {
        EmitOptions::default()
    }

    fn checkbox(key: &str, label: &str) -> UiNode {
        UiNode::new(NodeKey::new(key), NodeKind::Checkbox)
            .with_props([Prop::new("label", PropValue::Text(label.into()))])
    }

    #[test]
    fn the_header_names_only_the_widgets_the_source_provides() {
        // A fixed import list is what broke when the composites moved: the
        // list named types the program could not name, and the program did
        // not compile. The list is built from the widget source now.
        let primitives = header(WidgetSource::Primitives);
        assert!(
            !primitives.contains("Checkbox"),
            "the primitives alone cannot name a checkbox:\n{primitives}"
        );
        assert!(primitives.contains("use floom_widgets::"));

        let composites = header(WidgetSource::WithComposites);
        assert!(
            composites.contains("use floom_ui_composites::{Checkbox"),
            "the composite source must import the composites:\n{composites}"
        );
    }

    #[test]
    fn a_composite_kind_needs_the_composite_widget_source() {
        let document = doc(
            vec![stack("root", vec!["box"]), checkbox("box", "I agree")],
            "root",
        );

        // Without the composites, the kind has no seat: it becomes a
        // placeholder and the substitution is reported.
        let emitted = emit(&document, &options()).expect("lenient emit succeeds");
        assert!(
            !emitted.code.contains("Checkbox::new("),
            "a widget the chosen set cannot name must not be called:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .unmapped
                .iter()
                .any(|u| u.key == "box" && u.reason.contains("Checkbox")),
            "{:?}",
            emitted.unmapped
        );

        // With them, it is a real widget.
        let with_composites = EmitOptions {
            widgets: WidgetSource::WithComposites,
            ..options()
        };
        let emitted = emit(&document, &with_composites).expect("emits");
        assert!(
            emitted.code.contains("Checkbox::new("),
            "the composite source can name it:\n{}",
            emitted.code
        );
        assert!(
            !emitted
                .unmapped
                .iter()
                .any(|u| u.key == "box" && u.reason.contains("Checkbox")),
            "and it is not reported as unmapped:\n{:?}",
            emitted.unmapped
        );
    }

    #[test]
    fn a_simple_document_emits_the_widgets_it_names() {
        let document = doc(
            vec![stack("root", vec!["title"]), text("title", "hello")],
            "root",
        );
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            emitted.code.contains("Stack::new(Axis::Vertical)"),
            "{}",
            emitted.code
        );
        assert!(
            emitted.code.contains("Text::new(\"hello\")"),
            "{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("pub fn build() -> Box<dyn WidgetAny>")
        );
        assert!(emitted.unmapped.is_empty(), "{:?}", emitted.unmapped);
    }

    #[test]
    fn a_repeated_subtree_becomes_one_function() {
        let mut nodes = vec![stack("root", vec!["a", "b", "c", "d"])];
        for key in ["a", "b", "c", "d"] {
            nodes.push(text(key, "same"));
        }
        let document = doc(nodes, "root");
        let emitted = emit(&document, &options()).expect("emits");
        assert_eq!(
            emitted.code.matches("fn shared_").count(),
            1,
            "expected exactly one shared function:\n{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains("for _ in 0..4 { v = v.push(shared_0); }"),
            "the run did not collapse:\n{}",
            emitted.code
        );
    }

    #[test]
    fn distinct_children_are_not_shared() {
        // The control for the test above. Without it, "one shared function"
        // could mean the sharing rule fires unconditionally.
        let mut nodes = vec![stack("root", vec!["a", "b", "c", "d"])];
        for (index, key) in ["a", "b", "c", "d"].into_iter().enumerate() {
            nodes.push(text(key, &format!("label {index}")));
        }
        let document = doc(nodes, "root");
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            !emitted.code.contains("fn shared_"),
            "distinct subtrees must not be shared:\n{}",
            emitted.code
        );
    }

    #[test]
    fn an_axis_and_a_gap_reach_the_stack() {
        let mut root = stack("root", vec!["title"]);
        root.props = vec![
            Prop::new("axis", PropValue::Ident("horizontal".into())),
            Prop::new("gap", PropValue::Number(8.0)),
        ];
        let document = doc(vec![root, text("title", "hi")], "root");
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            emitted
                .code
                .contains("Stack::new(Axis::Horizontal).gap(8.0f32)"),
            "{}",
            emitted.code
        );
    }

    #[test]
    fn an_event_is_wired_and_its_named_handler_is_reported() {
        let mut button = UiNode::new(NodeKey::new("ok"), NodeKind::Button)
            .with_props([Prop::new("label", PropValue::Text("OK".into()))]);
        button.events = vec![EventBinding::new(crate::ui_ir::EventName::Click, "save")];
        let document = doc(vec![button], "ok");
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            emitted.code.contains(".on_click(|| {  })"),
            "{}",
            emitted.code
        );
        assert!(
            emitted
                .unmapped
                .iter()
                .any(|u| u.reason.contains("`click` handler `save`")),
            "the named handler was not reported: {:?}",
            emitted.unmapped
        );
    }

    #[test]
    fn a_kind_with_no_builder_equivalent_is_a_reported_placeholder() {
        let document = doc(
            vec![
                stack("root", vec!["grid"]),
                {
                    let mut grid = UiNode::new(NodeKey::new("grid"), NodeKind::Grid);
                    grid.props = vec![Prop::new("columns", PropValue::Int(3))];
                    grid.slots = vec![Slot::default_slot(vec![NodeKey::from("cell")])];
                    grid
                },
                text("cell", "x"),
            ],
            "root",
        );
        let emitted = emit(&document, &options()).expect("lenient emit succeeds");
        assert!(
            emitted
                .unmapped
                .iter()
                .any(|u| u.key == "grid" && u.reason.contains("no builder equivalent")),
            "{:?}",
            emitted.unmapped
        );
        // Strict mode refuses the same document.
        let strict = EmitOptions {
            strict: true,
            ..EmitOptions::default()
        };
        let error = emit(&document, &strict).expect_err("strict emit refuses");
        assert!(matches!(error, EmitError::Unmapped(_)), "{error:?}");
        // The children survive the placeholder, which is the point of the
        // substitution being a Stack rather than nothing.
        assert!(
            emitted.code.contains("Text::new(\"x\")"),
            "{}",
            emitted.code
        );
    }

    #[test]
    fn a_property_with_no_seat_is_named() {
        let mut node = text("title", "hi");
        node.props.push(Prop::new("radius", PropValue::Number(4.0)));
        let document = doc(vec![node], "title");
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            emitted
                .unmapped
                .iter()
                .any(|u| u.key == "title" && u.reason.contains("`radius`")),
            "{:?}",
            emitted.unmapped
        );
    }

    #[test]
    fn an_invalid_document_is_refused_rather_than_walked() {
        // A self-parenting node is a cycle, and the fingerprint walk recurses.
        // Emitting first and validating never would not terminate.
        let mut node = stack("loop", vec!["loop"]);
        node.slots = vec![Slot::default_slot(vec![NodeKey::from("loop")])];
        let document = doc(vec![node], "loop");
        let error = emit(&document, &options()).expect_err("refused");
        assert!(matches!(error, EmitError::Invalid(_)), "{error:?}");
    }

    #[test]
    fn layout_properties_ride_on_the_parents_push() {
        let mut child = text("title", "hi");
        child
            .props
            .push(Prop::new("width", PropValue::Number(100.0)));
        child.props.push(Prop::new("grow", PropValue::Number(1.0)));
        let document = doc(vec![stack("root", vec!["title"]), child], "root");
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            emitted.code.contains(
                "v = v.push_layout(LayoutRequest::new().width(100.0f32).grow(1.0f32), || {"
            ),
            "{}",
            emitted.code
        );
        assert!(emitted.unmapped.is_empty(), "{:?}", emitted.unmapped);
    }

    #[test]
    fn a_native_participant_keeps_its_composition_words() {
        let mut node = UiNode::new(NodeKey::new("web"), NodeKind::Native(NativeKind::Web));
        node.props = vec![
            Prop::new("composition", PropValue::Ident("surface".into())),
            Prop::new("input_mode", PropValue::Ident("capture".into())),
        ];
        let document = doc(vec![node], "web");
        let emitted = emit(&document, &options()).expect("emits");
        assert!(
            emitted
                .code
                .contains("NativeSurface::new(NativeViewKind::Web)"),
            "{}",
            emitted.code
        );
        assert!(
            emitted.code.contains(".input(InputMode::Capture)"),
            "{}",
            emitted.code
        );
        assert!(
            emitted
                .code
                .contains(".modes(&[NativeCompositionMode::Surface])"),
            "{}",
            emitted.code
        );
        assert!(emitted.unmapped.is_empty(), "{:?}", emitted.unmapped);
    }

    #[test]
    fn the_emitted_code_is_deterministic() {
        // Two runs over the same document must be byte-identical, or a golden
        // of the output would be worthless.
        let mut nodes = vec![stack("root", vec!["a", "b", "c", "d"])];
        for key in ["a", "b", "c", "d"] {
            nodes.push(text(key, "same"));
        }
        let document = doc(nodes, "root");
        assert_eq!(
            emit(&document, &options()).expect("emits"),
            emit(&document, &options()).expect("emits")
        );
    }

    #[test]
    fn repetition_is_sublinear_in_the_copy_count() {
        // The sharp claim behind sharing: ten times the copies must NOT be
        // ten times the code. Identical siblings collapse to a loop whose
        // bound is a couple of characters, so bytes stay essentially flat.
        let identical = |count: usize| {
            let keys: Vec<String> = (0..count).map(|i| format!("t{i}")).collect();
            let mut nodes = vec![stack(
                "root",
                keys.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
            )];
            for key in &keys {
                nodes.push(text(key, "same"));
            }
            emit(&doc(nodes, "root"), &options())
                .expect("emits")
                .code
                .len()
        };
        let ten = identical(10);
        let hundred = identical(100);
        assert!(
            (hundred as f64) < 2.0 * (ten as f64),
            "growing 10 -> 100 identical copies multiplied the code by {}",
            hundred as f64 / ten as f64
        );
    }

    #[test]
    fn the_control_group_distinct_nodes_grow_linearly() {
        // The control for the test above. Without it, "flat" could just mean
        // the emitter silently ignored how many children the document had.
        // Distinct labels cannot share or collapse, so the code must scale
        // with the node count.
        let distinct = |count: usize| {
            let keys: Vec<String> = (0..count).map(|i| format!("t{i}")).collect();
            let mut nodes = vec![stack(
                "root",
                keys.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
            )];
            for (index, key) in keys.iter().enumerate() {
                nodes.push(text(key, &format!("label {index}")));
            }
            emit(&doc(nodes, "root"), &options())
                .expect("emits")
                .code
                .len()
        };
        let ten = distinct(10);
        let hundred = distinct(100);
        let ratio = hundred as f64 / ten as f64;
        assert!(
            ratio > 5.0,
            "10 -> 100 distinct nodes only multiplied the code by {ratio}; \
             the fixture is not varying the document"
        );
    }
}
