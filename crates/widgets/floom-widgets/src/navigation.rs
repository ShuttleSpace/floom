// SPDX-License-Identifier: MIT
//! Typed routing and navigation for Floom 2.0.
//!
//! Framework sits *below* widgets in the architecture, so this crate is
//! deliberately widget-free: it models navigation as plain data and state
//! machines, and the widget layer renders whatever state it exposes.
//!
//! Four pieces:
//!
//! * [`Route`] — the typed route contract (`encode`/`decode`), so an app's
//!   route enum is the single source of truth for URLs.
//! * [`Router`] / [`Navigator`] — history + a page stack with transitions.
//! * [`navigation_url`] — the deep-link boundary: `scheme://host/path?query` parsing,
//!   deliberately the only stringly-typed edge in the crate.
//! * [`navigation_containers`] — `StackNav`/`TabNav`/`SplitNav` layout state machines.

pub mod navigation_containers;
pub mod navigation_url;

/// The typed route contract.
///
/// Implement this on the app's route enum; `encode` produces the canonical
/// path segment (starting with `/`) and `decode` is its exact inverse for
/// recognised inputs. Deep links decode through this pair, so a route the app
/// cannot express simply fails to decode instead of arriving as a string the
/// UI has to second-guess.
pub trait Route: Clone + PartialEq + std::fmt::Debug {
    /// Canonical path for this route, e.g. `/settings/profile`.
    fn encode(&self) -> String;

    /// The inverse of [`encode`](Route::encode); `None` for unknown paths.
    fn decode(path: &str) -> Option<Self>
    where
        Self: Sized;
}

/// Navigation history over typed routes.
///
/// `Router` owns the *where am I* question: a current route plus a back stack.
/// It never touches widgets — the app observes [`Router::current`] and
/// re-renders.
#[derive(Debug, Clone)]
pub struct Router<R: Route> {
    stack: Vec<R>,
    /// How deep the back stack may grow; `0` = unlimited.
    limit: usize,
}

impl<R: Route> Router<R> {
    /// A router showing `initial` with an empty back stack.
    pub fn new(initial: R) -> Self {
        Self {
            stack: vec![initial],
            limit: 0,
        }
    }

    /// Cap the history depth (oldest entries are dropped first).
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        if limit > 0 && self.stack.len() > limit {
            let excess = self.stack.len() - limit;
            self.stack.drain(0..excess);
        }
        self
    }

    /// The route currently on screen.
    pub fn current(&self) -> &R {
        self.stack
            .last()
            .expect("router always holds at least one route")
    }

    /// Whether [`pop`](Router::pop) would leave the initial route untouched.
    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }

    /// Push a new route onto the stack.
    pub fn push(&mut self, route: R) {
        self.stack.push(route);
        if self.limit > 0 {
            while self.stack.len() > self.limit {
                self.stack.remove(0);
            }
        }
    }

    /// Replace the current route without touching the back stack (redirects).
    pub fn replace(&mut self, route: R) {
        *self
            .stack
            .last_mut()
            .expect("router always holds at least one route") = route;
    }

    /// Pop the current route; returns it, or `None` when already at the root.
    pub fn pop(&mut self) -> Option<R> {
        if self.can_pop() {
            self.stack.pop()
        } else {
            None
        }
    }

    /// Pop to the very first route (Android back-to-home).
    ///
    /// Returns the number of routes removed.
    pub fn pop_to_root(&mut self) -> usize {
        let removed = self.stack.len().saturating_sub(1);
        self.stack.truncate(1);
        removed
    }

    /// Replace the whole history with a single route (sign-out etc.).
    pub fn reset(&mut self, route: R) {
        self.stack.clear();
        self.stack.push(route);
    }

    /// The back stack, root first, current last. Observation only.
    pub fn history(&self) -> impl Iterator<Item = &R> {
        self.stack.iter()
    }

    /// Back-stack depth excluding the current route.
    pub fn depth(&self) -> usize {
        self.stack.len() - 1
    }
}

/// One page on the [`Navigator`] stack, with a stable identity so the widget
/// layer can keep per-page state across pushes and pops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// Stable identity (survives re-renders; used as a widget key).
    pub key: u64,
    /// The route this page renders.
    pub path: String,
}

/// A page stack with enter/exit transition state.
///
/// [`Router`] decides *what* the route is; `Navigator` models *how pages come
/// and go* — including a brief exit window for pop animations, which is what
/// makes keyed page widgets keep their state through a transition.
#[derive(Debug, Clone)]
pub struct Navigator {
    pages: Vec<Page>,
    next_key: u64,
    /// The page most recently popped, kept alive for its exit transition.
    exiting: Option<Page>,
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new("root")
    }
}

impl Navigator {
    /// A navigator whose root page renders `root_path`.
    pub fn new(root_path: impl Into<String>) -> Self {
        Self {
            pages: vec![Page {
                key: 0,
                path: root_path.into(),
            }],
            next_key: 1,
            exiting: None,
        }
    }

    /// Pages currently alive (top = last), plus [`Navigator::exiting`].
    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    /// A popped page still playing its exit transition, if any.
    pub fn exiting(&self) -> Option<&Page> {
        self.exiting.as_ref()
    }

    /// The topmost page.
    pub fn top(&self) -> &Page {
        self.pages.last().expect("navigator always keeps the root")
    }

    /// Push a page; returns its stable key.
    pub fn push(&mut self, path: impl Into<String>) -> u64 {
        let key = self.next_key;
        self.next_key += 1;
        self.pages.push(Page {
            key,
            path: path.into(),
        });
        // A new push cancels any pending exit: the user navigated onward.
        self.exiting = None;
        key
    }

    /// Pop the top page. Returns `false` when only the root remains.
    ///
    /// The popped page moves to `exiting`, where it stays until the next
    /// [`finish_exit`](Navigator::finish_exit) (the widget layer calls that
    /// when its animation ends) or the next navigation event.
    pub fn pop(&mut self) -> bool {
        if self.pages.len() <= 1 {
            return false;
        }
        let page = self.pages.pop().expect("checked non-empty");
        self.exiting = Some(page);
        true
    }

    /// Drop the exiting page; safe to call any time (no-op when none).
    pub fn finish_exit(&mut self) {
        self.exiting = None;
    }

    /// Pop everything above the root.
    pub fn pop_to_root(&mut self) -> usize {
        let removed = self.pages.len().saturating_sub(1);
        if removed > 0 {
            let last = self.pages.pop().expect("checked non-empty");
            self.pages.truncate(1);
            self.exiting = Some(last);
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A typical app route enum exercising the whole contract.
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum TestRoute {
        Home,
        Profile(u32),
        Settings,
    }

    impl Route for TestRoute {
        fn encode(&self) -> String {
            match self {
                TestRoute::Home => "/".to_string(),
                TestRoute::Profile(id) => format!("/profile/{id}"),
                TestRoute::Settings => "/settings".to_string(),
            }
        }

        fn decode(path: &str) -> Option<Self> {
            match path {
                "/" => Some(TestRoute::Home),
                "/settings" => Some(TestRoute::Settings),
                p if p.starts_with("/profile/") => {
                    let id = p.trim_start_matches("/profile/").parse().ok()?;
                    Some(TestRoute::Profile(id))
                }
                _ => None,
            }
        }
    }

    #[test]
    fn router_pushes_pops_and_replaces() {
        let mut router = Router::new(TestRoute::Home);
        assert!(!router.can_pop());

        router.push(TestRoute::Profile(7));
        assert_eq!(router.current(), &TestRoute::Profile(7));
        assert_eq!(router.depth(), 1);

        router.replace(TestRoute::Settings);
        assert_eq!(router.current(), &TestRoute::Settings);
        assert_eq!(router.depth(), 1, "replace does not grow the stack");

        assert_eq!(router.pop(), Some(TestRoute::Settings));
        assert_eq!(router.current(), &TestRoute::Home);
        assert_eq!(router.pop(), None, "the root is not popped");
    }

    #[test]
    fn router_limit_drops_oldest_entries() {
        let mut router = Router::new(TestRoute::Home).with_limit(3);
        for id in 1..=5 {
            router.push(TestRoute::Profile(id));
        }
        // Home was evicted; the oldest survivor is Profile(3).
        let history: Vec<_> = router.history().cloned().collect();
        assert_eq!(
            history,
            vec![
                TestRoute::Profile(3),
                TestRoute::Profile(4),
                TestRoute::Profile(5)
            ]
        );
        assert_eq!(router.depth(), 2);
    }

    #[test]
    fn router_reset_and_pop_to_root() {
        let mut router = Router::new(TestRoute::Home);
        router.push(TestRoute::Profile(1));
        router.push(TestRoute::Settings);
        assert_eq!(router.pop_to_root(), 2);
        assert_eq!(router.current(), &TestRoute::Home);

        router.push(TestRoute::Settings);
        router.reset(TestRoute::Profile(9));
        assert_eq!(router.current(), &TestRoute::Profile(9));
        assert!(!router.can_pop(), "reset clears the back stack");
    }

    #[test]
    fn navigator_keys_pages_and_exit_window() {
        let mut nav = Navigator::new("/");
        let first = nav.push("/list");
        let second = nav.push("/detail/2");
        assert_ne!(first, second, "page keys are unique");
        assert_eq!(nav.top().path, "/detail/2");
        assert_eq!(nav.pages().len(), 3);

        // Pop keeps the page for its exit transition.
        assert!(nav.pop());
        assert_eq!(nav.top().path, "/list");
        assert_eq!(nav.exiting().map(|p| p.key), Some(second));

        // The next push cancels the pending exit.
        let third = nav.push("/detail/3");
        assert!(nav.exiting().is_none());

        // Popping to root exits only the top page; keys never repeat.
        assert_eq!(nav.pop_to_root(), 2);
        assert_eq!(nav.exiting().map(|p| p.key), Some(third));
        nav.finish_exit();
        assert_eq!(nav.pages().len(), 1);
        assert!(!nav.pop(), "the root does not pop");
    }

    #[test]
    fn deep_link_round_trip_through_typed_route() {
        let route = TestRoute::Profile(42);
        let parsed = navigation_url::parse("floom://app/profile/42?ref=push").expect("valid url");
        assert_eq!(parsed.path, "/profile/42");
        assert_eq!(parsed.query("ref"), Some("push"));

        let decoded = TestRoute::decode(&parsed.path).expect("recognised route");
        assert_eq!(decoded, route);
        assert_eq!(decoded.encode(), "/profile/42");
    }
}
