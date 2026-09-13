//! Neutral accessibility description carried by widgets through the core contract.
//!
//! `floem-core` must not depend on `floem-accessibility` (the framework layer
//! depends on core, not the reverse), so the role/state vocabulary lives here in
//! engine-neutral form. The host lowers these into the concrete accessibility
//! schema when it builds the semantics tree.

/// Accessible role of a node, in vocabulary common to every backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SemanticsRole {
    #[default]
    None,
    Button,
    CheckBox,
    Radio,
    Switch,
    Slider,
    TextField,
    TextView,
    Image,
    ImageButton,
    Link,
    Heading,
    StaticText,
    List,
    ListItem,
    Group,
    Scrollable,
    ProgressBar,
    Tab,
    TabBar,
    Dialog,
    MenuItem,
    /// Framework or application-specific role.
    Custom(u32),
}

/// State flags reported to assistive technology.
///
/// A plain `u32` newtype (not `bitflags`) so core stays dependency-free; the
/// flags mirror `floem-accessibility::SemanticsState` one-for-one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SemanticsState(u32);

impl SemanticsState {
    pub const EMPTY: Self = Self(0);
    pub const SELECTED: Self = Self(1 << 0);
    pub const CHECKED: Self = Self(1 << 1);
    pub const DISABLED: Self = Self(1 << 2);
    pub const HIDDEN: Self = Self(1 << 3);
    pub const FOCUSED: Self = Self(1 << 4);
    pub const EXPANDED: Self = Self(1 << 5);
    pub const COLLAPSED: Self = Self(1 << 6);
    pub const ENABLED: Self = Self(1 << 7);
    pub const FOCUSABLE: Self = Self(1 << 8);
    pub const SCROLLABLE: Self = Self(1 << 9);
    pub const LIVE: Self = Self(1 << 10);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn contains(self, other: SemanticsState) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn union(self, other: SemanticsState) -> SemanticsState {
        SemanticsState(self.0 | other.0)
    }
}

impl Default for SemanticsState {
    fn default() -> Self {
        Self(0)
    }
}

/// Description of one accessible element, engine-neutral.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticsHint {
    pub role: SemanticsRole,
    pub label: Option<String>,
    pub state: SemanticsState,
}

impl SemanticsHint {
    /// Decorative / transparent: skipped by assistive technology.
    pub const fn decorative() -> Self {
        Self {
            role: SemanticsRole::None,
            label: None,
            state: SemanticsState::empty(),
        }
    }

    pub fn labeled(role: SemanticsRole, label: impl Into<String>) -> Self {
        Self {
            role,
            label: Some(label.into()),
            state: SemanticsState::empty(),
        }
    }

    /// Attaches accessibility state (`CHECKED`, `DISABLED`, `FOCUSED`, …).
    pub fn with_state(mut self, state: SemanticsState) -> Self {
        self.state = self.state.union(state);
        self
    }
}
