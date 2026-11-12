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
    // The badge names itself: it is the count, and that is what a reader
    // hears. It was missing from this list, which meant it could be built,
    // returned from `view()`, and then not mounted.
    crate::badge::Badge,
    crate::bottom_sheet::BottomSheet,
    crate::checkbox::Checkbox,
    // The chip names itself too: it is a button when it can be pressed, and
    // static text when it cannot.
    crate::chip::Chip,
    // The counter names itself as a group, so it publishes its own role rather
    // than leaving it to the `Pressable`s inside it. It was missing from this
    // list entirely, which meant `Counter` could be built, returned from
    // `view()`, and then not mounted -- `Box::new(Counter::new(1))` failed with
    // "not a Floom 2.0 `Widget`". Found by the component gallery, which has to
    // mount every component.
    crate::counter::Counter,
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
    // The tooltip names itself once its bubble is up: until then it says
    // nothing, because there is nothing to say. It was missing from this
    // list, which meant it could be built, returned from `view()`, and then
    // not mounted.
    crate::tooltip::Tooltip,
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
    // The icon button publishes nothing of its own: the `Pressable` inside it
    // carries the button role and the icon inside *that* carries the label.
    // It was missing here for the same reason `Counter` was -- it could be
    // built and returned from `view()`, and then not mounted.
    crate::icon_button::IconButton,
    // The bottom navigation bar publishes nothing of its own: the
    // destinations' `Pressable`s are what speak. It was missing here -- the
    // same gap the others had -- so it could be built, returned from `view()`,
    // and then not mounted. The gallery found it, because it has to mount every
    // component it shows. (`TabsBar` and `WindowViewer` already implement
    // `WidgetAny` by hand in their own modules, so they are not listed here.)
    crate::bottom_navigation::BottomNavigationBar,
);
