//! Golden example: typed navigation (Phase 30.1).
//!
//! The navigation layer is deliberately widget-free, so every interesting part
//! of it runs headless. Four pieces are shown:
//!
//! 1. **Typed routes** — an app route enum implements [`Route`]; `encode`
//!    produces the canonical path and `decode` is its exact inverse, so a deep
//!    link the app cannot express fails to decode instead of arriving as a
//!    string the UI has to second-guess.
//! 2. **History** — [`Router`] owns "where am I": push, replace, pop and a
//!    bounded back stack.
//! 3. **Transitions** — [`Navigator`] models pages with stable keys, including
//!    the exit window a pop animation needs.
//! 4. **Containers + deep links** — `StackNav`/`TabNav`/`SplitNav` state, and
//!    the `url` boundary that turns `app://host/profile/42?tab=posts` into a
//!    route.
//!
//! Run with `cargo run -p floom-navigation-demo`.

use floom_navigation::containers::{SplitNav, StackNav, TabNav};
use floom_navigation::url;
use floom_navigation::{Navigator, Route, Router};

/// The app's routes: one enum, one canonical spelling each.
#[derive(Debug, Clone, PartialEq)]
enum AppRoute {
    Home,
    Profile { id: u32 },
    Settings,
}

impl Route for AppRoute {
    fn encode(&self) -> String {
        match self {
            AppRoute::Home => "/home".to_string(),
            AppRoute::Profile { id } => format!("/profile/{id}"),
            AppRoute::Settings => "/settings".to_string(),
        }
    }

    fn decode(path: &str) -> Option<Self> {
        match path {
            "/home" => Some(AppRoute::Home),
            "/settings" => Some(AppRoute::Settings),
            other => {
                let id_part = other.strip_prefix("/profile/")?;
                id_part
                    .parse::<u32>()
                    .ok()
                    .map(|id| AppRoute::Profile { id })
            }
        }
    }
}

fn main() {
    // (1) encode/decode round-trip, and a route the app cannot express.
    for route in [
        AppRoute::Home,
        AppRoute::Profile { id: 42 },
        AppRoute::Settings,
    ] {
        let path = route.encode();
        let decoded = AppRoute::decode(&path).expect("every encoded route decodes");
        assert_eq!(decoded, route);
        println!("{route:?} <-> {path}");
    }
    assert!(
        AppRoute::decode("/admin").is_none(),
        "an unexpressible path must fail to decode rather than default silently"
    );

    // (2) History with a bounded back stack: `with_limit(2)` keeps at most two
    // entries *including* the current route, so the oldest drops off.
    let mut router = Router::new(AppRoute::Home).with_limit(2);
    router.push(AppRoute::Profile { id: 7 });
    router.push(AppRoute::Settings);
    println!(
        "\ncurrent={:?} depth={} can_pop={} history={:?}",
        router.current(),
        router.depth(),
        router.can_pop(),
        router.history().map(|r| r.encode()).collect::<Vec<_>>()
    );
    assert_eq!(router.current(), &AppRoute::Settings);
    assert_eq!(
        router.history().count(),
        2,
        "the back stack is capped: Home has been dropped"
    );
    assert_eq!(router.depth(), 1, "depth excludes the current route");

    let popped = router.pop();
    println!("popped={popped:?} -> current={:?}", router.current());
    assert_eq!(popped, Some(AppRoute::Settings));
    assert_eq!(router.current(), &AppRoute::Profile { id: 7 });

    // replace() swaps the top without growing history (a redirect).
    router.replace(AppRoute::Profile { id: 9 });
    let history: Vec<String> = router.history().map(|r| r.encode()).collect();
    println!("after replace: {history:?}");
    assert_eq!(history, vec!["/profile/9".to_string()]);
    assert_eq!(router.pop_to_root(), 0, "already at the root after the cap");

    // (3) Page stack with stable keys and an exit window.
    let mut nav = Navigator::new("/home");
    let profile_key = nav.push("/profile/42");
    let settings_key = nav.push("/settings");
    println!(
        "\npages={:?} (profile page key={profile_key}, settings key={settings_key})",
        nav.pages()
            .iter()
            .map(|p| (p.key, p.path.as_str()))
            .collect::<Vec<_>>()
    );
    assert_eq!(nav.top().path, "/settings");

    assert!(nav.pop(), "a non-root page pops");
    println!(
        "after pop: top={:?} exiting={:?}",
        nav.top().path,
        nav.exiting().map(|p| p.path.as_str())
    );
    assert_eq!(
        nav.exiting().map(|p| p.key),
        Some(settings_key),
        "the popped page stays alive for its exit transition, keyed stably"
    );
    nav.finish_exit();
    assert!(nav.exiting().is_none());

    assert!(nav.pop(), "the profile page pops too");
    assert_eq!(nav.pages().len(), 1, "only the root page remains");
    assert!(!nav.pop(), "the root page cannot pop away");
    println!(
        "root-bound: pages={} popped_to_root={}",
        nav.pages().len(),
        nav.pop_to_root()
    );
    assert_eq!(
        nav.pages().len(),
        1,
        "only the root page survives pop_to_root"
    );

    // (4) Containers: adaptive two-pane, tabs, and a clamped splitter.
    let mut stack = StackNav::default();
    stack.set_expanded(false);
    println!(
        "\nstack collapsed -> shows_secondary={}",
        stack.shows_secondary()
    );
    assert!(!stack.shows_secondary());
    stack.open_secondary();
    assert!(
        stack.shows_secondary(),
        "opening the detail pane reveals it"
    );
    println!(
        "stack detail opened -> shows_secondary={}",
        stack.shows_secondary()
    );

    let mut tabs = TabNav::new(3, 0);
    tabs.select(2);
    println!("tabs selected={} count={}", tabs.selected(), tabs.count());
    assert!(tabs.is_selected(2) && !tabs.is_selected(0));

    let mut split = SplitNav::default();
    split.set_ratio(0.02, 0.2);
    println!(
        "split clamped to min -> ratio={:.2}",
        split.effective_ratio()
    );
    assert!(
        (split.effective_ratio() - 0.2).abs() < 1e-6,
        "a divider drag that would squeeze a pane below its minimum is clamped"
    );
    split.toggle_leading();
    assert_eq!(split.effective_ratio(), 0.0);

    // The stringly-typed edge: deep links decode through the typed contract.
    let deep = url::parse("app://example.com/profile/42?tab=posts").expect("parses");
    let route = AppRoute::decode(&deep.path).expect("known path");
    println!(
        "\ndeep link -> path={} route={route:?} tab={:?}",
        deep.path,
        deep.query("tab")
    );
    assert_eq!(route, AppRoute::Profile { id: 42 });
    assert_eq!(deep.query("tab"), Some("posts"));

    println!("\nnavigation golden example: OK");
}
