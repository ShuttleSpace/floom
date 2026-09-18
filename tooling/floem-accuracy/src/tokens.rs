//! The token table a screen implementation consumes, and the edits an agent
//! may propose against it.
//!
//! The implementation under test is a function of these tokens and nothing
//! else, which is exactly the surface a design-token-driven app exposes: an
//! agent repairs an implementation by naming a token path and the value the
//! design contract expects there.

use floem_types::Color;

/// Every visual decision a gallery screen takes, in design-token form.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenTokens {
    /// Gap between siblings in a content stack.
    pub gap: f64,
    /// Padding inside a card, uniform on all sides.
    pub padding: f64,
    /// Corner radius of a card.
    pub radius: f32,
    /// Title font size.
    pub title_size: f32,
    /// Body font size.
    pub body_size: f32,
    /// Height of an action box — the touch target.
    pub control_height: f32,
    pub accent: Color,
    pub on_accent: Color,
    pub surface: Color,
    pub card: Color,
    pub text: Color,
}

impl ScreenTokens {
    /// The values the reference design contract specifies.
    pub fn reference() -> Self {
        Self {
            gap: 12.0,
            padding: 16.0,
            radius: 10.0,
            title_size: 18.0,
            body_size: 14.0,
            control_height: 48.0,
            // Accent carries white text at WCAG-AA contrast with margin to
            // spare; #2F6FED would sit at 4.55, one rounding away from failing
            // the floor the same fixture declares.
            accent: Color::rgb(0x1E, 0x5F, 0xD6),
            on_accent: Color::rgb(0xFF, 0xFF, 0xFF),
            surface: Color::rgb(0xFF, 0xFF, 0xFF),
            card: Color::rgb(0xF4, 0xF6, 0xF8),
            text: Color::rgb(0x17, 0x21, 0x2B),
        }
    }

    /// Drift the named tokens the way a broken implementation would. Paths are
    /// the same names [`TokenEdit`] repairs, so a test can say exactly what it
    /// broke.
    pub fn drifted(&self, paths: &[&str]) -> Self {
        let mut tokens = self.clone();
        for &path in paths {
            match path {
                "gap" => tokens.gap += 6.0,
                "padding" => tokens.padding += 8.0,
                "radius" => tokens.radius -= 6.0,
                "title_size" => tokens.title_size += 2.0,
                "body_size" => tokens.body_size += 2.0,
                "control_height" => tokens.control_height -= 16.0,
                "accent" => tokens.accent = Color::rgb(0xBA, 0x68, 0xC8),
                "on_accent" => tokens.on_accent = Color::rgb(0x12, 0x12, 0x12),
                "surface" => tokens.surface = Color::rgb(0xF0, 0xF0, 0xF0),
                "card" => tokens.card = Color::rgb(0xE0, 0xE0, 0xE0),
                "text" => tokens.text = Color::rgb(0xB0, 0xB7, 0xBD),
                other => panic!("unknown drift path {other:?}"),
            }
        }
        tokens
    }

    /// Every drift the token table can express.
    pub const ALL_DRIFTS: &'static [&'static str] = &[
        "gap",
        "padding",
        "radius",
        "title_size",
        "body_size",
        "control_height",
        "accent",
        "on_accent",
        "surface",
        "card",
        "text",
    ];
}

/// One edit an agent proposes: set the token a contract path maps to.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenEdit {
    Gap(f64),
    Padding(f64),
    Radius(f32),
    TitleSize(f32),
    BodySize(f32),
    ControlHeight(f32),
    Accent(Color),
    OnAccent(Color),
    Surface(Color),
    Card(Color),
    Text(Color),
}

impl TokenEdit {
    /// Applies the edit to an implementation's token table.
    pub fn apply(&self, tokens: &mut ScreenTokens) {
        match *self {
            TokenEdit::Gap(v) => tokens.gap = v,
            TokenEdit::Padding(v) => tokens.padding = v,
            TokenEdit::Radius(v) => tokens.radius = v,
            TokenEdit::TitleSize(v) => tokens.title_size = v,
            TokenEdit::BodySize(v) => tokens.body_size = v,
            TokenEdit::ControlHeight(v) => tokens.control_height = v,
            TokenEdit::Accent(v) => tokens.accent = v,
            TokenEdit::OnAccent(v) => tokens.on_accent = v,
            TokenEdit::Surface(v) => tokens.surface = v,
            TokenEdit::Card(v) => tokens.card = v,
            TokenEdit::Text(v) => tokens.text = v,
        }
    }

    /// The IR-schema property this edit lowers to. An agent that reads the
    /// component schema (Phase 25.2) authors edits a validator accepts, so
    /// every edit must name a declared property.
    pub fn property_name(&self) -> &'static str {
        match self {
            TokenEdit::Gap(_) => "gap",
            TokenEdit::Padding(_) => "padding",
            TokenEdit::Radius(_) => "radius",
            TokenEdit::TitleSize(_) | TokenEdit::BodySize(_) => "font_size",
            TokenEdit::ControlHeight(_) => "height",
            TokenEdit::Accent(_) | TokenEdit::Surface(_) | TokenEdit::Card(_) => "background",
            TokenEdit::OnAccent(_) | TokenEdit::Text(_) => "foreground",
        }
    }

    /// Maps one `compare()` disagreement back to the edit that would fix it.
    ///
    /// `compare` renders values with its own canonical spellings — three
    /// decimals for numbers, `to_hex` for colours — so the expected side of an
    /// entry parses deterministically. Paths outside the token vocabulary
    /// (dimensions, the copied animation/accessibility blocks) return `None`:
    /// they are not things a token edit can repair.
    pub fn from_diff_entry(entry: &floem_ui_ir::DiffEntry) -> Option<Self> {
        let parse_number = |text: &str| text.parse::<f32>().ok();
        let parse_color = |text: &str| {
            floem_ui_ir::Rgba::from_hex(text)
                .map(|rgba| Color::rgba(rgba.r, rgba.g, rgba.b, rgba.a))
        };
        match entry.path.as_str() {
            "spacing.gap" => parse_number(&entry.expected).map(|v| Self::Gap(v as f64)),
            "spacing.padding.top" | "spacing.padding.left" => {
                parse_number(&entry.expected).map(|v| Self::Padding(v as f64))
            }
            "radius.corner" => parse_number(&entry.expected).map(Self::Radius),
            "typography.title.font_size" => parse_number(&entry.expected).map(Self::TitleSize),
            "typography.body.font_size" => parse_number(&entry.expected).map(Self::BodySize),
            "colors.accent.value" => parse_color(&entry.expected).map(Self::Accent),
            "colors.on_accent.value" => parse_color(&entry.expected).map(Self::OnAccent),
            "colors.surface.value" => parse_color(&entry.expected).map(Self::Surface),
            "colors.card.value" => parse_color(&entry.expected).map(Self::Card),
            "colors.text.value" => parse_color(&entry.expected).map(Self::Text),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drift_moves_exactly_the_named_tokens() {
        let base = ScreenTokens::reference();
        let drifted = base.drifted(&["gap", "accent"]);
        assert_eq!(drifted.gap, base.gap + 6.0);
        assert_ne!(drifted.accent, base.accent);
        assert_eq!(drifted.padding, base.padding, "padding must not move");
        assert_eq!(drifted.radius, base.radius, "radius must not move");
    }

    #[test]
    fn edits_parse_back_from_the_diff_vocabulary() {
        let entry = |path: &str, expected: &str| floem_ui_ir::DiffEntry {
            path: path.into(),
            expected: expected.into(),
            actual: String::new(),
        };
        assert_eq!(
            TokenEdit::from_diff_entry(&entry("spacing.gap", "12.000")),
            Some(TokenEdit::Gap(12.0))
        );
        assert_eq!(
            TokenEdit::from_diff_entry(&entry("colors.accent.value", "#1e5fd6")),
            Some(TokenEdit::Accent(Color::rgb(0x1E, 0x5F, 0xD6)))
        );
        // Paths outside the token vocabulary are not repairable by edits.
        assert_eq!(
            TokenEdit::from_diff_entry(&entry("dimensions.width", "360.000")),
            None
        );
        assert_eq!(
            TokenEdit::from_diff_entry(&entry("animation.easing", "ease-out")),
            None
        );
    }

    #[test]
    fn every_edit_names_a_declared_schema_property() {
        let edits = [
            TokenEdit::Gap(8.0),
            TokenEdit::Padding(12.0),
            TokenEdit::Radius(6.0),
            TokenEdit::TitleSize(16.0),
            TokenEdit::BodySize(15.0),
            TokenEdit::ControlHeight(44.0),
            TokenEdit::Accent(Color::rgb(0, 0, 0)),
            TokenEdit::OnAccent(Color::rgb(0, 0, 0)),
            TokenEdit::Surface(Color::rgb(0, 0, 0)),
            TokenEdit::Card(Color::rgb(0, 0, 0)),
            TokenEdit::Text(Color::rgb(0, 0, 0)),
        ];
        for edit in edits {
            assert!(floem_ui_ir::prop_is_declared(edit.property_name()));
        }
    }
}
