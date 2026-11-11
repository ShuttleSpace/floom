// SPDX-License-Identifier: MIT
//! Headless → PNG + element-tree dump (P1-4 / WS-4.3 developer surface).
//!
//! One function, [`capture`], mounts a gallery screen in the real headless
//! shell and returns everything CI or an agent needs to look at a frame:
//!
//! * the software-rendered **PNG bytes** ([`Capture::png`]), produced by the
//!   dependency-free `floom_snapshot::encode_png` — no window, no GPU, no
//!   display;
//! * one [`SnapshotBundle`] whose every field is a type this crate already
//!   publishes (`UiTree`, `SemanticsEntryDto`, `ScreenDto`, `EvaluationReport`)
//!   or `floom_inspector::SceneLines`. A consumer deserializes it with the
//!   agent/evaluate vocabulary it already links — no new protocol.
//!
//! [`write_capture`] writes `<dir>/<stem>.png` and `<dir>/<stem>.json` and
//! returns both paths. `examples/snapshot.rs` is a thin CLI over this, so the
//! library surface and the command cannot drift apart.

use std::fs;
use std::path::{Path, PathBuf};

use floom_inspector::SceneLines;
use floom_snapshot::{PixelSnapshot, ReferencePixels, encode_png};
use floom_testkit::TestApp;
use serde::{Deserialize, Serialize};

use crate::agent::{
    AgentRequest, AgentSession, Command, PROTOCOL_VERSION, Payload, ScreenDto, SemanticsEntryDto,
    UiTree,
};
use crate::evaluate::{EvaluationReport, evaluate_screen};
use crate::screens::{Screen, screen_size};
use crate::tokens::ScreenTokens;

/// The serializable half of a capture: geometry, semantics, scene, pixels and
/// the evaluation report — one JSON object, all published types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotBundle {
    /// `Screen`'s Debug name, e.g. `"Settings"`.
    pub screen: String,
    pub width: u32,
    pub height: u32,
    /// FNV-1a of the RGBA buffer (the same fingerprint `evaluate` records).
    pub pixel_hash: String,
    /// Filename of the PNG written next to this JSON.
    pub png: String,
    pub tree: UiTree,
    pub semantics: Vec<SemanticsEntryDto>,
    pub scene: SceneLines,
    pub screenshot: ScreenDto,
    pub evaluation: EvaluationReport,
}

/// A headless frame: the PNG bytes plus the JSON-serializable bundle.
#[derive(Debug, Clone)]
pub struct Capture {
    pub stem: String,
    pub png: Vec<u8>,
    pub bundle: SnapshotBundle,
}

/// Lowercase `Screen` Debug name, used for both output filenames.
pub fn stem(screen: Screen) -> String {
    format!("{screen:?}").to_lowercase()
}

/// Render `screen` headlessly and collect the full inspection bundle.
///
/// The pixel layer uses the same `PixelSnapshot::of(... Some(resources) ...)`
/// call `evaluate_screen` uses, so text nodes are rasterised rather than
/// skipped; the agent DTOs come from a real `AgentSession` over the same
/// mounted `TestApp`.
pub fn capture(screen: Screen, tokens: &ScreenTokens) -> Result<Capture, String> {
    let size = screen_size();
    let mut app = TestApp::new(size, screen.build(tokens));

    // Pixels: borrow the host immutably, then release it before the mutable
    // borrow the AgentSession needs.
    let (png, width, height, pixel_hash) = {
        let resources = app.host().resources();
        let pixels = PixelSnapshot::of(app.host().scene(), Some(resources), size, 1.0);
        let bytes = encode_png(&pixels.rgba, pixels.width, pixels.height)
            .map_err(|e| format!("encode PNG for {screen:?}: {e}"))?;
        (bytes, pixels.width, pixels.height, pixels.hash)
    };

    let (tree, semantics, scene, screenshot) = {
        let mut session = AgentSession::new(&mut app);
        let tree = match ask(&mut session, 1, Command::Tree)? {
            Payload::Tree(t) => t,
            other => return Err(unexpected("Tree", &other)),
        };
        let semantics = match ask(&mut session, 2, Command::Semantics)? {
            Payload::Semantics(s) => s,
            other => return Err(unexpected("Semantics", &other)),
        };
        let scene = match ask(&mut session, 3, Command::Scene)? {
            Payload::Scene(s) => s,
            other => return Err(unexpected("Scene", &other)),
        };
        let screenshot = match ask(&mut session, 4, Command::Screenshot)? {
            Payload::Screenshot(s) => s,
            other => return Err(unexpected("Screenshot", &other)),
        };
        (tree, semantics, scene, screenshot)
    };

    let file_stem = stem(screen);
    let png_name = format!("{file_stem}.png");
    let bundle = SnapshotBundle {
        screen: format!("{screen:?}"),
        width,
        height,
        pixel_hash,
        png: png_name,
        tree,
        semantics,
        scene,
        screenshot,
        evaluation: evaluate_screen(screen, tokens),
    };

    Ok(Capture {
        stem: file_stem,
        png,
        bundle,
    })
}

/// Write `<dir>/<stem>.png` and `<dir>/<stem>.json`, creating `dir` if needed.
/// Returns `(png_path, json_path)`.
pub fn write_capture(dir: &Path, shot: &Capture) -> Result<(PathBuf, PathBuf), String> {
    fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let png_path = dir.join(format!("{}.png", shot.stem));
    let json_path = dir.join(format!("{}.json", shot.stem));

    fs::write(&png_path, &shot.png).map_err(|e| format!("{}: {e}", png_path.display()))?;
    let json =
        serde_json::to_string_pretty(&shot.bundle).map_err(|e| format!("serialize bundle: {e}"))?;
    fs::write(&json_path, json).map_err(|e| format!("{}: {e}", json_path.display()))?;
    Ok((png_path, json_path))
}

fn ask(session: &mut AgentSession<'_>, id: u64, command: Command) -> Result<Payload, String> {
    let response = session.handle(AgentRequest {
        version: PROTOCOL_VERSION,
        id,
        token: None,
        command,
    });
    response
        .result
        .map_err(|e| format!("agent command {id} failed: {e:?}"))
}

fn unexpected(wanted: &str, got: &Payload) -> String {
    format!("agent returned {got:?} where {wanted} was expected")
}
