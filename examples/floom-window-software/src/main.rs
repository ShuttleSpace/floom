// SPDX-License-Identifier: MIT
//! One window, the software renderer, and a report of what it got.
//!
//! The other window examples need a GPU. This one needs only a window server,
//! which is what makes it possible to run "the stack opens a window and draws"
//! on a machine with no GPU — a CI runner under Xvfb, for instance. See
//! `scripts/desktop-smoke.sh`.
//!
//! ```sh
//! xvfb-run -a cargo run -p floom-window-software
//! ```
//!
//! It goes through the same public door an app does —
//! [`floom::run_desktop_software_observed`] — and only adds a watcher, so what
//! CI exercises is the entry point users call, not a copy of it.
//!
//! `FLOOM_SOFTWARE_EXIT_FRAMES` (default 8) bounds the run: the watcher closes
//! the process itself, so the script never has to kill a window that is waiting
//! for a human.
//!
//! `--agent-selftest` asks this window the two questions an agent would -- read
//! the tree, tap a label -- over the in-memory transport, then leaves. In the
//! binary rather than in a test because what is being checked is a *window that
//! exists*: the same session code, the same `WidgetHost`, a real surface.
//! Same-thread and port-free on purpose: a probe thread racing a frame loop was
//! the first shape of this, and it failed on a runner for reasons that had
//! nothing to do with the protocol.
//!
//! `--agent <port>` starts the loopback TCP transport for a human or a tool;
//! nothing in CI asserts on it, because the transport has its own test.

use std::env;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use floom_accuracy::agent::{
    AgentSession, Command, Payload, Pending, Selector, WindowApp, bind, local_channel, spawn_bridge,
};
use floom_app::AppRunner;
use floom_core::{Axis, LayoutRequest};
use floom_renderer_software::SoftwarePresenter;
use floom_types::Size;
use floom_widgets::{Button, Stack};

/// Everything the run reports and the two ways it ends.
struct Watch {
    frames: u32,
    exit_after: u32,
    #[allow(dead_code)]
    deadline: Instant,
    agent: Option<Receiver<Pending>>,
    selftest: bool,
}

impl Watch {
    fn frame(&mut self, runner: &mut AppRunner<SoftwarePresenter>) {
        self.frames += 1;
        let presenter = runner.presenter();
        let (width, height) = (presenter.width(), presenter.height());
        if self.frames == 1 {
            // The evidence line: a window exists and has a real size. The script
            // asserts on it, so it cannot pass silently.
            let host = runner.host().size();
            eprintln!(
                "floom-software: window surface {}x{} at scale {}",
                host.width, host.height, 1.0
            );
        }
        let inked = presenter
            .pixels()
            .chunks_exact(4)
            .filter(|pixel| pixel[3] > 0)
            .count();
        eprintln!(
            "floom-software: frame {} drew {width}x{height} with {inked} inked pixels",
            self.frames
        );

        self.serve_agent(runner);

        if self.selftest && self.frames >= 2 {
            // A frame publishes semantics; how many a host needs before the tree
            // is laid out is not something to guess at, so paint until it says
            // something and report the count.
            let mut tries = 0;
            while runner.host().semantics().iter().count() == 0 && tries < 5 {
                runner.host_mut().frame();
                tries += 1;
            }
            eprintln!(
                "agent: {} semantic nodes after {} extra frames",
                runner.host().semantics().iter().count(),
                tries
            );
            let answered = agent_selftest(runner.host_mut());
            eprintln!(
                "floom-software: agent self-test {}",
                if answered { "passed" } else { "failed" }
            );
            std::process::exit(if answered { 0 } else { 1 });
        }

        if inked > 0 && self.frames >= 2 && self.agent.is_none() && !self.selftest {
            eprintln!("floom-software: the window drew; leaving");
            std::process::exit(0);
        }
        if self.frames >= self.exit_after && self.agent.is_none() && !self.selftest {
            eprintln!(
                "floom-software: {} frames is the report; leaving",
                self.frames
            );
            std::process::exit(0);
        }
        if self.deadline.elapsed() > Duration::from_secs(10) && self.agent.is_none() {
            eprintln!(
                "floom-software: {} frames in 10s, the loop is still waiting",
                self.frames
            );
            std::process::exit(1);
        }
    }

    fn serve_agent(&mut self, runner: &mut AppRunner<SoftwarePresenter>) {
        let Some(requests) = self.agent.as_ref() else {
            return;
        };
        while let Ok(pending) = requests.try_recv() {
            let size = runner.host().size();
            let mut app = WindowApp::new(runner.host_mut(), size);
            let response = AgentSession::new(&mut app).handle(pending.request().clone());
            pending.answer(response);
        }
    }
}

/// Ask this window the two questions an agent would, over the in-memory
/// transport, and report what came back.
fn agent_selftest(host: &mut floom_app::WidgetHost) -> bool {
    let (client, server) = local_channel();
    let ask = |host: &mut floom_app::WidgetHost, command: Command| {
        client.send(command);
        while let Some(request) = server.next_request() {
            let size = host.size();
            let mut app = WindowApp::new(host, size);
            let response = AgentSession::new(&mut app).handle(request);
            server.respond(response);
        }
        client.receive()
    };

    let mut ok = true;
    match ask(host, Command::Tree) {
        Some(response) => match response.result {
            Ok(Payload::Tree(tree)) => {
                let node = tree
                    .nodes
                    .iter()
                    .find(|n| n.label.as_deref() == Some("Increment"));
                match node {
                    Some(node) => eprintln!(
                        "agent: tree has {} {} at {:.0},{:.0} {:.0}x{:.0}",
                        node.role,
                        node.label.clone().unwrap_or_default(),
                        node.x,
                        node.y,
                        node.width,
                        node.height
                    ),
                    None => {
                        eprintln!("agent: the tree has no Increment");
                        ok = false;
                    }
                }
            }
            Ok(other) => {
                eprintln!("agent: Tree answered with {other:?}");
                ok = false;
            }
            Err(e) => {
                eprintln!("agent: Tree failed: {e:?}");
                ok = false;
            }
        },
        None => {
            eprintln!("agent: Tree got no answer");
            ok = false;
        }
    }

    let tap = Command::Tap {
        selector: Selector::Label("Increment".to_string()),
    };
    match ask(host, tap) {
        Some(response) => match response.result {
            Ok(Payload::Done { repainted }) => {
                eprintln!("agent: tap Increment repainted={repainted}");
                ok &= repainted;
            }
            Ok(other) => {
                eprintln!("agent: Tap answered with {other:?}");
                ok = false;
            }
            Err(e) => {
                eprintln!("agent: Tap failed: {e:?}");
                ok = false;
            }
        },
        None => {
            eprintln!("agent: Tap got no answer");
            ok = false;
        }
    }
    ok
}

fn main() {
    let exit_after = env::var("FLOOM_SOFTWARE_EXIT_FRAMES")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8u32);

    let mut agent = None;
    let mut selftest = false;
    let mut reloading = false;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--agent-selftest" {
            selftest = true;
        } else if arg == "--watch" {
            // The in-place path: rebuild and remount instead of restarting the
            // process. `FLOOM_WATCH_ROOT` exists because a binary's cwd is not
            // always its crate root.
            reloading = true;
        } else if arg == "--agent" {
            let port: u16 = args
                .next()
                .and_then(|port| port.parse().ok())
                .unwrap_or_else(|| {
                    eprintln!("--agent needs a port");
                    std::process::exit(2);
                });
            let listener = bind("127.0.0.1", port).unwrap_or_else(|error| {
                eprintln!("agent listener refused: {error}");
                std::process::exit(2);
            });
            let addr = listener
                .local_addr()
                .expect("a bound listener has an address");
            agent = Some(spawn_bridge(listener).expect("the bridge thread starts"));
            eprintln!("floom-software: agent listening on {addr}");
        }
    }

    let mut watch = Watch {
        frames: 0,
        exit_after,
        deadline: Instant::now(),
        agent,
        selftest,
    };

    if reloading {
        // Absolute on purpose: `Watch` sweeps on every poll, and a relative root
        // would resolve against whatever cwd the window happened to start in.
        let root = match env::var("FLOOM_WATCH_ROOT") {
            Ok(path) => path,
            Err(_) => format!("{}/src", env!("CARGO_MANIFEST_DIR")),
        };
        let mut watch = floom_hot_reload::watch::Watch::new(&root);
        // Baseline now, not on the first idle turn: the window is created before
        // the loop ever has a spare moment, and a file touched in between would
        // be invisible to a watcher that only looked afterwards.
        let known = watch.poll().len();
        eprintln!("floom-software: watching {root} for in-place reload");
        let _ = known;
        let result = floom::run_desktop_software_watched(
            "floom — software window (reloading)",
            Size::new(320.0, 240.0),
            move |host| {
                eprintln!("floom-software: rebuilding the tree");
                host.mount_root(
                    floom_layout::LayoutStyle::default(),
                    Stack::new(Axis::Vertical)
                        .gap(8.0)
                        .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                            Box::new(Button::new("Increment"))
                        }),
                );
            },
            move || !watch.poll().is_empty(),
        );
        if let Err(error) = result {
            eprintln!("window loop failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    let result = floom::run_desktop_software_observed(
        "floom — software window",
        Size::new(320.0, 240.0),
        |host| {
            host.mount_root(
                floom_layout::LayoutStyle::default(),
                Stack::new(Axis::Vertical)
                    .gap(8.0)
                    .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                        Box::new(Button::new("Increment"))
                    }),
            );
        },
        move |runner| watch.frame(runner),
    );

    if let Err(error) = result {
        eprintln!("window loop failed: {error}");
        std::process::exit(1);
    }
    eprintln!(
        "floom-software: the loop returned after {} frames",
        exit_after
    );
}
