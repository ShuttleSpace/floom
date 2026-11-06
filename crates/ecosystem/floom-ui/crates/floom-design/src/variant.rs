use floom_style::StyleState;

/// An emphasis level for a control.
///
/// The five names are Material's, but every system has the same five slots
/// under different spellings (Cupertino's *filled* / *tinted* / *plain*,
/// Fluent's *primary* / *standard* / *subtle*). Keeping one enum means a
/// gallery and a set of tests are written once; a system that lacks a slot
/// maps it onto its nearest neighbour and says so in its own docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Variant {
    /// Highest emphasis: a solid fill in the accent colour.
    Filled,
    /// Secondary emphasis: a muted fill derived from the accent.
    Tonal,
    /// Medium emphasis: transparent, with a visible outline.
    Outlined,
    /// Low emphasis: text only, no fill or border at rest.
    Text,
    /// A filled surface lifted off the page by a shadow.
    Elevated,
}

impl Variant {
    pub const ALL: [Variant; 5] = [
        Variant::Filled,
        Variant::Tonal,
        Variant::Outlined,
        Variant::Text,
        Variant::Elevated,
    ];

    /// Whether this variant is expected to carry a visible border at rest.
    ///
    /// The systems assert against this rather than each hard-coding the rule:
    /// `Outlined` is the only one that is *defined* by its outline, so a system
    /// that fails to give it a border width is wrong by its own contract.
    pub fn has_border_at_rest(self) -> bool {
        matches!(self, Variant::Outlined)
    }

    /// Whether this variant is expected to cast an elevation at rest.
    pub fn has_elevation_at_rest(self) -> bool {
        matches!(self, Variant::Elevated)
    }

    /// The label a gallery prints under the sample.
    pub fn label(self) -> &'static str {
        match self {
            Variant::Filled => "Filled",
            Variant::Tonal => "Tonal",
            Variant::Outlined => "Outlined",
            Variant::Text => "Text",
            Variant::Elevated => "Elevated",
        }
    }
}

/// A control's size step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ControlSize {
    Sm,
    Md,
    Lg,
}

impl ControlSize {
    pub const ALL: [ControlSize; 3] = [ControlSize::Sm, ControlSize::Md, ControlSize::Lg];

    /// Pick the value for this step.
    ///
    /// Every system holds the same three-variant table of metrics, so the
    /// selection lives here instead of being re-matched four times.
    pub fn pick<T>(self, sm: T, md: T, lg: T) -> T {
        match self {
            ControlSize::Sm => sm,
            ControlSize::Md => md,
            ControlSize::Lg => lg,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ControlSize::Sm => "sm",
            ControlSize::Md => "md",
            ControlSize::Lg => "lg",
        }
    }
}

/// One resolved interaction state, for the purpose of styling.
///
/// This is a single state rather than `floom_style::StyleState`'s flag set
/// because a gallery has to enumerate *disjoint* samples: "hovered and pressed
/// and disabled at once" has a resolution order but no meaningful swatch.
/// Callers that do have a flag set convert with [`Self::to_style_state`] or
/// build the union themselves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InteractionState {
    Rest,
    Hovered,
    Pressed,
    Focused,
    Disabled,
    Selected,
}

impl InteractionState {
    pub const ALL: [InteractionState; 6] = [
        InteractionState::Rest,
        InteractionState::Hovered,
        InteractionState::Pressed,
        InteractionState::Focused,
        InteractionState::Disabled,
        InteractionState::Selected,
    ];

    pub fn to_style_state(self) -> StyleState {
        match self {
            InteractionState::Rest => StyleState::NORMAL,
            InteractionState::Hovered => StyleState::HOVER,
            InteractionState::Pressed => StyleState::PRESSED,
            InteractionState::Focused => StyleState::FOCUSED,
            InteractionState::Disabled => StyleState::DISABLED,
            InteractionState::Selected => StyleState::SELECTED,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            InteractionState::Rest => "rest",
            InteractionState::Hovered => "hover",
            InteractionState::Pressed => "press",
            InteractionState::Focused => "focus",
            InteractionState::Disabled => "disabled",
            InteractionState::Selected => "selected",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_keeps_its_defining_shape_rule() {
        // Only `Outlined` is defined by its border and only `Elevated` by its
        // shadow. If this drifts, the design systems' own assertions become
        // vacuous, so pin it.
        let bordered: Vec<_> = Variant::ALL
            .into_iter()
            .filter(|v| v.has_border_at_rest())
            .collect();
        assert_eq!(bordered, vec![Variant::Outlined]);
        let elevated: Vec<_> = Variant::ALL
            .into_iter()
            .filter(|v| v.has_elevation_at_rest())
            .collect();
        assert_eq!(elevated, vec![Variant::Elevated]);
    }

    #[test]
    fn states_map_onto_distinct_style_flags() {
        let mut seen = Vec::new();
        for state in InteractionState::ALL {
            let flag = state.to_style_state();
            assert!(!seen.contains(&flag), "{state:?} collides with {flag:?}");
            seen.push(flag);
        }
        assert_eq!(seen.len(), InteractionState::ALL.len());
    }

    #[test]
    fn size_steps_are_ordered_smallest_first() {
        // `ALL` is the canonical order and the derived `Ord` must agree with it:
        // callers sort size lists, so a declaration-order mismatch would make
        // `Sm` sort after `Lg` in a menu.
        assert!(
            ControlSize::ALL.windows(2).all(|w| w[0] < w[1]),
            "declaration order disagrees with the derived Ord"
        );
        let mut sorted = ControlSize::ALL;
        sorted.sort();
        assert_eq!(sorted, ControlSize::ALL);
    }

    #[test]
    fn pick_selects_the_matching_step() {
        assert_eq!(ControlSize::Sm.pick(1, 2, 3), 1);
        assert_eq!(ControlSize::Md.pick(1, 2, 3), 2);
        assert_eq!(ControlSize::Lg.pick(1, 2, 3), 3);
    }
}
