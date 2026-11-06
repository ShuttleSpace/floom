use std::any::Any;

use floom_core::{SemanticsHint, WidgetAny};

/// Implements [`WidgetAny`] for widgets that carry no accessibility description
/// of their own.
///
/// A widget that publishes nothing is not an error — a decorative composite has
/// nothing to say, and its inner `Pressable` or `Text` does the publishing. What
/// matters is that it can still be recovered by type after a mount.
macro_rules! impl_widget_any {
    ($($widget:ty),* $(,)?) => {
        $(
            impl WidgetAny for $widget {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn semantics_hint(&self) -> Option<SemanticsHint> {
                    None
                }
            }
        )*
    };
}

impl_widget_any!(crate::nav::NavHost);

// `Select` is the first widget here that publishes semantics of its own, so it
// needs the same lowering `floom-widgets` does: a hint naming this fork's
// accessibility vocabulary becomes the engine-neutral one `floom-core` owns.
// The mapping is called rather than re-spelled -- two copies of a role table is
// how the two vocabularies drift apart, which is why `to_core_role` /
// `to_core_state` are `pub`. It is spelled inline rather than as a `From` impl
// because `From` over two foreign types is an orphan-rule violation.
macro_rules! impl_widget_any_labelled {
    ($($ty:ty),* $(,)?) => {
        $(
            impl WidgetAny for $ty {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn semantics_hint(&self) -> Option<SemanticsHint> {
                    let hint = <$ty>::semantics_hint(self);
                    Some(SemanticsHint {
                        role: floom_widgets::to_core_role(hint.role),
                        label: hint.label,
                        state: floom_widgets::to_core_state(hint.state),
                    })
                }
            }
        )*
    };
}

impl_widget_any_labelled!(crate::select::Select, crate::select::SelectPopup);

impl_widget_any_labelled!(
    crate::accordion::Accordion,
    crate::accordion::SelectGroup,
    crate::bottom_sheet::BottomSheet,
    crate::checkbox::Checkbox,
    crate::dialog::Dialog,
    // The three navigation surfaces each publish their own role: a bar and a
    // rail are `Navigation`, a drawer is a `Dialog` once it is open and
    // decorative when it is not. Leaving them out of this list is what the
    // module doc above warns about — the omission only surfaces as a compile
    // error at the mount site, which is where `examples/floom-probe` hit it.
    crate::navigation::NavigationBar,
    crate::navigation::NavigationDrawer,
    crate::navigation::NavigationRail,
    crate::overlay::Overlay,
    crate::popup::Popup,
    crate::radio::Radio,
    crate::slider::Slider,
    crate::tabs::Tabs,
    crate::text_field::TextField,
);

// The leaves and rows are structural: they exist so a parent can hand the host
// a stable `ChildKey` per panel, per indicator or per option, and the content
// they wrap publishes the semantics. A row that also claimed a role would
// double up in the semantics tree -- the same reason `SelectGroupRow` was
// decorative where the `Pressable` inside it was not.
impl_widget_any!(
    crate::accordion::AccordionPanelLeaf,
    crate::accordion::AccordionIndicatorSlot,
    crate::accordion::SelectGroupRow,
);
