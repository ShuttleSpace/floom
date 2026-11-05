// SPDX-License-Identifier: MIT
//! Task 27.2 — the automated render-and-compare loop.
//!
//! `evaluate_screen` mounts a screen in the real headless shell, settles it,
//! and measures the *rendered* result: geometry and radius out of the retained
//! scene, colours out of the painted rects and text runs, semantics out of the
//! a11y tree, and a pixel fingerprint out of the software renderer. Those
//! measurements become an *observed* design contract (Phase 26 vocabulary),
//! which `compare` diffs against the screen's reference fixture. Floors the
//! contract declares — WCAG contrast and the touch target — are checked
//! against the measurements directly, because a floor is a ">= " claim, not an
//! exact value.

use floom_snapshot::{PixelSnapshot, ReferencePixels};
use floom_testkit::{Selector, TestApp};
use floom_types::contrast_ratio;
use floom_types::{Point, Rect};
use floom_dsl::ui_ir::{
    ColorRole, ContractDiff, ContractVersion, DesignContract, Dimensions, Radius, Spacing,
    TypeStyle, compare,
};
use floom_widgets::Text;

use crate::screens::{Screen, screen_size};
use crate::tokens::ScreenTokens;

/// One a11y-floor violation, in report form.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FloorFailure {
    pub check: String,
    pub measured: String,
    pub floor: String,
}

/// What the harness measured off the rendered screen. Published so a test can
/// talk about quantities (line heights, measured radii) rather than only about
/// pass/fail.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Observation {
    pub gap: f64,
    pub padding_top: f64,
    pub padding_left: f64,
    pub radius: f32,
    pub title_font_size: f32,
    pub body_font_size: f32,
    pub title_line_height: f64,
    pub body_line_height: f64,
    pub primary_height: f64,
    pub surface: floom_dsl::ui_ir::Rgba,
    pub card: floom_dsl::ui_ir::Rgba,
    pub accent: floom_dsl::ui_ir::Rgba,
    pub on_accent: floom_dsl::ui_ir::Rgba,
    pub text: floom_dsl::ui_ir::Rgba,
}

/// The evaluation result: the contract diff, the floor failures, the missing
/// labels, and the pixel layer.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EvaluationReport {
    pub screen: String,
    pub diff: ContractDiff,
    pub floors: Vec<FloorFailure>,
    pub labels_missing: Vec<String>,
    pub pixel_hash: String,
    pub pixel_stable: bool,
    pub observation: Observation,
}

impl EvaluationReport {
    /// Every disagreement the harness found, of every kind.
    pub fn problems(&self) -> usize {
        self.diff.entries.len() + self.floors.len() + self.labels_missing.len()
    }

    /// Clean means zero problems *and* a render that reproduces itself.
    pub fn is_clean(&self) -> bool {
        self.problems() == 0 && self.pixel_stable
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("the report is JSON-serializable")
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Mounts `screen` built from `tokens`, and evaluates it against the
/// reference contract.
pub fn evaluate_screen(screen: Screen, tokens: &ScreenTokens) -> EvaluationReport {
    let app = TestApp::new(screen_size(), screen.build(tokens));
    measure(screen, &app)
}

/// The reference contract fixture for a screen.
pub fn reference_contract(screen: Screen) -> DesignContract {
    let json = match screen {
        Screen::Settings => include_str!("../fixtures/contracts/settings.json"),
        Screen::Dashboard => include_str!("../fixtures/contracts/dashboard.json"),
        Screen::Feed => include_str!("../fixtures/contracts/feed.json"),
        Screen::Editor => include_str!("../fixtures/contracts/editor.json"),
        Screen::MediaList => include_str!("../fixtures/contracts/media_list.json"),
        Screen::Form => include_str!("../fixtures/contracts/form.json"),
        Screen::Dialog => include_str!("../fixtures/contracts/dialog.json"),
        Screen::Navigation => include_str!("../fixtures/contracts/navigation.json"),
        Screen::Webview => include_str!("../fixtures/contracts/webview.json"),
    };
    let contract: DesignContract = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("{screen:?} contract fixture is invalid: {e}"));
    assert!(
        contract.version.accepts(ContractVersion::CURRENT),
        "{screen:?} fixture was written for contract version {:?}",
        contract.version
    );
    contract
}

fn measure(screen: Screen, app: &TestApp) -> EvaluationReport {
    let reference = reference_contract(screen);
    let window = screen_size();
    let near = |a: f64, b: f64| (a - b).abs() < 0.5;
    let covers_window = |r: &Rect| near(r.width(), window.width) && near(r.height(), window.height);
    let same_rect = |a: &Rect, b: &Rect| {
        near(a.x0, b.x0) && near(a.y0, b.y0) && near(a.x1, b.x1) && near(a.y1, b.y1)
    };

    let nodes = app.scene_nodes();

    // The surface is the first full-window painted rect (paint order: the root
    // paints it before any child can cover it). The card is the largest rect
    // that does *not* cover the window — a dialog scrim is full-window and so
    // excluded by the same rule, no special case needed.
    let surface_fill = nodes.iter().find_map(|node| match node {
        floom_scene::SceneNode::Rect { bounds, fill, .. }
            if covers_window(bounds) && fill.solid().is_some_and(|c| c.a == 255) =>
        {
            fill.solid()
        }
        _ => None,
    });
    let card = nodes
        .iter()
        .filter_map(|node| match node {
            floom_scene::SceneNode::Rect {
                bounds,
                fill,
                radius,
                ..
            } if !covers_window(bounds) && bounds.width() > 1.0 && bounds.height() > 1.0 => Some((
                bounds.width() * bounds.height(),
                *bounds,
                fill.solid(),
                *radius,
            )),
            _ => None,
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, bounds, fill, radius)| (bounds, fill, radius));

    let (card_bounds, card_fill, card_radius) = card.expect("every gallery screen paints a card");
    let surface = surface_fill.unwrap_or_default();

    // The action pair: adjacent siblings in the content column, so the gap
    // between them is the contract's gap.
    let primary_bounds = app.bounds(Selector::label(screen.primary_action()));
    let secondary_bounds = app.bounds(Selector::label(screen.secondary_action()));
    let gap = secondary_bounds.y0 - primary_bounds.y1;

    // The accent fill is the rect painted at the primary action's own bounds.
    let accent = nodes
        .iter()
        .find_map(|node| match node {
            floom_scene::SceneNode::Rect { bounds, fill, .. }
                if same_rect(bounds, &primary_bounds) =>
            {
                fill.solid()
            }
            _ => None,
        })
        .unwrap_or_default();

    // Title and body: the text node painted at the widget's own origin.
    let title_bounds = app.bounds(Selector::label(screen.title()));
    let body_bounds = app.bounds(Selector::label(screen.body()));
    let title_text = text_node_at(&nodes, title_bounds.origin());
    let body_text = text_node_at(&nodes, body_bounds.origin());
    let text_color = title_text.as_ref().map(|t| t.color).unwrap_or_default();
    let on_accent = nodes
        .iter()
        .find_map(|node| match node {
            floom_scene::SceneNode::Text {
                origin,
                size,
                color,
                ..
            } if origin.x >= primary_bounds.x0
                && origin.x + size.width <= primary_bounds.x1
                && origin.y >= primary_bounds.y0
                && origin.y + size.height <= primary_bounds.y1 =>
            {
                Some(*color)
            }
            _ => None,
        })
        .unwrap_or_default();

    // Resolved font sizes come from the mounted widgets; line heights from the
    // painted extents.
    let title_font_size = app
        .widget::<Text>(Selector::label(screen.title()))
        .font_size();
    let body_font_size = app
        .widget::<Text>(Selector::label(screen.body()))
        .font_size();

    let observation = Observation {
        gap,
        padding_top: title_bounds.y0 - card_bounds.y0,
        padding_left: title_bounds.x0 - card_bounds.x0,
        radius: card_radius,
        title_font_size,
        body_font_size,
        title_line_height: title_text.as_ref().map(|t| t.height).unwrap_or_default(),
        body_line_height: body_text.as_ref().map(|t| t.height).unwrap_or_default(),
        primary_height: primary_bounds.height(),
        surface: surface.into(),
        card: card_fill.unwrap_or_default().into(),
        accent: accent.into(),
        on_accent: on_accent.into(),
        text: text_color.into(),
    };

    // The observed contract: measured values in the reference's own shape.
    // Animation and the accessibility *floors* are copied, not measured —
    // motion is not statically observable, and a floor is enforced as a ">="
    // check below, not compared for equality.
    let observed = DesignContract {
        version: ContractVersion::CURRENT,
        name: reference.name.clone(),
        dimensions: Dimensions {
            width: window.width as f32,
            height: window.height as f32,
        },
        spacing: Spacing {
            gap: observation.gap as f32,
            padding: [
                observation.padding_top as f32,
                observation.padding_left as f32,
                observation.padding_top as f32,
                observation.padding_left as f32,
            ],
        },
        radius: Radius {
            corner: observation.radius,
        },
        typography: vec![
            TypeStyle {
                name: "body".into(),
                font_size: observation.body_font_size,
                line_height: None,
                weight: None,
            },
            TypeStyle {
                name: "title".into(),
                font_size: observation.title_font_size,
                line_height: None,
                weight: None,
            },
        ],
        colors: vec![
            ColorRole {
                name: "accent".into(),
                value: observation.accent,
            },
            ColorRole {
                name: "card".into(),
                value: observation.card,
            },
            ColorRole {
                name: "on_accent".into(),
                value: observation.on_accent,
            },
            ColorRole {
                name: "surface".into(),
                value: observation.surface,
            },
            ColorRole {
                name: "text".into(),
                value: observation.text,
            },
        ],
        states: Vec::new(),
        animation: reference.animation.clone(),
        accessibility: floom_dsl::ui_ir::Accessibility {
            role: reference.accessibility.role.clone(),
            label: reference.accessibility.label.clone(),
            min_contrast: reference.accessibility.min_contrast,
            min_touch_target: reference.accessibility.min_touch_target,
        },
    };
    let diff = compare(&reference, &observed);

    // Floors: measured values against the fixture's declared minima.
    let mut floors = Vec::new();
    let text_contrast = contrast_ratio(observation.text.into(), observation.card.into());
    if text_contrast < reference.accessibility.min_contrast {
        floors.push(FloorFailure {
            check: "contrast: text on card".into(),
            measured: format!("{text_contrast:.3}"),
            floor: format!("{:.3}", reference.accessibility.min_contrast),
        });
    }
    let accent_contrast = contrast_ratio(observation.on_accent.into(), observation.accent.into());
    if accent_contrast < reference.accessibility.min_contrast {
        floors.push(FloorFailure {
            check: "contrast: on_accent on accent".into(),
            measured: format!("{accent_contrast:.3}"),
            floor: format!("{:.3}", reference.accessibility.min_contrast),
        });
    }
    if (observation.primary_height as f32) < reference.accessibility.min_touch_target {
        floors.push(FloorFailure {
            check: "touch target: primary action".into(),
            measured: format!("{:.3}", observation.primary_height),
            floor: format!("{:.3}", reference.accessibility.min_touch_target),
        });
    }

    // Semantics: every label the contract names must be published.
    let published: Vec<String> = app.labels();
    let labels_missing: Vec<String> = screen
        .expected_labels()
        .iter()
        .filter(|label| !published.iter().any(|p| p == *label))
        .map(|label| label.to_string())
        .collect();

    // Pixels: the deterministic fingerprint, rendered twice. The registry is
    // mandatory here, not optional — without it the renderer skips Text nodes
    // entirely and the "pixel layer" silently measures only rectangles.
    let resources = app.host().resources();
    let first = PixelSnapshot::of(app.host().scene(), Some(resources), window, 1.0);
    let second = PixelSnapshot::of(app.host().scene(), Some(resources), window, 1.0);
    let pixel_stable = first.hash == second.hash;

    EvaluationReport {
        screen: format!("{screen:?}"),
        diff,
        floors,
        labels_missing,
        pixel_hash: first.hash,
        pixel_stable,
        observation,
    }
}

/// A small record of one painted text run, extracted from the scene.
struct TextRun {
    color: floom_types::Color,
    height: f64,
}

fn text_node_at(nodes: &[floom_scene::SceneNode], origin: Point) -> Option<TextRun> {
    nodes.iter().find_map(|node| match node {
        floom_scene::SceneNode::Text {
            origin: at,
            size,
            color,
            ..
        } if (at.x - origin.x).abs() < 0.5 && (at.y - origin.y).abs() < 0.5 => Some(TextRun {
            color: *color,
            height: size.height,
        }),
        _ => None,
    })
}
