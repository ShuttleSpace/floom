//! The demo registry — the one list every backend mounts.
//!
//! A [`Demo`] is a name, a section, and a *builder*: a replayable
//! `fn() -> Box<dyn WidgetAny>` that constructs a fresh widget tree. The host
//! owns the live widgets, so a builder is the only thing a registry can hold —
//! the same reason `Child::new` takes a closure rather than a value.
//!
//! The registry is deliberately the single source of truth: the headless
//! runner, the desktop window and the Android app all call [`gallery_root`],
//! so a demo that renders in CI is the same demo that renders on screen.
//!
//! The registry is split into one module per [`Section`], each exposing a
//! `demos()` list, so a section can be read (and added to) on its own.

use std::sync::OnceLock;

use floem_core::{Axis, Child, ChildKey, LayoutRequest, WidgetAny};
use floem_widgets::{Spacer, Stack, Text};

mod composites;
mod controls;
mod data;
mod forms;
mod motion;
mod navigation;
mod primitives;
mod styling;

/// The gallery's top-level grouping. Every [`Demo`] names exactly one, and the
/// root stack renders them in [`Section::ALL`] order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    /// `Text`, `Icon`, `Divider`, `Spacer`, `Container`, `Canvas`.
    Primitives,
    /// `Button`, `Checkbox`, `Radio`, `Switch`, `Slider`, `Select`, `TextField`,
    /// `Progress`, `Spinner`.
    Controls,
    /// `Stack`, `List`, `Grid`, `Tabs`, `Dialog`, `Overlay`, `Conditional`.
    Composites,
    /// `Crossfade`, `AnimatedContent`, `Opacity`, `SharedTransitionLayout`,
    /// `floem-animation` tweens and easing.
    Motion,
    /// `NavHost`, `NavDestination`, `NavigationBar`/`Drawer`/`Rail`.
    Navigation,
    /// `floem-widgets-data` grids and charts.
    Data,
    /// Validation, dirty/touched tracking, control bindings.
    Forms,
    /// Gradient/shadow/filter tokens and the design-system themes.
    Styling,
}

impl Section {
    /// Rendering order of the root stack.
    pub const ALL: [Section; 8] = [
        Section::Primitives,
        Section::Controls,
        Section::Composites,
        Section::Motion,
        Section::Navigation,
        Section::Data,
        Section::Forms,
        Section::Styling,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Section::Primitives => "Primitives",
            Section::Controls => "Controls",
            Section::Composites => "Composites",
            Section::Motion => "Motion",
            Section::Navigation => "Navigation",
            Section::Data => "Data",
            Section::Forms => "Forms",
            Section::Styling => "Styling",
        }
    }
}

/// A replayable demo: the builder is called once per mount.
#[derive(Clone)]
pub struct Demo {
    /// Stable id, unique across the registry (used for lookups and trace lines).
    pub id: &'static str,
    /// Human-readable title shown above the demo.
    pub title: &'static str,
    pub section: Section,
    /// Constructs a fresh subtree. `fn` pointers are `Send + Sync + 'static`,
    /// which is what a `Child` slot requires.
    pub build: fn() -> Box<dyn WidgetAny>,
}

/// The whole registry, in declaration order: sections in [`Section::ALL`] order,
/// each section's demos in the order that section declares them.
fn registry() -> &'static [Demo] {
    static REGISTRY: OnceLock<Vec<Demo>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut all = Vec::new();
        for section in [
            primitives::demos(),
            controls::demos(),
            composites::demos(),
            motion::demos(),
            navigation::demos(),
            data::demos(),
            forms::demos(),
            styling::demos(),
        ] {
            all.extend(section);
        }
        all
    })
}

/// The whole registry, in section then declaration order.
pub fn demos() -> &'static [Demo] {
    registry()
}

/// The demos in one section, in declaration order.
pub fn section_demos(section: Section) -> impl Iterator<Item = &'static Demo> {
    registry().iter().filter(move |demo| demo.section == section)
}

/// A heading row for a section, keyed by `key`.
fn section_header(key: usize, section: Section) -> Child {
    Child::new(ChildKey::index(key), LayoutRequest::new(), move || {
        Box::new(Text::new(section.title()).size(20.0))
    })
}

/// The full gallery: every section header followed by its demos, stacked
/// vertically. This is the tree both the window and the Android app mount.
pub fn gallery_root() -> Stack {
    let mut stack = Stack::new(Axis::Vertical).gap(18.0);
    let mut key = 0usize;
    for section in Section::ALL {
        if section_demos(section).next().is_none() {
            continue;
        }
        stack = stack.child(section_header(key, section));
        key += 1;
        for demo in section_demos(section) {
            stack = stack.child(Child::new(
                ChildKey::index(key),
                LayoutRequest::new(),
                demo.build,
            ));
            key += 1;
        }
    }
    stack
}

/// One demo wrapped in a stack, so its builder's `Box<dyn WidgetAny>` can be
/// mounted by the headless runner (a mount point takes `impl WidgetAny`, and a
/// boxed trait object is not one).
pub fn demo_root(demo: &'static Demo) -> Stack {
    Stack::new(Axis::Vertical)
        .child(Child::new(ChildKey::index(0), LayoutRequest::new(), demo.build))
        .child(Child::new(
            ChildKey::index(1),
            LayoutRequest::new(),
            || Box::new(Spacer::new(0.0)),
        ))
}
