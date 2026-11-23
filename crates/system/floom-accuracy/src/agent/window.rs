// SPDX-License-Identifier: MIT
//! The same protocol, driven against a *running* window.
//!
//! [`WindowApp`] is the [`AgentApp`] implementation for a real `WidgetHost`:
//! an agent's tap lands on the same event path a human's does (the host
//! dispatches a pointer down/up at the element's centre), and its screen
//! report comes from the same scene the renderer draws.
//!
//! Two commands are refused rather than faked: `TypeText` needs a focused text
//! field's input session and `Drag` a pointer path with timing, and inventing
//! either here would teach an agent a gesture the app never receives.

use floom_app::WidgetHost;
use floom_scene::SceneNode;
use floom_testkit::Selector;
use floom_types::{Point, Size};

use crate::agent::session::AgentApp;

/// A live window as the agent protocol sees it.
pub struct WindowApp<'a> {
    host: &'a mut WidgetHost,
    size: Size,
}

impl<'a> WindowApp<'a> {
    /// `size` is the surface the window is drawing at, which the host's layout
    /// may disagree with for a frame after a resize.
    pub fn new(host: &'a mut WidgetHost, size: Size) -> Self {
        Self { host, size }
    }

    /// The element a wire selector names, or why nothing matched.
    fn element(&self, selector: &Selector) -> Option<floom_types::ElementId> {
        self.host
            .semantics()
            .iter()
            .find(|(_, node)| match selector {
                Selector::Label(label) => node.label == *label,
                // The two role enums (`floom_core` and `floom_accessibility`)
                // spell variants identically; comparing the names keeps this
                // from growing a 23-arm translation table that would drift.
                Selector::Role(role) => {
                    format!("{:?}", node.role).to_lowercase() == format!("{role:?}").to_lowercase()
                }
                // Node ids are handles the wire never carries (see the module
                // docs of `agent`), so nothing can address one.
                Selector::Id(_) => false,
            })
            .map(|(id, _)| id)
    }
}

impl AgentApp for WindowApp<'_> {
    fn host(&self) -> &WidgetHost {
        self.host
    }
    fn host_mut(&mut self) -> &mut WidgetHost {
        self.host
    }

    fn size(&self) -> Size {
        self.size
    }

    fn scene_nodes(&self) -> Vec<SceneNode> {
        self.host
            .scene()
            .iter()
            .map(|(_, node)| node.clone())
            .collect()
    }

    fn labels(&self) -> Vec<String> {
        self.host
            .semantics()
            .iter()
            .map(|(_, node)| node.label.clone())
            .filter(|label| !label.is_empty())
            .collect()
    }

    fn has(&self, selector: Selector) -> bool {
        self.element(&selector).is_some()
    }

    fn tap(&mut self, selector: Selector) -> bool {
        let Some(element) = self.element(&selector) else {
            return false;
        };
        // `activate` dispatches press and release at the element's centre, so
        // the answer to "did the app react" is the host's frame request, not a
        // repaint this function performs.
        self.host.activate(element);
        self.host.needs_frame()
    }

    fn type_text(&mut self, _selector: Selector, _text: &str) -> bool {
        false
    }

    fn drag(&mut self, _from: Point, _to: Point) -> bool {
        false
    }
}
