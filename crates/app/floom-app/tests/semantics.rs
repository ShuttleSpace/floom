// SPDX-License-Identifier: MIT
//! The semantics tree reaches the platform's adapter, geometry included.
//!
//! This is the seam every assistive-technology backend needs. Before it, an
//! adapter knew a button's label and had no idea where the button was — which
//! is exactly what the iOS gate recorded: labels present, every frame zero.

use std::cell::RefCell;
use std::rc::Rc;

use floom_accessibility::{
    AccessibilityAdapter, AccessibilityCapabilities, SemanticsAction, SemanticsDiff, SemanticsRole,
};
use floom_app::AppRunner;
use floom_core::{ElementId, Length};
use floom_layout::LayoutStyle;
use floom_platform::{HostPlatform, Platform, WindowConfig};
use floom_renderer_software::SoftwarePresenter;
use floom_types::Size;
use floom_widgets::controls::Button;

/// Records every diff it is handed, so a test can ask what the OS layer would
/// have seen.
#[derive(Clone, Default)]
struct Recorder {
    diffs: Rc<RefCell<Vec<SemanticsDiff>>>,
}

impl AccessibilityAdapter for Recorder {
    fn update(&mut self, diff: &SemanticsDiff) {
        self.diffs.borrow_mut().push(diff.clone());
    }

    fn capabilities(&self) -> AccessibilityCapabilities {
        AccessibilityCapabilities::SCREEN_READER | AccessibilityCapabilities::NATIVE_BRIDGE
    }

    fn perform_action(&mut self, _node: ElementId, _action: SemanticsAction) -> bool {
        false
    }

    fn reset(&mut self) {}
}

/// A 200x32 window whose root is one button, attached to a platform that has an
/// adapter.
fn app_with_adapter() -> (AppRunner<SoftwarePresenter>, HostPlatform, Recorder) {
    let size = Size::new(200.0, 32.0);
    let recorder = Recorder::default();
    let mut platform = HostPlatform::new().with_accessibility(Box::new(recorder.clone()));
    let mut host = floom_app::WidgetHost::new();
    host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(32.0)),
            ..LayoutStyle::default()
        },
        Button::new("Ping"),
    );
    let runner = AppRunner::new(host, SoftwarePresenter::new());
    let _window = platform.create_window(WindowConfig {
        title: "semantics".into(),
        size,
        resizable: false,
    });
    (runner, platform, recorder)
}

#[test]
fn an_adapter_receives_the_button_with_the_bounds_layout_gave_it() {
    let (mut runner, mut platform, recorder) = app_with_adapter();
    for _ in 0..6 {
        runner.pump(&mut platform);
    }

    let diffs = recorder.diffs.borrow();
    let node = diffs
        .iter()
        .flat_map(|diff| diff.added.iter())
        .find(|(_, node)| node.label == "Ping")
        .map(|(_, node)| node.clone())
        .expect("the button reached the adapter at all");

    assert_eq!(node.role, SemanticsRole::Button);
    assert!(
        node.bounds.width() > 0.0 && node.bounds.height() > 0.0,
        "a node with zero geometry is one a screen reader can name and not touch: {:?}",
        node.bounds
    );
}

#[test]
fn a_backend_without_an_adapter_keeps_the_tree_instead_of_panicking() {
    let mut host = floom_app::WidgetHost::new();
    host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(32.0)),
            ..LayoutStyle::default()
        },
        Button::new("Ping"),
    );
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());
    let mut platform = HostPlatform::new();
    let _window = platform.create_window(WindowConfig {
        title: "semantics".into(),
        size: Size::new(200.0, 32.0),
        resizable: false,
    });
    for _ in 0..6 {
        runner.pump(&mut platform);
    }

    let tree = runner.host().semantics();
    assert_eq!(
        tree.node_count(),
        1,
        "the tree is built whether or not anyone listens"
    );
    let (_, node) = tree.iter().next().expect("one node");
    assert!(node.bounds.width() > 0.0);
}

#[test]
fn activating_a_semantics_node_runs_what_a_click_would() {
    // The whole point of a screen reader is that its double-tap reaches the
    // same handler a finger would. Deliver it as a click at the node's centre,
    // so a widget needs no a11y-specific path that could drift from its own.
    use std::cell::Cell;

    let hits = Rc::new(Cell::new(0));
    let counter = hits.clone();
    let mut host = floom_app::WidgetHost::new();
    host.mount_root(
        LayoutStyle {
            width: Some(Length::Px(200.0)),
            height: Some(Length::Px(32.0)),
            ..LayoutStyle::default()
        },
        Button::new("Ping").on_click(move || counter.set(counter.get() + 1)),
    );
    host.frame();

    let (id, _) = host
        .semantics()
        .iter()
        .next()
        .map(|(id, node)| (id, node.clone()))
        .expect("the button publishes itself");
    assert!(
        host.activate(id),
        "an accessible button must report the activation"
    );
    assert_eq!(hits.get(), 1, "the click handler must have run once");
    assert!(
        !host.activate(ElementId::new(9999, 1)),
        "a node that is gone must not report a delivery"
    );
}
