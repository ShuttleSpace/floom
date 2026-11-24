// SPDX-License-Identifier: MIT
//! The agent session (Task 25.3): the protocol bound to a running headless
//! app.
//!
//! A session is the server half of the protocol. It owns nothing about
//! transports — a channel, a socket or a test can all hand it requests — and
//! it drives the app exclusively through the testkit's public actors, so an
//! agent's tap is the *same* tap a test's is: same event path, same frame
//! accounting, same damage answer.
//!
//! Boundary: this crate drives and observes; it does not inspect the retained
//! hot-reload tree (that is `floom-devtools`, Phase 23). The two protocols
//! share a wire vocabulary through `floom-inspector` — versions, the scene
//! line format — and nothing else.

use floom_app::WidgetHost;
use floom_core::{SemanticsHint, SemanticsState};
use floom_inspector::SceneLines;
use floom_scene::SceneNode;
use floom_snapshot::{PixelSnapshot, ReferencePixels};
use floom_testkit::{Selector, TestApp};
use floom_types::{Point, Size};

use crate::agent::protocol::{
    AgentError, AgentRequest, AgentResponse, Command, PROTOCOL_VERSION, Payload,
    PerformanceSummary, PointDto, ScreenDto, SemanticsEntryDto, UiNodeInfo, UiTree,
};
use crate::agent::schema::component_schema;

/// What a session can drive. [`TestApp`] is the headless half; a running
/// window implements it over its own `WidgetHost`, which is what makes an
/// agent's commands reach the same event path a human's do.
pub trait AgentApp {
    fn host(&self) -> &WidgetHost;
    fn host_mut(&mut self) -> &mut WidgetHost;
    fn size(&self) -> Size;
    fn scene_nodes(&self) -> Vec<SceneNode>;
    fn labels(&self) -> Vec<String>;
    /// Whether `selector` names something on screen right now.
    fn has(&self, selector: Selector) -> bool;
    fn tap(&mut self, selector: Selector) -> bool;
    fn type_text(&mut self, selector: Selector, text: &str) -> bool;
    fn drag(&mut self, from: Point, to: Point) -> bool;

    /// The node a protocol id names, when there is one on screen with it.
    fn element_of_id(&self, id: u64) -> Option<floom_types::ElementId> {
        let index = usize::try_from(id).ok()?;
        let host = self.host();
        let node = host.node_at(index)?;
        host.live_semantics_hint(node)?;
        Some(host.node_id(node))
    }

    /// Click a node the protocol already resolved to an element.
    fn tap_element(&mut self, element: floom_types::ElementId) -> bool {
        self.host_mut().activate(element)
    }

    /// The node mounted under `key`, looked up by the name the author gave it.
    fn element_of_key(&self, key: &str) -> Option<floom_types::ElementId> {
        let node = self.host().node_by_agent_key(key)?;
        self.host().live_semantics_hint(node)?;
        Some(self.host().node_id(node))
    }
}

/// What a wire selector turned into: either something the testkit can find, or
/// an element the protocol already resolved by id.
enum Target {
    Selector(floom_testkit::Selector),
    Element(floom_types::ElementId),
}

impl AgentApp for TestApp {
    fn host(&self) -> &WidgetHost {
        TestApp::host(self)
    }
    fn host_mut(&mut self) -> &mut WidgetHost {
        TestApp::host_mut(self)
    }
    fn size(&self) -> Size {
        TestApp::size(self)
    }
    fn scene_nodes(&self) -> Vec<SceneNode> {
        TestApp::scene_nodes(self)
    }
    fn labels(&self) -> Vec<String> {
        TestApp::labels(self)
    }
    fn has(&self, selector: Selector) -> bool {
        TestApp::find(self, selector).is_some()
    }
    fn tap(&mut self, selector: Selector) -> bool {
        TestApp::tap(self, selector)
    }
    fn type_text(&mut self, selector: Selector, text: &str) -> bool {
        TestApp::type_text(self, selector, text)
    }
    fn drag(&mut self, from: Point, to: Point) -> bool {
        TestApp::drag(self, from, to)
    }
}

/// The server half of the agent protocol over one app.
pub struct AgentSession<'a, A: AgentApp = TestApp> {
    app: &'a mut A,
    token: Option<String>,
    stats: PerformanceSummary,
}

impl<'a, A: AgentApp> AgentSession<'a, A> {
    pub fn new(app: &'a mut A) -> Self {
        Self {
            app,
            token: None,
            stats: PerformanceSummary::default(),
        }
    }

    /// Require `token` on every request. A tokenless server is for tests; a
    /// development server should have one.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Handle one request, answering `id` with either a payload or the reason.
    pub fn handle(&mut self, request: AgentRequest) -> AgentResponse {
        let result = self.dispatch(&request);
        AgentResponse {
            id: request.id,
            result,
        }
    }

    fn dispatch(&mut self, request: &AgentRequest) -> Result<Payload, AgentError> {
        crate::agent::protocol::check_version(request.version)?;
        if let Some(expected) = &self.token
            && request.token.as_deref() != Some(expected.as_str())
        {
            return Err(AgentError::AuthRequired);
        }
        match &request.command {
            Command::Tree => Ok(Payload::Tree(self.tree())),
            Command::Semantics => Ok(Payload::Semantics(self.semantics())),
            Command::Scene => Ok(Payload::Scene(self.scene())),
            Command::Screenshot => Ok(Payload::Screenshot(self.screenshot())),
            Command::Performance => Ok(Payload::Performance(self.stats)),
            Command::Schema => Ok(Payload::Schema(component_schema())),
            Command::Tap { selector } => {
                let repainted = match self.resolve(selector)? {
                    Target::Selector(selector) => self.app.tap(selector),
                    Target::Element(element) => self.app.tap_element(element),
                };
                self.stats.interactions += 1;
                self.stats.taps += 1;
                if repainted {
                    self.stats.repaints += 1;
                }
                Ok(Payload::Done { repainted })
            }
            Command::TypeText { selector, text } => {
                let repainted = match self.resolve(selector)? {
                    Target::Selector(selector) => self.app.type_text(selector, text),
                    // A node reached by id is still typed into by what it says:
                    // text input needs a field, and text is what addresses one.
                    Target::Element(_) => false,
                };
                self.stats.interactions += 1;
                self.stats.text_commits += 1;
                if repainted {
                    self.stats.repaints += 1;
                }
                Ok(Payload::Done { repainted })
            }
            Command::Drag { from, to } => {
                let repainted = self.app.drag(from.point(), to.point());
                self.stats.interactions += 1;
                self.stats.drags += 1;
                if repainted {
                    self.stats.repaints += 1;
                }
                Ok(Payload::Done { repainted })
            }
        }
    }

    /// Resolve a wire selector against the live screen, refusing nothing
    /// quietly: a miss carries what *is* on screen so an agent can correct
    /// itself rather than re-guess blind.
    fn resolve(&self, selector: &crate::agent::protocol::Selector) -> Result<Target, AgentError> {
        let on_screen = || self.app.labels();
        let candidate = match selector {
            crate::agent::protocol::Selector::Id(id) => {
                return match self.app.element_of_id(*id) {
                    Some(element) => Ok(Target::Element(element)),
                    None => Err(AgentError::SelectorNotFound {
                        selector: selector.to_string(),
                        on_screen: on_screen(),
                    }),
                };
            }
            crate::agent::protocol::Selector::Key(key) => {
                return match self.app.element_of_key(key) {
                    Some(element) => Ok(Target::Element(element)),
                    None => Err(AgentError::SelectorNotFound {
                        selector: selector.to_string(),
                        on_screen: on_screen(),
                    }),
                };
            }
            crate::agent::protocol::Selector::Label(label) => {
                floom_testkit::Selector::Label(label.clone())
            }
            crate::agent::protocol::Selector::Role(name) => {
                let Some(role) = self.find_role(name) else {
                    return Err(AgentError::SelectorNotFound {
                        selector: selector.to_string(),
                        on_screen: on_screen(),
                    });
                };
                floom_testkit::Selector::Role(role)
            }
        };
        if self.app.has(candidate.clone()) {
            Ok(Target::Selector(candidate))
        } else {
            Err(AgentError::SelectorNotFound {
                selector: selector.to_string(),
                on_screen: on_screen(),
            })
        }
    }

    /// The role name of the first live node publishing one that matches
    /// `name` case-insensitively, so the wire can say `"checkbox"`.
    fn find_role(&self, name: &str) -> Option<floom_core::SemanticsRole> {
        let host = self.app.host();
        (0..host.node_count())
            .filter_map(|index| host.node_at(index))
            .filter_map(|node| host.live_semantics_hint(node))
            .find(|hint| role_name(hint.role) == name.to_lowercase())
            .map(|hint| hint.role)
    }

    fn live_nodes(&self) -> Vec<(u64, SemanticsHint, floom_types::Rect)> {
        self.live_keyed_nodes()
            .into_iter()
            .map(|(id, _, hint, bounds)| (id, hint, bounds))
            .collect()
    }

    fn live_keyed_nodes(
        &self,
    ) -> Vec<(u64, Option<&'static str>, SemanticsHint, floom_types::Rect)> {
        let host = self.app.host();
        (0..host.node_count())
            .filter_map(|index| host.node_at(index))
            .enumerate()
            .filter_map(|(order, node)| {
                let hint = host.live_semantics_hint(node)?;
                let bounds = host.bounds(node);
                Some((order as u64, host.agent_key(node), hint, bounds))
            })
            .collect()
    }

    fn tree(&self) -> UiTree {
        let nodes = self
            .live_keyed_nodes()
            .into_iter()
            .map(|(order, key, hint, bounds)| UiNodeInfo {
                id: order,
                key: key.map(str::to_string),
                role: role_name(hint.role),
                label: hint.label,
                checked: hint.state.contains(SemanticsState::CHECKED),
                disabled: hint.state.contains(SemanticsState::DISABLED),
                focused: hint.state.contains(SemanticsState::FOCUSED),
                x: bounds.x0,
                y: bounds.y0,
                width: bounds.width(),
                height: bounds.height(),
            })
            .collect();
        UiTree { nodes }
    }

    fn semantics(&self) -> Vec<SemanticsEntryDto> {
        self.live_nodes()
            .into_iter()
            .map(|(_, hint, _)| SemanticsEntryDto {
                role: role_name(hint.role),
                label: hint.label,
                checked: hint.state.contains(SemanticsState::CHECKED),
                disabled: hint.state.contains(SemanticsState::DISABLED),
                focused: hint.state.contains(SemanticsState::FOCUSED),
            })
            .collect()
    }

    fn scene(&self) -> SceneLines {
        SceneLines {
            version: PROTOCOL_VERSION,
            lines: self
                .app
                .scene_nodes()
                .iter()
                .map(|node| format!("{node:?}"))
                .collect(),
        }
    }

    fn screenshot(&self) -> ScreenDto {
        let shot = PixelSnapshot::of(self.app.host().scene(), None, self.app.size(), 1.0);
        ScreenDto {
            width: shot.width,
            height: shot.height,
            hash: shot.hash,
        }
    }
}

/// The wire spelling of a role: the Debug name, lowercased, so
/// `SemanticsRole::CheckBox` is served as `"checkbox"`.
fn role_name(role: floom_core::SemanticsRole) -> String {
    format!("{role:?}").to_lowercase()
}

impl PointDto {
    fn point(self) -> Point {
        Point::new(self.x, self.y)
    }
}
