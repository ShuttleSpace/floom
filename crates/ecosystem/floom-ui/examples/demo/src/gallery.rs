//! Component gallery data for the floom-ui demo.
//!
//! `build()` returns a `Gallery { pages, overlays }` consumed by `main.rs`.
//!
//! * `pages` is the master list — one `(name, factory)` entry per component. The
//!   factory builds that component's demo on demand.
//! * `overlays` holds the floating surfaces (Dialog, Menu, Snackbar,
//!   NavigationDrawer, Sheet, Overlay) that MUST render at the window root. The
//!   original single-file demo pushed them inline into a scrollable `list`, so
//!   their `width_pct(100%).height_pct(100%)` scrim collapsed to zero height and
//!   they were invisible / unclickable. They are built once here and returned
//!   separately; `main.rs` renders them as full-window children of the root view.
//!   Their per-page demos show only a trigger button + description.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use floom::IntoView;
use floom::View;
use floom::event::{EventCx, EventPropagation};
use floom::paint::PaintCx;
use floom::reactive::{Effect, RwSignal, SignalGet, SignalUpdate};
use floom::style::{Style, StyleClassRef, StyleCx};
use floom::view::{AnyView, ViewId};
use floom::views::{Decorators, Label, Stack};
use floom_core_theme::theme::Theme;
use floom_elements::button::ButtonVariant;
use floom_elements::prelude::*;
use floom_widget_runtime::theme_signal;
use crate::showcase::{
    button_showcase, card_showcase, chip_showcase, segmented, slider_showcase, switch_showcase,
    tabs_showcase,
};
use crate::showcase_pilots::{
    alert_showcase, datatable_showcase, dialog_showcase, fab_showcase, locale_provider_showcase,
    navigation_drawer_showcase, rating_showcase, select_showcase, snackbar_showcase,
    text_field_showcase, theme_provider_showcase,
};
use crate::showcase_rollout::{
    avatar_showcase, badge_showcase, banner_showcase, checkbox_showcase, checkbox_btn_showcase,
    chip_group_showcase,
    container_showcase, counter_showcase, divider_showcase, file_input_showcase,
    grid_showcase, list_group_showcase, list_item_showcase, list_subheader_showcase,
    number_input_showcase, otp_input_showcase, pagination_showcase, progress_circular_showcase,
    progress_linear_showcase, radio_group_showcase, radio_showcase, range_slider_showcase,
    spacer_showcase, textarea_showcase, tooltip_showcase,
    text_showcase, icon_showcase, list_showcase, toolbar_showcase, app_bar_showcase,
    bottom_navigation_showcase, image_showcase, system_bar_showcase, combobox_showcase,
    autocomplete_showcase, skeleton_loader_showcase, footer_showcase,
    stepper_showcase, treeview_showcase, carousel_showcase, calendar_showcase,
    breadcrumbs_showcase, color_picker_showcase, icon_button_showcase,
    expansion_panels_showcase, speed_dial_showcase, empty_state_showcase,
    timeline_showcase,
    date_picker_showcase, time_picker_showcase, slide_group_showcase, item_group_showcase,
    button_group_showcase, table_showcase, kbd_showcase, hover_showcase,
    fade_transition_showcase, scale_transition_showcase, expand_transition_showcase,
    slide_transition_showcase,
    sparkline_showcase, data_iterator_showcase, responsive_showcase, confirm_edit_showcase,
    display_provider_showcase, defaults_provider_showcase, icons_provider_showcase,
    date_adapter_provider_showcase, parallax_showcase, validation_messages_showcase,
    window_showcase, code_showcase, virtual_scroller_showcase, app_showcase, main_showcase,
    infinite_scroller_showcase,
};

/// A page factory: build one component's demo `AnyView`.
pub type Page = Box<dyn Fn() -> AnyView>;

/// The materialized gallery: master list + root overlays.
pub struct Gallery {
    pub pages: Vec<(String, Page)>,
    pub overlays: Vec<AnyView>,
}

// ---------------------------------------------------------------------------
// SharedView: host a pre-built `Box<dyn View>` so the Form and PullToRefresh
// pages (which own `create_effect`s created once in `build()`) can be rendered
// repeatedly by `dyn_view` without rebuilding the view or leaking effects.
// `Box<dyn View>` is not `Clone`, so we wrap it in an `Rc<RefCell<..>>` and
// delegate the `View` trait.
// ---------------------------------------------------------------------------
struct SharedView {
    inner: Rc<RefCell<Box<dyn View>>>,
}

impl Clone for SharedView {
    fn clone(&self) -> Self {
        SharedView {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl View for SharedView {
    fn id(&self) -> ViewId {
        self.inner.borrow().id()
    }

    fn view_style(&self) -> Option<Style> {
        View::view_style(&**self.inner.borrow())
    }

    fn view_class(&self) -> Option<StyleClassRef> {
        View::view_class(&**self.inner.borrow())
    }

    fn debug_name(&self) -> Cow<'static, str> {
        View::debug_name(&**self.inner.borrow())
    }

    fn style_pass(&mut self, cx: &mut StyleCx<'_>) {
        self.inner.borrow_mut().style_pass(cx);
    }

    fn event_capture(&mut self, cx: &mut EventCx) -> EventPropagation {
        self.inner.borrow_mut().event_capture(cx)
    }

    fn event(&mut self, cx: &mut EventCx) -> EventPropagation {
        self.inner.borrow_mut().event(cx)
    }

    fn paint(&mut self, cx: &mut PaintCx) {
        self.inner.borrow_mut().paint(cx);
    }

    fn post_paint(&mut self, cx: &mut PaintCx) {
        self.inner.borrow_mut().post_paint(cx);
    }
}

/// Wrap a demo body with a component-name heading.
fn page(title: &str, content: AnyView) -> AnyView {
    Stack::vertical((
        Text::new(title.to_string()).view().style(|s| s.font_size(18.0)),
        content,
    ))
    .style(|s| s.gap(12.0).items_start())
    .into_any()
}

// SVGs used by IconButton / SpeedDial (these render via svg(), not icon names).
const HEART_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z\"/></svg>";

/// Build the full gallery: 98 component pages + 6 root overlays.
///
/// `create_effect`s for the Form and PullToRefresh pages live here (once), so
/// they must be called within an active reactive scope — `main.rs` invokes
/// `build()` from inside the window view factory.
pub fn build() -> Gallery {
    let theme = theme_signal();

    // ---- Root overlays: built ONCE with persistent open signals ----
    // Each floating surface (Dialog / Menu / Snackbar / NavigationDrawer / Sheet
    // / Overlay) is built exactly once here and returned in `overlays`; `main.rs`
    // renders them at the window root. Each reads a persistent `RwSignal` that
    // the corresponding page's trigger button toggles. Wrapping the surface in
    // `dyn_view` (which reads `design_system_signal`) makes it re-skin when the
    // active Design System changes at runtime — without rebuilding the surface on
    // every value change. Previously these were built *inside* the right-pane
    // scroll's page factory, so the window-root `Overlay` scrim collapsed to zero
    // height and they rendered stuck in the right-hand layout ("drawer opens
    // completely messed up"). Building them here at the root fixes that.
    let dialog_open = RwSignal::new(false);
    let menu_open = RwSignal::new(false);
    let snackbar_open = RwSignal::new(false);
    let drawer_open = RwSignal::new(false);
    let drawer_side = RwSignal::new(NavigationDrawerSide::Left);
    let sheet_open = RwSignal::new(false);
    let sheet_side = RwSignal::new(SheetSide::Bottom);
    let overlay_show = RwSignal::new(false);

    // NOTE: overlays are built DIRECTLY here (not wrapped in `dyn_view`). Floom's
    // `Overlay::new()` captures `get_current_view()` as its reparenting root at
    // construction time. `build()` runs inside the window view factory while the
    // current view is the window root, so the overlay content (full-screen
    // scrim with `width_pct/height_pct(100)`) reparents to the full-window root
    // and renders correctly. Wrapping it in `dyn_view` would capture the
    // zero-size `dyn_view` container as the root, collapsing the scrim to zero
    // height — the "overlays never appear" bug. (Trade-off: overlays no longer
    // re-skin on a runtime Design System switch; documented limitation.)
    let dialog_overlay: AnyView = Dialog::new()
        .title("Confirm action")
        .content("This is a Material 3 alert dialog. Use the action buttons, the x, or the scrim to dismiss.")
        .with_open_signal(dialog_open)
        .action("Cancel", || {})
        .action("Confirm", || {})
        .on_close(move || dialog_open.set(false))
        .view()
        .into_any();

    let menu_overlay: AnyView = Menu::new()
        .item("Profile", || {})
        .item("Settings", || {})
        .item("Sign out", || {})
        .with_open_signal(menu_open)
        .view()
        .into_any();

    let snackbar_overlay: AnyView = Snackbar::new()
        .message("Item saved to your library")
        .action_label(Some("Undo".to_string()))
        .with_open_signal(snackbar_open)
        .on_action(move || snackbar_open.set(false))
        .duration_ms(4000)
        .view()
        .into_any();

    let drawer_overlay: AnyView = {
        let open = drawer_open;
        let side = drawer_side;
        #[cfg(feature = "multi")]
        {
            NavigationDrawer::new()
                .header(|| Text::new("Navigation").view())
                .child(|| {
                    List::new()
                        .item("Home", || {})
                        .item("Components", || {})
                        .item("About", || {})
                        .view()
                })
                .side(side.get())
                .with_open_signal(open)
                .view()
                .into_any()
        }
        #[cfg(not(feature = "multi"))]
        {
            NavigationDrawer::new()
                .header(Text::new("Navigation").view())
                .child(
                    List::new()
                        .item("Home", || {})
                        .item("Components", || {})
                        .item("About", || {})
                        .view(),
                )
                .side(side.get())
                .with_open_signal(open)
                .view()
                .into_any()
        }
    };

    let sheet_overlay: AnyView = {
        let open = sheet_open;
        let side = sheet_side;
        #[cfg(feature = "multi")]
        {
            Sheet::new()
                .side(side.get())
                .with_open_signal(open)
                .child(move || {
                    Stack::vertical((
                        Text::new("Sheet").view().into_any(),
                        Text::new("A surface anchored to the chosen screen edge.").view().into_any(),
                        Button::new("Close")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move || open.set(false))
                            .view()
                            .into_any(),
                    ))
                    .style(|s| s.gap(8.0))
                })
                .view()
                .into_any()
        }
        #[cfg(not(feature = "multi"))]
        {
            Sheet::new()
                .side(side.get())
                .with_open_signal(open)
                .child(
                    Stack::vertical((
                        Text::new("Sheet").view().into_any(),
                        Text::new("A surface anchored to the chosen screen edge.").view().into_any(),
                        Button::new("Close")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move || open.set(false))
                            .view()
                            .into_any(),
                    ))
                    .style(|s| s.gap(8.0)),
                )
                .view()
                .into_any()
        }
    };

    let overlay_overlay: AnyView = {
        let show = overlay_show;
        #[cfg(feature = "multi")]
        {
            Overlay::new()
                .with_show_signal(show)
                .on_click(move || show.set(false))
                .child(|| {
                    Stack::vertical((
                        Text::new("Overlay content").view(),
                        Text::new("Click the scrim to dismiss").view(),
                    ))
                    .style(|s| s.gap(8.0).items_center())
                    .into_any()
                })
                .view()
                .into_any()
        }
        #[cfg(not(feature = "multi"))]
        {
            Overlay::new()
                .with_show_signal(show)
                .on_click(move || show.set(false))
                .child(
                    Stack::vertical((
                        Text::new("Overlay content").view(),
                        Text::new("Click the scrim to dismiss").view(),
                    ))
                    .style(|s| s.gap(8.0).items_center())
                    .into_any(),
                )
                .view()
                .into_any()
        }
    };

    let overlays: Vec<AnyView> = vec![
        dialog_overlay,
        menu_overlay,
        snackbar_overlay,
        drawer_overlay,
        sheet_overlay,
        overlay_overlay,
    ];

    // ---- Form page (built once; validity effect lives here) ----
    let name = RwSignal::new(String::new());
    let email = RwSignal::new(String::new());
    let submitted = RwSignal::new(false);

    let name_field = TextField::new()
        .placeholder("Name")
        .on_change({
            let name = name;
            move |v| name.set(v.to_string())
        })
        .view();
    let email_field = TextField::new()
        .placeholder("Email")
        .on_change({
            let email = email;
            move |v| email.set(v.to_string())
        })
        .view();

    let form = Form::new()
        .submit_label("Register")
        .show_submit(true)
        .on_submit(move || submitted.set(true));
    let form_valid = form.valid_signal();

    let vm = ValidationMessages::new();
    let vm_sig = vm.messages_signal();
    let vm_view = vm.view();

    let form_fields = Stack::vertical((name_field.into_any(), email_field.into_any(), vm_view.into_any()))
        .style(|s| s.gap(8.0).width_pct(100.0));
    let form_view = form.child(form_fields).view();

    let success = Label::new("Registered! ✓")
        .style(move |s| {
            let t = theme.get();
            s.font_size(13.0)
                .color(t.colors().primary.to_peniko())
                .apply_if(!submitted.get(), |s| s.hide())
        })
        .into_any();

    let form_page_inner = floom::views::Container::new(
        Stack::vertical((form_view.into_any(), success)).style(|s| s.gap(8.0)),
    )
    .style(|s| s.width(420.0))
    .into_any();
    let form_shared = SharedView {
        inner: Rc::new(RefCell::new(form_page_inner)),
    };

    Effect::new(move |_| {
        let mut errs = Vec::new();
        if name.get().trim().is_empty() {
            errs.push("Name is required.".to_string());
        }
        let email_val = email.get();
        if email_val.trim().is_empty() {
            errs.push("Email is required.".to_string());
        } else if !email_val.contains('@') {
            errs.push("Email must contain '@'.".to_string());
        }
        form_valid.set(errs.is_empty());
        vm_sig.set(errs);
    });

    // ---- PullToRefresh page (built once; completion effect lives here) ----
    let ptr_request = RwSignal::new(false);
    let ptr = PullToRefresh::new()
        .threshold(60.0)
        .on_refresh({
            let ptr_request = ptr_request;
            move || ptr_request.set(true)
        })
        .child(|| {
            Text::new("Pull-to-refresh demo content")
                .view()
                .style(|s| s.font_size(14.0).padding(20.0))
                .into_any()
        });
    let ptr_refreshing = ptr.refreshing_signal();
    let ptr_view = ptr.view().style(|s| s.width(360.0).height(160.0));
    let ptr_status = Label::derived(move || {
        if ptr_refreshing.get() {
            "Refreshing…".to_string()
        } else {
            "Pull down on the box to refresh".to_string()
        }
    })
    .style(|s| s.font_size(14.0))
    .into_any();
    let ptr_page_inner = Stack::vertical((ptr_view.into_any(), ptr_status))
        .style(|s| s.gap(8.0).items_start().width(360.0))
        .into_any();
    let ptr_shared = SharedView {
        inner: Rc::new(RefCell::new(ptr_page_inner)),
    };

    Effect::new(move |_| {
        if ptr_request.get() {
            ptr_refreshing.set(false);
            ptr_request.set(false);
        }
    });

    // ---- Assemble the 98-page master list (canonical component order) ----
    let mut pages: Vec<(String, Page)> = Vec::new();

    // 1. Dialog — pilot showcase (open/close method demo + API + code); toggles window-root overlay.
    pages.push((
        "Dialog".to_string(),
        Box::new({
            let open = dialog_open;
            move || dialog_showcase(open)
        }),
    ));

    // 2. Menu — trigger toggles the window-root overlay built in `build()`.
    pages.push((
        "Menu".to_string(),
        Box::new({
            let open = menu_open;
            move || {
                page(
                    "Menu",
                    Stack::vertical((
                        Button::new("Open menu")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move || open.set(true))
                            .view()
                            .into_any(),
                        Text::new(
                            "A popup menu. Click any item to select and dismiss it. The floating \
                             surface renders at the window root and re-skins on a DS switch.",
                        )
                        .view()
                        .into_any(),
                    ))
                    .style(|s| s.gap(10.0).items_start())
                    .into_any(),
            )
        }
        }),
    ));

    // 3. Button — Vant-style: interactive property controls + static variants.
    pages.push((
        "Button".to_string(),
        Box::new(button_showcase),
    ));

    // 4. Text
    pages.push((
        "Text".to_string(),
        Box::new(text_showcase),
    ));

    // 5. Divider
    pages.push((
        "Divider".to_string(),
        Box::new(divider_showcase),
    ));

    // 6. Checkbox
    pages.push((
        "Checkbox".to_string(),
        Box::new(checkbox_showcase),
    ));

    // 7. Switch — Vant-style: interactive disabled control + static states.
    pages.push((
        "Switch".to_string(),
        Box::new(switch_showcase),
    ));

    // 8. Icon
    pages.push((
        "Icon".to_string(),
        Box::new(icon_showcase),
    ));

    // 9. Slider
    pages.push((
        "Slider".to_string(),
        Box::new(slider_showcase),
    ));

    // 10. RadioGroup
    pages.push((
        "RadioGroup".to_string(),
        Box::new(radio_group_showcase),
    ));

    // 11. Tabs
    pages.push((
        "Tabs".to_string(),
        Box::new(tabs_showcase),
    ));

    // 12. List
    pages.push((
        "List".to_string(),
        Box::new(list_showcase),
    ));

    // 13. Tooltip
    pages.push((
        "Tooltip".to_string(),
        Box::new(tooltip_showcase),
    ));

    // 14. TextField
    pages.push((
        "TextField".to_string(),
        Box::new(text_field_showcase),
    ));

    // 15. Select
    pages.push((
        "Select".to_string(),
        Box::new(select_showcase),
    ));

    // 16. Textarea
    pages.push((
        "Textarea".to_string(),
        Box::new(textarea_showcase),
    ));

    // 17. Chip
    pages.push((
        "Chip".to_string(),
        Box::new(chip_showcase),
    ));

    // 18. Badge
    pages.push((
        "Badge".to_string(),
        Box::new(badge_showcase),
    ));

    // 19. Snackbar — pilot showcase (method demo + API + code); toggles window-root overlay.
    pages.push((
        "Snackbar".to_string(),
        Box::new({
            let open = snackbar_open;
            move || snackbar_showcase(open)
        }),
    ));

    // 20. AppBar
    pages.push((
        "AppBar".to_string(),
        Box::new(app_bar_showcase),
    ));

    // 21. NavigationDrawer — pilot showcase (open/close + side + API + code).
    pages.push((
        "NavigationDrawer".to_string(),
        Box::new({
            let open = drawer_open;
            let side = drawer_side;
            move || navigation_drawer_showcase(open, side)
        }),
    ));

    // 22. BottomNavigation
    pages.push((
        "BottomNavigation".to_string(),
        Box::new(bottom_navigation_showcase),
    ));

    // 23. ProgressLinear
    pages.push((
        "ProgressLinear".to_string(),
        Box::new(progress_linear_showcase),
    ));

    // 24. ProgressCircular
    pages.push((
        "ProgressCircular".to_string(),
        Box::new(progress_circular_showcase),
    ));

    // 25. Card
    pages.push((
        "Card".to_string(),
        Box::new(card_showcase),
    ));

    // 26. Avatar
    pages.push((
        "Avatar".to_string(),
        Box::new(avatar_showcase),
    ));

    // 27. Alert
    pages.push((
        "Alert".to_string(),
        Box::new(alert_showcase),
    ));

    // 28. Rating
    pages.push((
        "Rating".to_string(),
        Box::new(rating_showcase),
    ));

    // 29. Pagination
    pages.push((
        "Pagination".to_string(),
        Box::new(pagination_showcase),
    ));

    // 30. Timeline
    pages.push((
        "Timeline".to_string(),
        Box::new(timeline_showcase),
    ));

    // 31. Sheet — Side control + trigger toggle the window-root overlay.
    pages.push((
        "Sheet".to_string(),
        Box::new({
            let open = sheet_open;
            let side = sheet_side;
            move || {
                page(
                    "Sheet",
                    Stack::vertical((
                        segmented(
                            "Side",
                            vec![
                                ("Bottom", SheetSide::Bottom),
                                ("Top", SheetSide::Top),
                                ("Start", SheetSide::Start),
                                ("End", SheetSide::End),
                            ],
                            side,
                        ),
                        Button::new("Open sheet")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move || open.set(true))
                            .view()
                            .into_any(),
                        Text::new(
                            "A sheet that slides in from the chosen edge with an animation. The sheet \
                             surface renders at the window root and re-skins on a DS switch. Try the \
                             Top / Start / End edges, not just the bottom.",
                        )
                        .view()
                        .into_any(),
                    ))
                    .style(|s| s.gap(10.0).items_start())
                    .into_any(),
                )
            }
        }
        ),
    ));

    // 32. Image
    pages.push((
        "Image".to_string(),
        Box::new(image_showcase),
    ));

    // 33. Banner
    pages.push((
        "Banner".to_string(),
        Box::new(banner_showcase),
    ));

    // 34. SkeletonLoader
    pages.push((
        "SkeletonLoader".to_string(),
        Box::new(skeleton_loader_showcase),
    ));

    // 35. Toolbar
    pages.push((
        "Toolbar".to_string(),
        Box::new(toolbar_showcase),
    ));

    // 36. SystemBar
    pages.push((
        "SystemBar".to_string(),
        Box::new(system_bar_showcase),
    ));

    // 37. Grid
    pages.push((
        "Grid".to_string(),
        Box::new(grid_showcase),
    ));

    // 38. Spacer
    pages.push((
        "Spacer".to_string(),
        Box::new(spacer_showcase),
    ));

    // 39. Footer
    pages.push((
        "Footer".to_string(),
        Box::new(footer_showcase),
    ));

    // 40. Container
    pages.push((
        "Container".to_string(),
        Box::new(container_showcase),
    ));

    // 41. Combobox
    pages.push((
        "Combobox".to_string(),
        Box::new(combobox_showcase),
    ));

    // 42. Autocomplete
    pages.push((
        "Autocomplete".to_string(),
        Box::new(autocomplete_showcase),
    ));

    // 43. DatePicker
    pages.push((
        "DatePicker".to_string(),
        Box::new(date_picker_showcase),
    ));

    // 44. TimePicker
    pages.push((
        "TimePicker".to_string(),
        Box::new(time_picker_showcase),
    ));

    // 45. EmptyState
    pages.push((
        "EmptyState".to_string(),
        Box::new(empty_state_showcase),
    ));

    // 46. DataTable
    pages.push((
        "DataTable".to_string(),
        Box::new(datatable_showcase),
    ));

    // 47. Treeview
    pages.push((
        "Treeview".to_string(),
        Box::new(treeview_showcase),
    ));

    // 48. Calendar
    pages.push((
        "Calendar".to_string(),
        Box::new(calendar_showcase),
    ));

    // 49. Carousel
    pages.push((
        "Carousel".to_string(),
        Box::new(carousel_showcase),
    ));

    // 50. VirtualScroller
    
    pages.push((
        "VirtualScroller".to_string(),
        Box::new(virtual_scroller_showcase),
    ));


    // 51. App
    
    pages.push((
        "App".to_string(),
        Box::new(app_showcase),
    ));


    // 52. Main
    
    pages.push((
        "Main".to_string(),
        Box::new(main_showcase),
    ));


    // 53. Form (pre-built, SharedView)
    pages.push((
        "Form".to_string(),
        Box::new(move || page("Form", form_shared.clone().into_any())),
    ));

    // 54. ValidationMessages
    pages.push((
        "ValidationMessages".to_string(),
        Box::new(validation_messages_showcase),
    ));

    // 55. Stepper
    pages.push((
        "Stepper".to_string(),
        Box::new(stepper_showcase),
    ));

    // 56. Breadcrumbs
    pages.push((
        "Breadcrumbs".to_string(),
        Box::new(breadcrumbs_showcase),
    ));

    // 57. ThemeProvider
    pages.push((
        "ThemeProvider".to_string(),
        Box::new(theme_provider_showcase),
    ));

    // 58. LocaleProvider
    pages.push((
        "LocaleProvider".to_string(),
        Box::new(locale_provider_showcase),
    ));

    // 59. DisplayProvider
    pages.push((
        "DisplayProvider".to_string(),
        Box::new(display_provider_showcase),
    ));

    // 60. DefaultsProvider
    pages.push((
        "DefaultsProvider".to_string(),
        Box::new(defaults_provider_showcase),
    ));

    // 61. IconsProvider
    pages.push((
        "IconsProvider".to_string(),
        Box::new(icons_provider_showcase),
    ));

    // 62. DateAdapterProvider
    pages.push((
        "DateAdapterProvider".to_string(),
        Box::new(date_adapter_provider_showcase),
    ));

    // 63. Window
    pages.push((
        "Window".to_string(),
        Box::new(window_showcase),
    ));

    // 64. SlideGroup
    pages.push((
        "SlideGroup".to_string(),
        Box::new(slide_group_showcase),
    ));

    // 65. ItemGroup
    pages.push((
        "ItemGroup".to_string(),
        Box::new(item_group_showcase),
    ));

    // 66. ColorPicker
    pages.push((
        "ColorPicker".to_string(),
        Box::new(color_picker_showcase),
    ));

    // 67. ButtonGroup
    pages.push((
        "ButtonGroup".to_string(),
        Box::new(button_group_showcase),
    ));

    // 68. FloatingActionButton
    pages.push((
        "FloatingActionButton".to_string(),
        Box::new(fab_showcase),
    ));

    // 69. RangeSlider
    pages.push((
        "RangeSlider".to_string(),
        Box::new(range_slider_showcase),
    ));

    // 70. FileInput
    pages.push((
        "FileInput".to_string(),
        Box::new(file_input_showcase),
    ));

    // 71. OtpInput
    pages.push((
        "OtpInput".to_string(),
        Box::new(otp_input_showcase),
    ));

    // 72. ListItem
    pages.push((
        "ListItem".to_string(),
        Box::new(list_item_showcase),
    ));

    // 73. Table
    pages.push((
        "Table".to_string(),
        Box::new(table_showcase),
    ));

    // 74. DataIterator
    pages.push((
        "DataIterator".to_string(),
        Box::new(data_iterator_showcase),
    ));

    // 75. Sparkline
    pages.push((
        "Sparkline".to_string(),
        Box::new(sparkline_showcase),
    ));

    // 76. Kbd
    pages.push((
        "Kbd".to_string(),
        Box::new(kbd_showcase),
    ));

    // 77. Code
    pages.push((
        "Code".to_string(),
        Box::new(code_showcase),
    ));

    // 78. Radio
    pages.push((
        "Radio".to_string(),
        Box::new(radio_showcase),
    ));

    // 79. Overlay — trigger toggles the window-root overlay built in `build()`.
    pages.push((
        "Overlay".to_string(),
        Box::new({
            let show = overlay_show;
            move || {
                page(
                    "Overlay",
                    Stack::vertical((
                        Button::new("Show overlay")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move || show.set(true))
                            .view()
                            .into_any(),
                        Text::new(
                            "A generic scrim + content surface. The floating surface renders at the \
                             window root and re-skins on a DS switch.",
                        )
                        .view()
                        .into_any(),
                    ))
                    .style(|s| s.gap(10.0).items_start())
                    .into_any(),
                )
            }
        }
        ),
    ));

    // 80. Parallax
    pages.push((
        "Parallax".to_string(),
        Box::new(parallax_showcase),
    ));

    // 81. InfiniteScroller
    
    pages.push((
        "InfiniteScroller".to_string(),
        Box::new(infinite_scroller_showcase),
    ));


    // 82. Hover
    pages.push((
        "Hover".to_string(),
        Box::new(hover_showcase),
    ));

    // 83. FadeTransition
    pages.push((
        "FadeTransition".to_string(),
        Box::new(fade_transition_showcase),
    ));

    // 84. ScaleTransition
    pages.push((
        "ScaleTransition".to_string(),
        Box::new(scale_transition_showcase),
    ));

    // 85. ExpandTransition
    pages.push((
        "ExpandTransition".to_string(),
        Box::new(expand_transition_showcase),
    ));

    // 86. SlideTransition
    pages.push((
        "SlideTransition".to_string(),
        Box::new(slide_transition_showcase),
    ));

    // 87. IconButton
    pages.push((
        "IconButton".to_string(),
        Box::new(icon_button_showcase),
    ));

    // 88. ExpansionPanels
    pages.push((
        "ExpansionPanels".to_string(),
        Box::new(expansion_panels_showcase),
    ));

    // 89. SpeedDial
    pages.push((
        "SpeedDial".to_string(),
        Box::new(speed_dial_showcase),
    ));

    // 90. ChipGroup
    pages.push((
        "ChipGroup".to_string(),
        Box::new(chip_group_showcase),
    ));

    // 91. ListGroup
    pages.push((
        "ListGroup".to_string(),
        Box::new(list_group_showcase),
    ));

    // 92. Counter
    pages.push((
        "Counter".to_string(),
        Box::new(counter_showcase),
    ));

    // 93. Responsive
    pages.push((
        "Responsive".to_string(),
        Box::new(responsive_showcase),
    ));

    // 94. CheckboxBtn
    pages.push((
        "CheckboxBtn".to_string(),
        Box::new(checkbox_btn_showcase),
    ));

    // 95. ListSubheader
    pages.push((
        "ListSubheader".to_string(),
        Box::new(list_subheader_showcase),
    ));

    // 96. NumberInput
    pages.push((
        "NumberInput".to_string(),
        Box::new(number_input_showcase),
    ));

    // 97. PullToRefresh (pre-built, SharedView)
    pages.push((
        "PullToRefresh".to_string(),
        Box::new(move || page("PullToRefresh", ptr_shared.clone().into_any())),
    ));

    // 98. ConfirmEdit
    pages.push((
        "ConfirmEdit".to_string(),
        Box::new(confirm_edit_showcase),
    ));

    Gallery { pages, overlays }
}
