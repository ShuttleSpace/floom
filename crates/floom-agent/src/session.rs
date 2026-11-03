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

use floom_core::{SemanticsHint, SemanticsState};
use floom_inspector::SceneLines;
use floom_snapshot::{PixelProbe, PixelSnapshot, ReferencePixels};
use floom_testkit::TestApp;
use floom_types::Point;

use crate::protocol::{
    AgentError, AgentRequest, AgentResponse, Command, PROTOCOL_VERSION, Payload,
    PerformanceSummary, PointDto, ScreenDto, SemanticsEntryDto, UiNodeInfo, UiTree,
};
use crate::schema::component_schema;

/// The server half of the agent protocol over one headless app.
pub struct AgentSession<'a> {
    app: &'a mut TestApp,
    token: Option<String>,
    stats: PerformanceSummary,
}

impl<'a> AgentSession<'a> {
    pub fn new(app: &'a mut TestApp) -> Self {
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
        crate::protocol::check_version(request.version)?;
        if let Some(expected) = &self.token {
            if request.token.as_deref() != Some(expected.as_str()) {
                return Err(AgentError::AuthRequired);
            }
        }
        match &request.command {
            Command::Tree => Ok(Payload::Tree(self.tree())),
            Command::Semantics => Ok(Payload::Semantics(self.semantics())),
            Command::Scene => Ok(Payload::Scene(self.scene())),
            Command::Screenshot => Ok(Payload::Screenshot(self.screenshot())),
            Command::Performance => Ok(Payload::Performance(self.stats)),
            Command::Schema => Ok(Payload::Schema(component_schema())),
            Command::Tap { selector } => {
                let selector = self.resolve(selector)?;
                let repainted = self.app.tap(selector);
                self.stats.interactions += 1;
                self.stats.taps += 1;
                if repainted {
                    self.stats.repaints += 1;
                }
                Ok(Payload::Done { repainted })
            }
            Command::TypeText { selector, text } => {
                let selector = self.resolve(selector)?;
                let repainted = self.app.type_text(selector, text);
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
    fn resolve(
        &self,
        selector: &crate::protocol::Selector,
    ) -> Result<floom_testkit::Selector, AgentError> {
        let on_screen = || self.app.labels();
        let candidate = match selector {
            crate::protocol::Selector::Label(label) => {
                floom_testkit::Selector::Label(label.clone())
            }
            crate::protocol::Selector::Role(name) => {
                let Some(role) = self.find_role(name) else {
                    return Err(AgentError::SelectorNotFound {
                        selector: selector.to_string(),
                        on_screen: on_screen(),
                    });
                };
                floom_testkit::Selector::Role(role)
            }
        };
        if self.app.find(candidate.clone()).is_some() {
            Ok(candidate)
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
        let host = self.app.host();
        (0..host.node_count())
            .filter_map(|index| host.node_at(index))
            .enumerate()
            .filter_map(|(order, node)| {
                let hint = host.live_semantics_hint(node)?;
                let bounds = host.bounds(node);
                Some((order as u64, hint, bounds))
            })
            .collect()
    }

    fn tree(&self) -> UiTree {
        let nodes = self
            .live_nodes()
            .into_iter()
            .map(|(order, hint, bounds)| UiNodeInfo {
                id: order,
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
