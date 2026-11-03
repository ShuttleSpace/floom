//! Application routes and navigation intents.
//!
//! Self-contained: no dependency on any other showcase module.

use floom_navigation::Route;

/// Every navigable destination in the showcase.
///
/// The `id` order is the stack order: `Router::push`/`pop` moves through it,
/// and `path` is the URL segment a route encodes to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppRoute {
    Home,
    Components,
    Signals,
    Kits,
    Editor,
    Devtools,
}

impl AppRoute {
    pub const ALL: [Self; 6] = [
        Self::Home,
        Self::Components,
        Self::Signals,
        Self::Kits,
        Self::Editor,
        Self::Devtools,
    ];

    pub fn path(self) -> &'static str {
        match self {
            Self::Home => "/",
            Self::Components => "/components",
            Self::Signals => "/signals",
            Self::Kits => "/kits",
            Self::Editor => "/editor",
            Self::Devtools => "/devtools",
        }
    }

    pub fn nav_label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Components => "Components",
            Self::Signals => "Signals",
            Self::Kits => "Kits",
            Self::Editor => "Editor",
            Self::Devtools => "Devtools",
        }
    }

    pub fn id(self) -> u64 {
        match self {
            Self::Home => 0,
            Self::Components => 1,
            Self::Signals => 2,
            Self::Kits => 3,
            Self::Editor => 4,
            Self::Devtools => 5,
        }
    }
}

impl Route for AppRoute {
    fn encode(&self) -> String {
        self.path().to_string()
    }

    fn decode(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|route| route.path() == path)
    }
}

/// A navigation request the shell's `update` turns into a router push/pop.
#[derive(Clone, Copy)]
pub enum NavIntent {
    Go(AppRoute),
    Back,
}
