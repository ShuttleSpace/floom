//! One window, the software renderer, and a report of what it got.
//!
//! The other window examples need a GPU. This one needs only a window server,
//! which is what makes it possible to run "the stack opens a window and draws"
//! on a machine with no GPU — a CI runner under Xvfb, for instance. See
//! `scripts/linux-smoke.sh`.
//!
//! ```sh
//! xvfb-run -a cargo run -p floom-window-software
//! ```
//!
//! `FLOOM_SOFTWARE_EXIT_FRAMES` (default 8) bounds the run: the app closes the
//! loop itself, so the script never has to kill a window that is waiting for a
//! human.
//!
//! `--agent <port>` starts the agent protocol on loopback, and then the binary
//! proves it: a probe thread asks for the tree, taps a label, and the app exits
//! once both answers are in. That is the whole "an agent can read and drive a
//! real window" claim, checked rather than asserted.

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use floom_accuracy::agent::{
    AgentClient, AgentSession, Command, Payload, Pending, Selector, WindowApp, bind, spawn_bridge,
};
use floom_app::AppRunner;
use floom_core::{Axis, LayoutRequest};
use floom_platform::{Platform, Size, WindowConfig, WindowId};
use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floom_renderer_software::SoftwarePresenter;
use floom_widgets::{Button, Stack};

struct SoftwareApp {
    runner: AppRunner<SoftwarePresenter>,
    window: Option<WindowId>,
    frames: u32,
    exit_after: u32,
    /// The loop only hands out frames while the tree is unsettled, so a frame
    /// budget alone can wait forever on a still tree. This is the other half.
    deadline: Instant,
    /// Requests waiting for this thread, when `--agent` was given.
    agent: Option<Receiver<Pending>>,
    /// Set by the probe once it has its answers.
    probed: Arc<AtomicBool>,
}

impl WinitApp for SoftwareApp {
    fn window_created(&mut self, platform: &mut WinitPlatform, window: WindowId) {
        self.window = Some(window);
        // The evidence line: a window exists and the OS gave it a surface of a
        // real size. The script asserts on it, so it cannot pass silently.
        if let Some(created) = platform.window(window) {
            let size = created.size();
            eprintln!(
                "floom-software: window surface {}x{} at scale {}",
                size.width,
                size.height,
                created.display_info().scale_factor.get()
            );
        }
    }

    fn events(&mut self, platform: &mut WinitPlatform) {
        let stats = self.runner.pump(platform);
        self.serve_agent();
        if self.probed.load(Ordering::SeqCst) {
            eprintln!("floom-software: the agent finished its probe; leaving");
            std::process::exit(0);
        }
        let _ = platform;
        let _ = platform;
        // The presenter's own pixels are the only proof that a frame was drawn
        // rather than a window opened over an empty buffer.
        let presenter = self.runner.presenter();
        if presenter.width() == 0 || presenter.height() == 0 {
            return;
        }
        let inked = presenter
            .pixels()
            .chunks_exact(4)
            .filter(|p| p[3] > 0)
            .count();
        self.frames += 1;
        // The extra numbers separate "no events reached the tree" from "the tree
        // painted nothing" from "the presenter cleared the buffer" — a frame that
        // arrives empty has to say which.
        eprintln!(
            "floom-software: frame {} drew {}x{} with {inked} inked pixels \
             (events={} scene_ops={} host={:?})",
            self.frames,
            presenter.width(),
            presenter.height(),
            stats.events,
            self.runner.host().scene().len(),
            self.runner.host().size(),
        );
        if inked > 0 && self.frames >= 2 && self.agent.is_none() {
            // `run` has no "close the loop" hook for the app, and this binary is
            // driven by a script rather than a human: leaving once a frame has
            // ink is the whole contract.
            eprintln!(
                "floom-software: {} frames is the report; leaving",
                self.frames
            );
            std::process::exit(0);
        }
        if self.frames >= self.exit_after && self.agent.is_none() {
            eprintln!(
                "floom-software: {} frames and no ink; leaving without a drawing",
                self.frames
            );
            std::process::exit(0);
        }
    }

    fn about_to_wait(&mut self, _platform: &mut WinitPlatform) {
        // Requests are answered here too, not only in `events`: an idle tree
        // stops handing out frames, and an agent asking a question must not
        // have to make the UI move first.
        self.serve_agent();
        if self.probed.load(Ordering::SeqCst) {
            eprintln!("floom-software: the agent finished its probe; leaving");
            std::process::exit(0);
        }
        if self.deadline.elapsed() > Duration::from_secs(10) {
            eprintln!(
                "floom-software: 10s and {} frames later, the loop is still waiting",
                self.frames
            );
            std::process::exit(0);
        }
    }
}

impl SoftwareApp {
    /// Answer every request that is waiting. Each one is handled against the
    /// live host, then a frame runs so a tap's effect is in the next report.
    fn serve_agent(&mut self) {
        loop {
            let Some(pending) = self.agent.as_ref().and_then(|rx| rx.try_recv().ok()) else {
                return;
            };
            let request = pending.request().clone();
            let size = self.runner.host().size();
            let response = {
                let mut app = WindowApp::new(self.runner.host_mut(), size);
                AgentSession::new(&mut app).handle(request)
            };
            pending.answer(response);
            self.runner.host_mut().frame();
        }
    }
}

/// Ask the app, from a thread of its own, the two questions an agent would.
///
/// A thread because the answer needs this thread's frame loop: calling the
/// client inline would block the only thing that can answer it.
fn spawn_probe(addr: std::net::SocketAddr, token: Option<String>, done: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(1500));
        let client = AgentClient::connect(addr, token);
        match client.call(Command::Tree) {
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
                    None => eprintln!("agent: the tree has no Increment"),
                }
            }
            Ok(other) => eprintln!("agent: Tree answered with {other:?}"),
            Err(e) => eprintln!("agent: Tree failed: {e:?}"),
        }
        match client.call(Command::Tap {
            selector: Selector::Label("Increment".to_string()),
        }) {
            Ok(Payload::Done { repainted }) => {
                eprintln!("agent: tap Increment repainted={repainted}")
            }
            Ok(other) => eprintln!("agent: Tap answered with {other:?}"),
            Err(e) => eprintln!("agent: Tap failed: {e:?}"),
        }
        done.store(true, Ordering::SeqCst);
    });
}

fn main() {
    let exit_after = env::var("FLOOM_SOFTWARE_EXIT_FRAMES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8u32);

    let mut host = floom_app::WidgetHost::new();
    host.mount_root(
        floom_layout::LayoutStyle::default(),
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(Button::new("Increment"))
            }),
    );

    let mut agent = None;
    let probed = Arc::new(AtomicBool::new(false));
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--agent" {
            let port: u16 = args.next().and_then(|p| p.parse().ok()).unwrap_or_else(|| {
                eprintln!("--agent needs a port");
                std::process::exit(2);
            });
            let listener = bind("127.0.0.1", port).unwrap_or_else(|e| {
                eprintln!("agent listener refused: {e}");
                std::process::exit(2);
            });
            let addr = listener
                .local_addr()
                .expect("a bound listener has an address");
            agent = Some(spawn_bridge(listener).expect("the bridge thread starts"));
            eprintln!("floom-software: agent listening on {addr}");
            let done = probed.clone();
            spawn_probe(addr, None, done);
        }
    }

    let app = SoftwareApp {
        runner: AppRunner::new(host, SoftwarePresenter::new()),
        window: None,
        frames: 0,
        exit_after,
        deadline: Instant::now(),
        agent,
        probed,
    };

    let windows = vec![WindowConfig {
        title: "floom — software window".into(),
        size: Size::new(320.0, 240.0),
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
    eprintln!("floom-software: the loop returned after {exit_after} frames");
}
