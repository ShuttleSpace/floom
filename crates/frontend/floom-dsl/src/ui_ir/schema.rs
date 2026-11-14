// SPDX-License-Identifier: MIT
//! The property and event tables: what each kind accepts, and what it emits.
//!
//! Table-driven on purpose. The alternative — validating each kind in its own
//! `match` arm — is a second implementation of the vocabulary that drifts the
//! moment a widget gains a property, and the drift is invisible because nothing
//! cross-checks the two. A single table can be audited (`NodeKind::ALL` × the
//! table), and the DSL, the validator and the codegen all read the same rows.

use crate::ui_ir::node::{EventName, NodeKind};
use crate::ui_ir::value::ValueType;

/// Which layer a property belongs to.
///
/// The split is what lets a lowering step route `radius` to `floom_style` and
/// `grow` to `floom_core::LayoutRequest` without knowing property names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropClass {
    /// Style properties (`floom_style::StyleProps` vocabulary).
    Style,
    /// Layout intent (`floom_core::LayoutRequest` vocabulary).
    Layout,
    /// A property of the widget itself.
    Widget,
}

/// What a property accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropSpec {
    pub class: PropClass,
    pub ty: ValueType,
    /// Non-empty when the value must be an identifier from this set.
    pub domain: &'static [&'static str],
}

#[derive(Debug, Clone, Copy)]
struct Row {
    name: &'static str,
    ty: ValueType,
    domain: &'static [&'static str],
}

const NONE: &[&str] = &[];
const AXIS: &[&str] = &["horizontal", "vertical"];
const ALIGN: &[&str] = &["stretch", "start", "center", "end"];
const POSITION: &[&str] = &["relative", "absolute"];
const CONTROL_SIZE: &[&str] = &["sm", "md", "lg"];
const VARIANT: &[&str] = &["filled", "outlined", "text", "elevated", "tonal"];
const FIT: &[&str] = &["fill", "contain", "cover", "none"];
const COMPOSITION: &[&str] = &["overlay", "texture", "surface", "offscreen"];
const INPUT_MODE: &[&str] = &["passthrough", "capture"];

const fn row(name: &'static str, ty: ValueType) -> Row {
    Row {
        name,
        ty,
        domain: NONE,
    }
}

const fn row_in(name: &'static str, ty: ValueType, domain: &'static [&'static str]) -> Row {
    Row { name, ty, domain }
}

/// Style properties. Every kind has them: a box is a box.
const STYLE_PROPS: &[Row] = &[
    row("background", ValueType::Color),
    row("foreground", ValueType::Color),
    row("font_size", ValueType::Number),
    row("opacity", ValueType::Number),
    // A single value, matching `floom_style::StyleProps::padding`; the lowering
    // step expands it for the widgets that want four sides.
    row("padding", ValueType::Number),
    row("radius", ValueType::Number),
    row("border_width", ValueType::Number),
    row("border_color", ValueType::Color),
    row("elevation", ValueType::Number),
];

/// Layout properties. Every kind has them, including leaves — a `Text` can be
/// given a width.
const LAYOUT_PROPS: &[Row] = &[
    row("width", ValueType::Number),
    row("height", ValueType::Number),
    row("grow", ValueType::Number),
    row_in("position", ValueType::Ident, POSITION),
    row("inset_left", ValueType::Number),
    row("inset_top", ValueType::Number),
    row("inset_right", ValueType::Number),
    row("inset_bottom", ValueType::Number),
    // Flow intent — the rest of the `LayoutRequest` vocabulary. These used to
    // live as Stack widget properties, which silently dropped them from every
    // layout table: the Phase 23 inspector read a mounted Stack back and its
    // `gap` was gone.
    row_in("axis", ValueType::Ident, AXIS),
    row("gap", ValueType::Number),
    row_in("main_align", ValueType::Ident, ALIGN),
    row_in("cross_align", ValueType::Ident, ALIGN),
];

/// Properties of a native participant. These mirror `floom_types`, so the same
/// document describes a WebView to a platform that has one and to a tool that
/// merely renders a placeholder.
const NATIVE_PROPS: &[Row] = &[
    row_in("composition", ValueType::Ident, COMPOSITION),
    row_in("input_mode", ValueType::Ident, INPUT_MODE),
];

/// Properties declared per kind.
///
/// Note that a name may appear twice with different types: `size` is a font
/// size on `Text` and a control metric on `Button`. That is why the table is
/// keyed by `(kind, name)` and not by name alone — a name-only table would have
/// to pick one type and would then reject half the documents that are correct.
const WIDGET_PROPS: &[(NodeKind, Row)] = &[
    (NodeKind::Text, row("label", ValueType::Text)),
    (NodeKind::Text, row("size", ValueType::Number)),
    (NodeKind::Icon, row("glyph", ValueType::Text)),
    (NodeKind::Icon, row("size", ValueType::Number)),
    (NodeKind::Image, row("resource", ValueType::Text)),
    (NodeKind::Image, row_in("fit", ValueType::Ident, FIT)),
    (NodeKind::Image, row("alt", ValueType::Text)),
    (NodeKind::Button, row("label", ValueType::Text)),
    (
        NodeKind::Button,
        row_in("variant", ValueType::Ident, VARIANT),
    ),
    (
        NodeKind::Button,
        row_in("size", ValueType::Ident, CONTROL_SIZE),
    ),
    (NodeKind::Button, row("disabled", ValueType::Bool)),
    (NodeKind::Checkbox, row("label", ValueType::Text)),
    (NodeKind::Checkbox, row("checked", ValueType::Bool)),
    (NodeKind::Checkbox, row("disabled", ValueType::Bool)),
    (NodeKind::Radio, row("label", ValueType::Text)),
    (NodeKind::Radio, row("checked", ValueType::Bool)),
    (NodeKind::Radio, row("disabled", ValueType::Bool)),
    (NodeKind::Switch, row("label", ValueType::Text)),
    (NodeKind::Switch, row("checked", ValueType::Bool)),
    (NodeKind::Switch, row("disabled", ValueType::Bool)),
    (NodeKind::Slider, row("value", ValueType::Number)),
    (NodeKind::Slider, row("min", ValueType::Number)),
    (NodeKind::Slider, row("max", ValueType::Number)),
    (NodeKind::Slider, row("disabled", ValueType::Bool)),
    (NodeKind::Progress, row("value", ValueType::Number)),
    (NodeKind::Progress, row("min", ValueType::Number)),
    (NodeKind::Progress, row("max", ValueType::Number)),
    // The ring and the bar take the same fraction and draw differently
    // enough that the satellite ships them as separate components; the
    // vocabulary says the same.
    (NodeKind::ProgressCircular, row("value", ValueType::Number)),
    (
        NodeKind::ProgressCircular,
        row("indeterminate", ValueType::Bool),
    ),
    (NodeKind::ProgressCircular, row("size", ValueType::Number)),
    (NodeKind::ProgressLinear, row("value", ValueType::Number)),
    (
        NodeKind::ProgressLinear,
        row("indeterminate", ValueType::Bool),
    ),
    // A date grid, its picker, its time counterpart, a colour selector and a
    // trend line: each is a neutral control the satellite ships as a component,
    // and each is not a progress bar or any other primitive.
    (NodeKind::Calendar, row("value", ValueType::Text)),
    (NodeKind::Calendar, row("min", ValueType::Text)),
    (NodeKind::Calendar, row("max", ValueType::Text)),
    (NodeKind::DatePicker, row("value", ValueType::Text)),
    (NodeKind::DatePicker, row("placeholder", ValueType::Text)),
    (NodeKind::TimePicker, row("value", ValueType::Text)),
    (NodeKind::ColorPicker, row("value", ValueType::Text)),
    (NodeKind::Sparkline, row("data", ValueType::List)),
    (NodeKind::Select, row("value", ValueType::Text)),
    (NodeKind::Select, row("placeholder", ValueType::Text)),
    (NodeKind::Select, row("disabled", ValueType::Bool)),
    (NodeKind::TextField, row("value", ValueType::Text)),
    (NodeKind::TextField, row("placeholder", ValueType::Text)),
    (NodeKind::TextField, row("disabled", ValueType::Bool)),
    (NodeKind::Grid, row("columns", ValueType::Int)),
    (NodeKind::Grid, row("column_gap", ValueType::Number)),
    (NodeKind::Grid, row("row_gap", ValueType::Number)),
    (NodeKind::List, row("gap", ValueType::Number)),
    (NodeKind::List, row("item_height", ValueType::Number)),
    (NodeKind::Conditional, row("when", ValueType::Bool)),
    (NodeKind::Overlay, row("present", ValueType::Bool)),
    (NodeKind::Dialog, row("present", ValueType::Bool)),
    (NodeKind::Dialog, row("title", ValueType::Text)),
    (NodeKind::Popup, row("present", ValueType::Bool)),
    (NodeKind::Tabs, row("selected", ValueType::Int)),
];

/// Which events each kind emits.
///
/// Deliberately narrow. A `Text` emits nothing; treating every view as
/// clickable would make "the handler is never called" unfalsifiable.
const EVENTS: &[(NodeKind, EventName)] = &[
    (NodeKind::Button, EventName::Click),
    (NodeKind::Image, EventName::Click),
    (NodeKind::Icon, EventName::Click),
    (NodeKind::Container, EventName::Click),
    (NodeKind::Checkbox, EventName::Toggle),
    (NodeKind::Switch, EventName::Toggle),
    (NodeKind::Radio, EventName::Toggle),
    (NodeKind::Slider, EventName::Change),
    (NodeKind::Select, EventName::Change),
    (NodeKind::Tabs, EventName::Change),
    (NodeKind::Calendar, EventName::Change),
    (NodeKind::DatePicker, EventName::Change),
    (NodeKind::TimePicker, EventName::Change),
    (NodeKind::ColorPicker, EventName::Change),
    (NodeKind::TextField, EventName::Input),
    (NodeKind::TextField, EventName::Submit),
    (NodeKind::TextField, EventName::Focus),
    (NodeKind::TextField, EventName::Blur),
    (NodeKind::List, EventName::Scroll),
    (NodeKind::Overlay, EventName::Dismiss),
    (NodeKind::Dialog, EventName::Dismiss),
    (NodeKind::Popup, EventName::Dismiss),
];

fn native_row(name: &str) -> Option<&'static Row> {
    NATIVE_PROPS.iter().find(|r| r.name == name)
}

/// The spec for `name` on `kind`, or `None` when the pair is not allowed.
///
/// `None` is ambiguous on purpose between "unknown property" and "not this
/// kind"; [`prop_is_declared`] tells the two apart so the error can say which.
pub fn prop_spec(kind: NodeKind, name: &str) -> Option<PropSpec> {
    let wrap = |class: PropClass, r: &Row| PropSpec {
        class,
        ty: r.ty,
        domain: r.domain,
    };
    if let Some(r) = STYLE_PROPS.iter().find(|r| r.name == name) {
        return Some(wrap(PropClass::Style, r));
    }
    if let Some(r) = LAYOUT_PROPS.iter().find(|r| r.name == name) {
        return Some(wrap(PropClass::Layout, r));
    }
    if matches!(kind, NodeKind::Native(_)) {
        return native_row(name).map(|r| wrap(PropClass::Widget, r));
    }
    WIDGET_PROPS
        .iter()
        .find(|(k, r)| *k == kind && r.name == name)
        .map(|(_, r)| wrap(PropClass::Widget, r))
}

/// Whether any kind declares `name` at all.
pub fn prop_is_declared(name: &str) -> bool {
    STYLE_PROPS.iter().any(|r| r.name == name)
        || LAYOUT_PROPS.iter().any(|r| r.name == name)
        || NATIVE_PROPS.iter().any(|r| r.name == name)
        || WIDGET_PROPS.iter().any(|(_, r)| r.name == name)
}

/// Every property name the tables declare, in declaration order, deduplicated.
///
/// The schema emitter (Phase 25.2) walks this list against each kind, so a
/// property a validator accepts but a schema omits would be exactly the
/// "drift between two implementations of the vocabulary" this crate exists to
/// make impossible. Deduplication matters: `size` is declared twice (Text and
/// Button) and must be probed once per kind, with the kind picking the row.
pub fn all_prop_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    let mut push = |name: &'static str| {
        if !names.contains(&name) {
            names.push(name);
        }
    };
    for r in STYLE_PROPS {
        push(r.name);
    }
    for r in LAYOUT_PROPS {
        push(r.name);
    }
    for r in NATIVE_PROPS {
        push(r.name);
    }
    for (_, r) in WIDGET_PROPS {
        push(r.name);
    }
    names
}

/// Whether `kind` emits `event`.
pub fn emits(kind: NodeKind, event: EventName) -> bool {
    EVENTS.iter().any(|(k, e)| *k == kind && *e == event)
}

/// Every event `kind` emits.
pub fn events_for(kind: NodeKind) -> impl Iterator<Item = EventName> {
    EVENTS
        .iter()
        .filter(move |(k, _)| *k == kind)
        .map(|(_, e)| *e)
}

/// The kinds that declare a given property name.
///
/// Used to turn "not allowed here" into "…it belongs to Checkbox, Switch",
/// which is the difference between an error an author can act on and one they
/// have to go read the source for.
pub fn kinds_declaring(name: &str) -> Vec<NodeKind> {
    if NATIVE_PROPS.iter().any(|r| r.name == name) {
        return NodeKind::ALL
            .iter()
            .copied()
            .filter(|k| matches!(k, NodeKind::Native(_)))
            .collect();
    }
    WIDGET_PROPS
        .iter()
        .filter(|(_, r)| r.name == name)
        .map(|(k, _)| *k)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_can_be_addressed_by_the_tables() {
        // The tables must be *usable* for every kind, not merely present: a kind
        // with no style/layout spec would make every document about it invalid.
        for kind in NodeKind::ALL {
            for name in ["background", "width"] {
                let spec =
                    prop_spec(*kind, name).unwrap_or_else(|| panic!("{kind} has no `{name}`"));
                assert!(
                    matches!(spec.class, PropClass::Style | PropClass::Layout),
                    "{kind} `{name}` is a widget property"
                );
            }
        }
    }

    #[test]
    fn a_widget_property_is_not_universal() {
        // The counterpart to the test above: the per-kind table must actually
        // discriminate, or "not allowed for this kind" could never fire.
        assert!(prop_spec(NodeKind::Button, "variant").is_some());
        assert!(prop_spec(NodeKind::Text, "variant").is_none());
        assert!(prop_is_declared("variant"), "…but it is a known name");
    }

    #[test]
    fn the_same_name_may_have_two_types() {
        assert_eq!(
            prop_spec(NodeKind::Text, "size").map(|s| s.ty),
            Some(ValueType::Number)
        );
        assert_eq!(
            prop_spec(NodeKind::Button, "size").map(|s| s.ty),
            Some(ValueType::Ident)
        );
    }

    #[test]
    fn identifiers_carry_their_domain() {
        let axis = prop_spec(NodeKind::Stack, "axis").expect("axis");
        assert_eq!(axis.domain, AXIS);
        let label = prop_spec(NodeKind::Text, "label").expect("label");
        assert!(label.domain.is_empty(), "free text has no domain");
    }

    #[test]
    fn only_container_kinds_declare_slots_in_practice() {
        // Not a rule the validator enforces (an empty slot list is harmless),
        // but the kinds we call containers should be the ones a document nests
        // under, and this keeps the list from rotting.
        assert!(NodeKind::Stack.is_container());
        assert!(NodeKind::Dialog.is_container());
        assert!(!NodeKind::Text.is_container());
        assert!(!NodeKind::Spacer.is_container());
    }

    #[test]
    fn all_prop_names_is_deduplicated_and_complete() {
        let names = all_prop_names();
        let unique = names.len();
        let expected = names.iter().collect::<std::collections::HashSet<_>>().len();
        assert_eq!(
            unique, expected,
            "`size` is declared twice and must not repeat"
        );
        // Both spellings of the double-declared name resolve through the list.
        assert!(names.contains(&"size"));
        assert!(names.contains(&"variant"));
        assert!(names.contains(&"background"));
        // And everything the tables declare is reachable through it.
        for (_, row) in WIDGET_PROPS {
            assert!(names.contains(&row.name), "{} missing", row.name);
        }
    }

    #[test]
    fn a_leaf_emits_nothing() {
        assert!(!emits(NodeKind::Text, EventName::Click));
        assert_eq!(events_for(NodeKind::Text).count(), 0);
        assert_eq!(events_for(NodeKind::TextField).count(), 4);
    }
}
