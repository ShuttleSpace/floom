//! Opt-in downcasting for widgets stored behind a trait object.
//!
//! `WidgetAny` itself now lives in `floom-core` (it is the type-erasure
//! contract, not widget-specific). This module implements it for every shipped
//! widget and bridges the crate's accessibility vocabulary
//! ([`crate::primitives::SemanticsHint`], which names `floom-accessibility`
//! types) into the engine-neutral [`floom_core::SemanticsHint`] that `floom-core`
//! owns. The host lowers that neutral hint into the concrete accessibility
//! schema when it builds the semantics tree.

use std::any::Any;

use floom_accessibility::SemanticsState as A11yState;
use floom_core::{
    SemanticsHint as CoreHint, SemanticsRole as CoreRole, SemanticsState as CoreState,
};

use crate::primitives::SemanticsHint;

/// Lowers this crate's accessibility role vocabulary into the engine-neutral
/// one `floom-core` owns.
///
/// Public so a sibling widget crate can publish semantics through the same
/// mapping instead of keeping a second copy of the table — two copies is how
/// the two vocabularies drift apart.
pub fn to_core_role(role: floom_accessibility::SemanticsRole) -> CoreRole {
    use floom_accessibility::SemanticsRole as A;
    match role {
        A::None => CoreRole::None,
        A::Button => CoreRole::Button,
        A::CheckBox => CoreRole::CheckBox,
        A::Radio => CoreRole::Radio,
        A::Switch => CoreRole::Switch,
        A::Slider => CoreRole::Slider,
        A::TextField => CoreRole::TextField,
        A::TextView => CoreRole::TextView,
        A::Image => CoreRole::Image,
        A::ImageButton => CoreRole::ImageButton,
        A::Link => CoreRole::Link,
        A::Heading => CoreRole::Heading,
        A::StaticText => CoreRole::StaticText,
        A::List => CoreRole::List,
        A::ListItem => CoreRole::ListItem,
        A::Group => CoreRole::Group,
        A::Scrollable => CoreRole::Scrollable,
        A::ProgressBar => CoreRole::ProgressBar,
        A::Tab => CoreRole::Tab,
        A::TabBar => CoreRole::TabBar,
        A::Dialog => CoreRole::Dialog,
        A::MenuItem => CoreRole::MenuItem,
        A::Custom(v) => CoreRole::Custom(v),
    }
}

/// Lowers this crate's accessibility state flags into the engine-neutral ones.
/// Public for the same reason as [`to_core_role`].
pub fn to_core_state(state: A11yState) -> CoreState {
    let mut out = CoreState::empty();
    let mut add = |flag: A11yState, core: CoreState| {
        if state.contains(flag) {
            out = out.union(core);
        }
    };
    use CoreState as C;
    add(A11yState::SELECTED, C::SELECTED);
    add(A11yState::CHECKED, C::CHECKED);
    add(A11yState::DISABLED, C::DISABLED);
    add(A11yState::HIDDEN, C::HIDDEN);
    add(A11yState::FOCUSED, C::FOCUSED);
    add(A11yState::EXPANDED, C::EXPANDED);
    add(A11yState::COLLAPSED, C::COLLAPSED);
    add(A11yState::ENABLED, C::ENABLED);
    add(A11yState::FOCUSABLE, C::FOCUSABLE);
    add(A11yState::SCROLLABLE, C::SCROLLABLE);
    add(A11yState::LIVE, C::LIVE);
    out
}

impl From<SemanticsHint> for CoreHint {
    fn from(h: SemanticsHint) -> Self {
        CoreHint {
            role: to_core_role(h.role),
            label: h.label,
            state: to_core_state(h.state),
        }
    }
}

macro_rules! impl_widget_any {
    ($($ty:ty),* $(,)?) => {
        $(
            impl floom_core::WidgetAny for $ty {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn semantics_hint(&self) -> Option<CoreHint> {
                    Some(<$ty>::semantics_hint(self).into())
                }
            }
        )*
    };
}

/// Implements `WidgetAny` for a widget that publishes no semantics of its own.
///
/// Distinct from `impl_widget_any!` because that macro assumes a
/// `semantics_hint()` returning this crate's own `SemanticsHint`. A decorative
/// composite has no such method, and `Option<CoreHint>` is already `None` for
/// it — so the honest answer is to *say* nothing rather than to invent an empty
/// hint.
macro_rules! impl_widget_any_decorative {
    ($($ty:ty),* $(,)?) => {
        $(
            impl floom_core::WidgetAny for $ty {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn semantics_hint(&self) -> Option<CoreHint> {
                    // No semantics method of its own, and therefore nothing to
                    // publish: `None` is what an unlabelled node reports.
                    None
                }
            }
        )*
    };
}

impl_widget_any!(
    crate::badge::Badge,
    crate::primitives::Text,
    crate::labeled::Labeled,
    crate::primitives::Icon,
    crate::vector_icon::VectorIcon,
    crate::primitives::Image,
    crate::primitives::Container,
    crate::surface::Surface,
    crate::primitives::Spacer,
    crate::primitives::Divider,
    crate::controls::Button,
    crate::controls::Pressable,
    crate::controls::TextField,
    crate::controls::Checkbox,
    crate::controls::Radio,
    crate::controls::Switch,
    crate::controls::Slider,
    crate::controls::Progress,
    crate::controls::Select,
    crate::controls::SelectPopup,
    crate::compose::Stack,
    crate::nav::NavHost,
    crate::compose::Conditional,
    crate::compose::PresenceLeaf,
    crate::compose::Crossfade,
    crate::compose::Opacity,
    crate::compose::Overlay,
    crate::compose::Dialog,
    crate::compose::Tabs,
    crate::compose::Accordion,
    crate::compose::SelectGroup,
    crate::compose::Popup,
    crate::compose::BottomSheet,
    crate::navigation::NavigationBar,
    crate::navigation::NavigationRail,
    crate::navigation::NavigationDrawer,
    crate::native::NativeSurface,
);

// The internals of `Accordion` and `SelectGroup`: decorative composites with no
// semantics method of their own, so they need the other macro. A `SelectGroup`'s
// row publishes nothing because the *`Pressable` inside it* publishes the radio or
// checkbox role and the selected state — a row that also claimed a role would
// double up in the semantics tree.
impl_widget_any_decorative!(
    crate::compose::AccordionPanelLeaf,
    crate::compose::AccordionIndicatorSlot,
    crate::compose::SelectGroupRow,
    // The ripple is a *wrapper* whose whole job is to paint ink over whatever it
    // holds, so it publishes nothing: the button inside it is what names itself and
    // carries the role. It was missing here entirely, which meant `Ripple` could be
    // constructed, returned from a public method, and then not mounted —
    // `Box::new(Ripple::new()..)` failed with "not a Floom 2.0 `Widget`". Found by
    // composing one for the first time; see `IconButton`.
    crate::ripple::Ripple,
);
