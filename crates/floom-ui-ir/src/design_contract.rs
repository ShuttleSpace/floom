//! The design contract (Phase 26.1).
//!
//! A design and its implementation agree on more than a tree of nodes: the
//! dimensions, the spacing, the type ramp, the colour roles, the corner
//! radius, the states, the motion and the accessibility floor. The contract is
//! that agreement as *data* — versioned, serialisable, and comparable.
//!
//! Comparison is the point. `compare` walks two contracts in one fixed field
//! order and reports every disagreement as a `(path, expected, actual)` entry,
//! so the output is a diff a tool can consume and a person can read, and the
//! same pair of inputs always produces the same bytes. Paths name the contract
//! field (`"colors.accent"`, `"states.hover.background"`), not the node that
//! produced it — the contract describes the design, not the walk that emitted
//! it.

use serde::{Deserialize, Serialize};

use crate::value::Rgba;

/// Version of the design contract.
///
/// The same rule the IR and the inspector apply, for the same reason: a
/// consumer must never be handed fields it cannot represent. Same major and an
/// older-or-equal minor loads; a newer minor or a different major is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
}

impl ContractVersion {
    /// The version this build writes.
    pub const CURRENT: Self = Self { major: 1, minor: 0 };

    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Whether a contract written at `written` can be read by a reader at
    /// `self`.
    pub fn accepts(self, written: ContractVersion) -> bool {
        written.major == self.major && written.minor <= self.minor
    }
}

/// One dimension family: the box the design is drawn for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: f32,
    pub height: f32,
}

/// Spacing intent: the gap between siblings and the padding inside the box,
/// as `[top, right, bottom, left]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Spacing {
    pub gap: f32,
    /// `[top, right, bottom, left]`, the CSS order.
    pub padding: [f32; 4],
}

/// Corner radius: one value, as the IR's `radius` property is one value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Radius {
    pub corner: f32,
}

/// One step of the type ramp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeStyle {
    pub name: String,
    pub font_size: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<String>,
}

/// One named colour role and its value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorRole {
    pub name: String,
    pub value: Rgba,
}

/// One state (or variant) the component declares, with the properties it
/// changes. An empty `overrides` is a variant that only names itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateStyle {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<Override>,
}

/// A property a state changes, with the value it changes it to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Override {
    pub property: String,
    pub value: ContractValue,
}

/// A value a state override can take. Closed on purpose: the contract is
/// compared as text, and a value that could be anything compares as nothing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractValue {
    Color(Rgba),
    Number(f32),
    Text(String),
}

/// Motion: how long a transition runs and on what curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub duration_ms: u32,
    pub easing: String,
}

/// The accessibility floor: the role, the announced label, the minimum
/// contrast the design accepts, and the minimum touch target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accessibility {
    pub role: String,
    pub label: String,
    pub min_contrast: f64,
    pub min_touch_target: f32,
}

/// The whole agreement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignContract {
    pub version: ContractVersion,
    pub name: String,
    pub dimensions: Dimensions,
    pub spacing: Spacing,
    pub radius: Radius,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub typography: Vec<TypeStyle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<ColorRole>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub states: Vec<StateStyle>,
    pub animation: Animation,
    pub accessibility: Accessibility,
}

/// One disagreement between expected and actual.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffEntry {
    pub path: String,
    pub expected: String,
    pub actual: String,
}

/// The result of a comparison: whether it matched, and every disagreement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractDiff {
    pub matches: bool,
    pub entries: Vec<DiffEntry>,
}

/// Compare `expected` against `actual`, deterministically.
///
/// The walk order is the struct's own field order, and every list is sorted by
/// name first, so the entries come out in the same sequence for the same pair
/// of contracts regardless of how either was built. A name present on only one
/// side is a disagreement like any other: `<absent>` stands in for the missing
/// value.
pub fn compare(expected: &DesignContract, actual: &DesignContract) -> ContractDiff {
    let mut entries = Vec::new();
    let mut diff = |path: String, expected: String, actual: String| {
        if expected != actual {
            entries.push(DiffEntry {
                path,
                expected,
                actual,
            });
        }
    };

    // dimensions + spacing + radius: scalars in field order.
    let dims = (&expected.dimensions, &actual.dimensions);
    diff(
        "dimensions.width".into(),
        num(dims.0.width),
        num(dims.1.width),
    );
    diff(
        "dimensions.height".into(),
        num(dims.0.height),
        num(dims.1.height),
    );
    diff(
        "spacing.gap".into(),
        num(expected.spacing.gap),
        num(actual.spacing.gap),
    );
    for (i, side) in ["top", "right", "bottom", "left"].iter().enumerate() {
        diff(
            format!("spacing.padding.{side}"),
            num(expected.spacing.padding[i]),
            num(actual.spacing.padding[i]),
        );
    }
    diff(
        "radius.corner".into(),
        num(expected.radius.corner),
        num(actual.radius.corner),
    );

    // typography / colors / states: sorted unions by name.
    compare_named(
        &mut diff,
        "typography",
        &typography_map(expected),
        &typography_map(actual),
    );
    compare_named(
        &mut diff,
        "colors",
        &colors_map(expected),
        &colors_map(actual),
    );
    compare_named(
        &mut diff,
        "states",
        &states_map(expected),
        &states_map(actual),
    );

    // animation + accessibility.
    diff(
        "animation.duration_ms".into(),
        expected.animation.duration_ms.to_string(),
        actual.animation.duration_ms.to_string(),
    );
    diff(
        "animation.easing".into(),
        expected.animation.easing.clone(),
        actual.animation.easing.clone(),
    );
    diff(
        "accessibility.role".into(),
        expected.accessibility.role.clone(),
        actual.accessibility.role.clone(),
    );
    diff(
        "accessibility.label".into(),
        expected.accessibility.label.clone(),
        actual.accessibility.label.clone(),
    );
    diff(
        "accessibility.min_contrast".into(),
        format!("{:.3}", expected.accessibility.min_contrast),
        format!("{:.3}", actual.accessibility.min_contrast),
    );
    diff(
        "accessibility.min_touch_target".into(),
        num(expected.accessibility.min_touch_target),
        num(actual.accessibility.min_touch_target),
    );

    ContractDiff {
        matches: entries.is_empty(),
        entries,
    }
}

/// The closure type `compare_named` reports through.
type Report<'d> = &'d mut dyn FnMut(String, String, String);

/// Compare two name→rendered-value maps: same names compared pairwise, names
/// on one side reported against `<absent>`.
fn compare_named(
    diff: Report<'_>,
    family: &str,
    expected: &[(String, Vec<(String, String)>)],
    actual: &[(String, Vec<(String, String)>)],
) {
    let find = |map: &[(String, Vec<(String, String)>)], name: &str| {
        map.iter()
            .find(|(n, _)| n == name)
            .map(|(_, fields)| fields.clone())
    };
    let names: Vec<&String> = expected
        .iter()
        .map(|(n, _)| n)
        .chain(actual.iter().map(|(n, _)| n))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    for name in names {
        let expected_fields = find(expected, name);
        let actual_fields = find(actual, name);
        match (&expected_fields, &actual_fields) {
            (Some(exp), Some(act)) => {
                let keys: Vec<&String> = exp
                    .iter()
                    .map(|(k, _)| k)
                    .chain(act.iter().map(|(k, _)| k))
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                for key in keys {
                    let expected_value = exp
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| ABSENT.into());
                    let actual_value = act
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| ABSENT.into());
                    diff(
                        format!("{family}.{name}.{key}"),
                        expected_value,
                        actual_value,
                    );
                }
            }
            (Some(exp), None) => {
                for (key, value) in exp {
                    diff(
                        format!("{family}.{name}.{key}"),
                        value.clone(),
                        ABSENT.into(),
                    );
                }
            }
            (None, Some(act)) => {
                for (key, value) in act {
                    diff(
                        format!("{family}.{name}.{key}"),
                        ABSENT.into(),
                        value.clone(),
                    );
                }
            }
            (None, None) => unreachable!("the name came from one of the two sides"),
        }
    }
}

const ABSENT: &str = "<absent>";

fn typography_map(c: &DesignContract) -> Vec<(String, Vec<(String, String)>)> {
    let mut styles: Vec<&TypeStyle> = c.typography.iter().collect();
    styles.sort_by(|a, b| a.name.cmp(&b.name));
    styles
        .into_iter()
        .map(|s| {
            let mut fields = vec![
                ("font_size".to_string(), num(s.font_size)),
                ("weight".to_string(), opt(&s.weight)),
            ];
            if let Some(lh) = s.line_height {
                fields.push(("line_height".to_string(), num(lh)));
            }
            (s.name.clone(), fields)
        })
        .collect()
}

fn colors_map(c: &DesignContract) -> Vec<(String, Vec<(String, String)>)> {
    let mut colors: Vec<&ColorRole> = c.colors.iter().collect();
    colors.sort_by(|a, b| a.name.cmp(&b.name));
    colors
        .into_iter()
        .map(|role| {
            (
                role.name.clone(),
                vec![("value".to_string(), role.value.to_hex())],
            )
        })
        .collect()
}

fn states_map(c: &DesignContract) -> Vec<(String, Vec<(String, String)>)> {
    let mut states: Vec<&StateStyle> = c.states.iter().collect();
    states.sort_by(|a, b| a.name.cmp(&b.name));
    states
        .into_iter()
        .map(|s| {
            let mut overrides: Vec<&Override> = s.overrides.iter().collect();
            overrides.sort_by(|a, b| a.property.cmp(&b.property));
            (
                s.name.clone(),
                overrides
                    .into_iter()
                    .map(|o| (o.property.clone(), value_text(&o.value)))
                    .collect(),
            )
        })
        .collect()
}

fn value_text(value: &ContractValue) -> String {
    match value {
        ContractValue::Color(c) => c.to_hex(),
        ContractValue::Number(n) => num(*n),
        ContractValue::Text(t) => t.clone(),
    }
}

fn opt(value: &Option<String>) -> String {
    value.clone().unwrap_or_else(|| ABSENT.to_string())
}

/// The canonical float spelling: three decimals.
///
/// Rendering *is* the comparison: `8`, `8.0` and `8.0004` all render `8.000`
/// and therefore match, while `8.5` renders `8.500` and disagrees. Design
/// values are specified in whole or half units, so a thousandth is formatting
/// noise, not a design change — and one rule does both jobs, where a separate
/// epsilon would be a second implementation of "how close is close".
fn num(value: f32) -> String {
    format!("{value:.3}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> DesignContract {
        DesignContract {
            version: ContractVersion::CURRENT,
            name: "Card".into(),
            dimensions: Dimensions {
                width: 320.0,
                height: 120.0,
            },
            spacing: Spacing {
                gap: 8.0,
                padding: [16.0, 12.0, 16.0, 12.0],
            },
            radius: Radius { corner: 12.0 },
            typography: vec![TypeStyle {
                name: "body".into(),
                font_size: 14.0,
                line_height: Some(20.0),
                weight: Some("regular".into()),
            }],
            colors: vec![ColorRole {
                name: "accent".into(),
                value: Rgba::opaque(0x22, 0x66, 0xff),
            }],
            states: vec![StateStyle {
                name: "hover".into(),
                overrides: vec![Override {
                    property: "background".into(),
                    value: ContractValue::Color(Rgba::opaque(0x22, 0x66, 0xff)),
                }],
            }],
            animation: Animation {
                duration_ms: 150,
                easing: "ease_out".into(),
            },
            accessibility: Accessibility {
                role: "button".into(),
                label: "Card".into(),
                min_contrast: 4.5,
                min_touch_target: 44.0,
            },
        }
    }

    #[test]
    fn identical_contracts_match_with_no_entries() {
        let diff = compare(&base(), &base());
        assert!(diff.matches, "{:?}", diff.entries);
        assert!(diff.entries.is_empty());
    }

    #[test]
    fn every_family_reports_its_own_path() {
        // One mutation per family; the diff must name exactly that path, with
        // the expected and actual rendered in canonical form.
        let cases: Vec<(DesignContract, &str, &str, &str)> = vec![
            (
                DesignContract {
                    dimensions: Dimensions {
                        width: 300.0,
                        ..base().dimensions
                    },
                    ..base()
                },
                "dimensions.width",
                "320.000",
                "300.000",
            ),
            (
                DesignContract {
                    spacing: Spacing {
                        gap: 12.0,
                        ..base().spacing
                    },
                    ..base()
                },
                "spacing.gap",
                "8.000",
                "12.000",
            ),
            (
                DesignContract {
                    spacing: Spacing {
                        padding: [16.0, 12.0, 16.0, 20.0],
                        ..base().spacing
                    },
                    ..base()
                },
                "spacing.padding.left",
                "12.000",
                "20.000",
            ),
            (
                DesignContract {
                    radius: Radius { corner: 0.0 },
                    ..base()
                },
                "radius.corner",
                "12.000",
                "0.000",
            ),
            (
                DesignContract {
                    animation: Animation {
                        duration_ms: 200,
                        ..base().animation
                    },
                    ..base()
                },
                "animation.duration_ms",
                "150",
                "200",
            ),
            (
                DesignContract {
                    accessibility: Accessibility {
                        min_contrast: 3.0,
                        ..base().accessibility
                    },
                    ..base()
                },
                "accessibility.min_contrast",
                "4.500",
                "3.000",
            ),
        ];
        for (actual, path, expected, found) in cases {
            let diff = compare(&base(), &actual);
            assert_eq!(diff.entries.len(), 1, "{path}: {:?}", diff.entries);
            assert_eq!(diff.entries[0].path, path);
            assert_eq!(diff.entries[0].expected, expected);
            assert_eq!(diff.entries[0].actual, found);
            assert!(!diff.matches);
        }
    }

    #[test]
    fn missing_and_extra_colours_are_reported() {
        let mut actual = base();
        actual.colors.clear();
        actual.colors.push(ColorRole {
            name: "surface".into(),
            value: Rgba::opaque(0xff, 0xff, 0xff),
        });
        let diff = compare(&base(), &actual);
        let paths: Vec<&str> = diff.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["colors.accent.value", "colors.surface.value"],
            "sorted, expected-then-actual, each against <absent>"
        );
        assert_eq!(diff.entries[0].actual, "<absent>");
        assert_eq!(diff.entries[1].expected, "<absent>");
    }

    #[test]
    fn floats_agree_within_tolerance_and_render_the_same() {
        let mut actual = base();
        actual.spacing.gap = 8.0004;
        assert!(
            compare(&base(), &actual).matches,
            "a thousandth is formatting noise"
        );
        let mut actual = base();
        actual.spacing.gap = 8.5;
        let diff = compare(&base(), &actual);
        assert_eq!(diff.entries[0].expected, "8.000");
        assert_eq!(diff.entries[0].actual, "8.500");
    }

    #[test]
    fn states_and_typography_compare_by_name_and_property() {
        let mut actual = base();
        actual.states[0].overrides[0].value = ContractValue::Color(Rgba::opaque(0, 0, 0));
        let diff = compare(&base(), &actual);
        assert_eq!(diff.entries.len(), 1);
        assert_eq!(diff.entries[0].path, "states.hover.background");
        assert_eq!(diff.entries[0].expected, "#2266ff");
        assert_eq!(diff.entries[0].actual, "#000000");

        let mut actual = base();
        actual.typography[0].font_size = 16.0;
        let diff = compare(&base(), &actual);
        assert_eq!(diff.entries.len(), 1);
        assert_eq!(diff.entries[0].path, "typography.body.font_size");
    }

    #[test]
    fn contracts_round_trip_over_json() {
        let json = serde_json::to_string_pretty(&base()).expect("serialise");
        let parsed: DesignContract = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed, base());
    }

    #[test]
    fn version_gate_accepts_current_and_older_minors_only() {
        assert!(ContractVersion::CURRENT.accepts(ContractVersion::CURRENT));
        assert!(ContractVersion::CURRENT.accepts(ContractVersion::new(1, 0)));
        assert!(!ContractVersion::CURRENT.accepts(ContractVersion::new(1, 99)));
        assert!(!ContractVersion::CURRENT.accepts(ContractVersion::new(2, 0)));
    }
}
