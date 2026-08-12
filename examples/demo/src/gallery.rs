//! Component gallery data for the floem-ui demo.
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
use std::collections::HashMap;
use std::rc::Rc;

use floem::IntoView;
use floem::View;
use floem::event::{EventCx, EventPropagation};
use floem::paint::PaintCx;
use floem::reactive::{create_effect, RwSignal, SignalGet, SignalUpdate};
use floem::style::{Style, StyleClassRef, StyleCx};
use floem::view::{AnyView, ViewId};
use floem::views::{Decorators, h_stack, label, text, v_stack, v_stack_from_iter};
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::with_alpha;
use floem_elements::button::ButtonVariant;
use floem_elements::prelude::*;
use floem_widget_runtime::theme_signal;

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
    v_stack((
        Text::new(title.to_string()).view().style(|s| s.font_size(18.0)),
        content,
    ))
    .style(|s| s.gap(12.0).items_start())
    .into_any()
}

// SVGs used by IconButton / SpeedDial (these render via svg(), not icon names).
const HEART_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M12 21.35l-1.45-1.32C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54L12 21.35z\"/></svg>";
const DIAL_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z\"/></svg>";
const EDIT_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z\"/></svg>";
const SHARE_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path d=\"M18 16.08c-.76 0-1.44.3-1.96.77L8.91 12.7c.05-.23.09-.46.09-.7s-.04-.47-.09-.7l7.05-4.11c.54.5 1.25.81 2.04.81a3 3 0 1 0-3-3c0 .24.04.47.09.7L8.04 9.81C7.5 9.31 6.79 9 6 9a3 3 0 0 0 0 6c.79 0 1.5-.31 2.04-.81l7.12 4.16c-.05.21-.08.43-.08.65a2.92 2.92 0 1 0 2.92-2.92z\"/></svg>";

/// Build the full gallery: 98 component pages + 6 root overlays.
///
/// `create_effect`s for the Form and PullToRefresh pages live here (once), so
/// they must be called within an active reactive scope — `main.rs` invokes
/// `build()` from inside the window view factory.
pub fn build() -> Gallery {
    let theme = theme_signal();

    // ---- Overlay open/show signals (Copy) ----
    let dialog = Dialog::new()
        .title("Confirm action")
        .content("This is a Material 3 alert dialog. Click the ✕ or the scrim to dismiss.");
    let dialog_open = dialog.open_signal();
    let dialog_view = dialog.view();

    let menu = Menu::new()
        .item("Profile", || {})
        .item("Settings", || {})
        .item("Sign out", || {});
    let menu_open = menu.open_signal();
    let menu_view = menu.view();

    let snackbar = Snackbar::new()
        .message("Item saved to your library")
        .action_label(Some("Undo".to_string()))
        .on_action(|| {})
        .duration_ms(4000);
    let snackbar_open = snackbar.open_signal();
    let snackbar_view = snackbar.view();

    let drawer = NavigationDrawer::new()
        .header(Text::new("Navigation").view())
        .child(
            List::new()
                .item("Home", || {})
                .item("Components", || {})
                .item("About", || {})
                .view(),
        );
    let drawer_open = drawer.open_signal();
    let drawer_view = drawer.view();

    let sheet = Sheet::new().side(SheetSide::Bottom).open(false);
    let sheet_open = sheet.open_signal();
    let sheet = sheet.child(
        v_stack((
            Text::new("Bottom sheet").view().into_any(),
            Text::new("A surface anchored to the bottom edge of the screen.")
                .view()
                .into_any(),
            Button::new("Close")
                .variant(ButtonVariant::Secondary)
                .on_click(move || sheet_open.set(false))
                .view()
                .into_any(),
        ))
        .style(|s| s.gap(8.0)),
    );
    let sheet_view = sheet.view();

    let overlay = Overlay::new().child(
        v_stack((
            Text::new("Overlay content").view(),
            Text::new("Click the scrim to dismiss").view(),
        ))
        .style(|s| s.gap(8.0).items_center())
        .into_any(),
    );
    let overlay_show = overlay.show_signal();
    let overlay_view = overlay.view();

    let mut overlays: Vec<AnyView> = Vec::new();
    overlays.push(dialog_view.into_any());
    overlays.push(menu_view.into_any());
    overlays.push(snackbar_view.into_any());
    overlays.push(drawer_view.into_any());
    overlays.push(sheet_view.into_any());
    overlays.push(overlay_view.into_any());

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

    let form_fields = v_stack((name_field.into_any(), email_field.into_any(), vm_view.into_any()))
        .style(|s| s.gap(8.0).width_pct(100.0));
    let form_view = form.child(form_fields).view();

    let success = text("Registered! ✓")
        .style(move |s| {
            let t = theme.get();
            s.font_size(13.0)
                .color(t.colors().primary.to_peniko())
                .apply_if(!submitted.get(), |s| s.hide())
        })
        .into_any();

    let form_page_inner = floem::views::Container::new(
        v_stack((form_view.into_any(), success)).style(|s| s.gap(8.0)),
    )
    .style(|s| s.width(420.0))
    .into_any();
    let form_shared = SharedView {
        inner: Rc::new(RefCell::new(form_page_inner)),
    };

    create_effect(move |_| {
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
    let ptr_status = label(move || {
        if ptr_refreshing.get() {
            "Refreshing…".to_string()
        } else {
            "Pull down on the box to refresh".to_string()
        }
    })
    .style(|s| s.font_size(14.0))
    .into_any();
    let ptr_page_inner = v_stack((ptr_view.into_any(), ptr_status))
        .style(|s| s.gap(8.0).items_start().width(360.0))
        .into_any();
    let ptr_shared = SharedView {
        inner: Rc::new(RefCell::new(ptr_page_inner)),
    };

    create_effect(move |_| {
        if ptr_request.get() {
            ptr_refreshing.set(false);
            ptr_request.set(false);
        }
    });

    // ---- Assemble the 98-page master list (canonical component order) ----
    let mut pages: Vec<(String, Page)> = Vec::new();

    // 1. Dialog (overlay surface lives in `overlays`)
    pages.push((
        "Dialog".to_string(),
        Box::new(move || {
            page(
                "Dialog",
                v_stack((
                    Button::new("Open dialog")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || dialog_open.set(true))
                        .view()
                        .into_any(),
                    Text::new(
                        "A Material 3 alert dialog. Click the button, then the ✕ or the scrim \
                         to dismiss. The floating surface is rendered at the window root.",
                    )
                    .view(),
                ))
                .style(|s| s.gap(10.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 2. Menu (overlay surface lives in `overlays`)
    pages.push((
        "Menu".to_string(),
        Box::new(move || {
            page(
                "Menu",
                v_stack((
                    Button::new("Open menu")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || menu_open.set(true))
                        .view()
                        .into_any(),
                    Text::new(
                        "A popup menu anchored to its trigger. The floating surface is rendered \
                         at the window root.",
                    )
                    .view(),
                ))
                .style(|s| s.gap(10.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 3. Button
    pages.push((
        "Button".to_string(),
        Box::new(|| {
            page(
                "Button",
                h_stack((
                    Button::new("Filled").variant(ButtonVariant::Primary).view().into_any(),
                    Button::new("Tonal").variant(ButtonVariant::Secondary).view().into_any(),
                    Button::new("Danger").variant(ButtonVariant::Danger).view().into_any(),
                    Button::new("Text").variant(ButtonVariant::Text).view().into_any(),
                    Button::new("Ghost").variant(ButtonVariant::Ghost).view().into_any(),
                    Button::new("Disabled").disabled(true).view().into_any(),
                ))
                .style(|s| s.gap(12.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 4. Text
    pages.push((
        "Text".to_string(),
        Box::new(|| {
            page(
                "Text",
                v_stack((
                    Text::new("Display text — heading scale").view().style(|s| s.font_size(24.0)),
                    Text::new(
                        "Body text wraps and uses the theme's on-surface colour by default.",
                    )
                    .view()
                    .style(|s| s.font_size(14.0)),
                    Text::new("A muted helper line.").view().style(|s| s.font_size(12.0)),
                ))
                .style(|s| s.gap(6.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 5. Divider
    pages.push((
        "Divider".to_string(),
        Box::new(|| {
            page(
                "Divider",
                v_stack((
                    Text::new("Above the divider").view(),
                    Divider::new().view(),
                    Text::new("Below the divider").view(),
                ))
                .style(|s| s.gap(6.0).width(360.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 6. Checkbox
    pages.push((
        "Checkbox".to_string(),
        Box::new(|| {
            page(
                "Checkbox",
                v_stack((
                    Checkbox::new("Enable notifications").view().into_any(),
                    Checkbox::new("Pre-checked").checked(true).view().into_any(),
                    Checkbox::new("Disabled").disabled(true).view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 7. Switch
    pages.push((
        "Switch".to_string(),
        Box::new(|| {
            page(
                "Switch",
                h_stack((
                    Switch::new().view().into_any(),
                    Switch::new().checked(true).view().into_any(),
                    Text::new("Switches").view(),
                ))
                .style(|s| s.gap(16.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 8. Icon
    pages.push((
        "Icon".to_string(),
        Box::new(|| {
            page(
                "Icon",
                h_stack((
                    Icon::new("home").view().into_any(),
                    Icon::new("favorite").view().into_any(),
                    Icon::new("settings").view().into_any(),
                    Icon::new("delete").view().into_any(),
                    Icon::new("add").view().into_any(),
                    Icon::new("search").view().into_any(),
                ))
                .style(|s| s.gap(16.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 9. Slider
    pages.push((
        "Slider".to_string(),
        Box::new(|| {
            let slider_val = RwSignal::new(40.0f32);
            let slider_label = label(move || format!("Slider: {:.0}", slider_val.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let slider_view = Slider::new()
                .value(40.0)
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .on_change(move |v| slider_val.set(v))
                .view()
                .into_any();
            page(
                "Slider",
                v_stack((slider_view, slider_label))
                    .style(|s| s.gap(8.0).width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 10. RadioGroup
    pages.push((
        "RadioGroup".to_string(),
        Box::new(|| {
            let radio_sel = RwSignal::new(0usize);
            let radio_view = RadioGroup::new()
                .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
                .selected(Some(0))
                .on_change(move |i| radio_sel.set(i))
                .view()
                .into_any();
            let radio_label = label(move || format!("Radio selected: {:?}", radio_sel.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            page(
                "RadioGroup",
                v_stack((radio_view, radio_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 11. Tabs
    pages.push((
        "Tabs".to_string(),
        Box::new(|| {
            let tab_sel = RwSignal::new(0usize);
            let tabs_view = Tabs::new()
                .tabs(vec!["One".into(), "Two".into(), "Three".into()])
                .selected(0)
                .on_change(move |i| tab_sel.set(i))
                .view()
                .into_any();
            let tab_label = label(move || format!("Tab selected: {}", tab_sel.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            page(
                "Tabs",
                v_stack((tabs_view, tab_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 12. List
    pages.push((
        "List".to_string(),
        Box::new(|| {
            page(
                "List",
                List::new()
                    .item("Profile", || {})
                    .item("Billing", || {})
                    .item("Logout", || {})
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 13. Tooltip
    pages.push((
        "Tooltip".to_string(),
        Box::new(|| {
            page(
                "Tooltip",
                Tooltip::new()
                    .text("Tooltip shown on hover")
                    .child(Button::new("Hover me").view())
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 14. TextField
    pages.push((
        "TextField".to_string(),
        Box::new(|| {
            let typed = RwSignal::new(String::new());
            let tf = TextField::new()
                .placeholder("Type something…")
                .on_change(move |v| typed.set(v.to_string()))
                .view()
                .into_any();
            let typed_label = label(move || format!("You typed: {}", typed.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            page(
                "TextField",
                v_stack((tf, typed_label))
                    .style(|s| s.gap(8.0).width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 15. Select
    pages.push((
        "Select".to_string(),
        Box::new(|| {
            let select_sel = RwSignal::new(Some(0usize));
            let select_label = label(move || format!("Select: {:?}", select_sel.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let select_view = Select::new()
                .options(vec!["Red".into(), "Green".into(), "Blue".into()])
                .selected(Some(0))
                .placeholder("Pick a color")
                .on_change(move |i| select_sel.set(i))
                .view()
                .into_any();
            page(
                "Select",
                v_stack((select_view, select_label))
                    .style(|s| s.gap(8.0).width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 16. Textarea
    pages.push((
        "Textarea".to_string(),
        Box::new(|| {
            let text_val = RwSignal::new(String::new());
            let textarea_view = Textarea::new()
                .value("Edit me…".to_string())
                .rows(4)
                .on_change(move |v| text_val.set(v))
                .view()
                .into_any();
            let textarea_label = label(move || format!("Textarea: {} chars", text_val.get().len()))
                .style(|s| s.font_size(14.0))
                .into_any();
            page(
                "Textarea",
                v_stack((textarea_view, textarea_label))
                    .style(|s| s.gap(8.0).width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 17. Chip
    pages.push((
        "Chip".to_string(),
        Box::new(|| {
            page(
                "Chip",
                h_stack((
                    Chip::new().label("Apple".to_string()).selected(true).on_click(|| {}).view().into_any(),
                    Chip::new().label("Banana".to_string()).on_click(|| {}).view().into_any(),
                    Chip::new().label("Cherry".to_string()).disabled(true).on_click(|| {}).view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 18. Badge
    pages.push((
        "Badge".to_string(),
        Box::new(|| {
            page(
                "Badge",
                h_stack((
                    Badge::new().content("3").child(Icon::new("notifications").view()).view().into_any(),
                    Badge::new().dot(true).child(Button::new("Dot").view()).view().into_any(),
                ))
                .style(|s| s.gap(16.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 19. Snackbar (overlay surface lives in `overlays`)
    pages.push((
        "Snackbar".to_string(),
        Box::new(move || {
            page(
                "Snackbar",
                v_stack((
                    Button::new("Show snackbar")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || snackbar_open.set(true))
                        .view()
                        .into_any(),
                    Text::new(
                        "A transient message with an optional action. It auto-dismisses after a \
                         duration and is rendered at the window root.",
                    )
                    .view(),
                ))
                .style(|s| s.gap(10.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 20. AppBar
    pages.push((
        "AppBar".to_string(),
        Box::new(|| {
            page(
                "AppBar",
                AppBar::new()
                    .title("floem-ui Gallery")
                    .leading(Icon::new("home").view())
                    .actions(vec![
                        Icon::new("search").view().into_any(),
                        Icon::new("add").view().into_any(),
                    ])
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 21. NavigationDrawer (overlay surface lives in `overlays`)
    pages.push((
        "NavigationDrawer".to_string(),
        Box::new(move || {
            page(
                "NavigationDrawer",
                v_stack((
                    Button::new("Open drawer")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || drawer_open.set(true))
                        .view()
                        .into_any(),
                    Text::new(
                        "A navigation surface that slides in from the edge. The floating surface \
                         is rendered at the window root.",
                    )
                    .view(),
                ))
                .style(|s| s.gap(10.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 22. BottomNavigation
    pages.push((
        "BottomNavigation".to_string(),
        Box::new(|| {
            let nav_sel = RwSignal::new(0usize);
            let bottom_nav_view = BottomNavigation::new()
                .labels(vec!["Home".into(), "Search".into(), "Profile".into()])
                .selected(0)
                .on_change(move |i| nav_sel.set(i))
                .view()
                .into_any();
            let nav_label = label(move || format!("Bottom nav selected: {}", nav_sel.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            page(
                "BottomNavigation",
                v_stack((bottom_nav_view, nav_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 23. ProgressLinear
    pages.push((
        "ProgressLinear".to_string(),
        Box::new(|| {
            page(
                "ProgressLinear",
                v_stack((
                    ProgressLinear::new().value(65.0).view().into_any(),
                    ProgressLinear::new().indeterminate(true).view().into_any(),
                ))
                .style(|s| s.gap(8.0).width(360.0))
                .into_any(),
            )
        }),
    ));

    // 24. ProgressCircular
    pages.push((
        "ProgressCircular".to_string(),
        Box::new(|| {
            page(
                "ProgressCircular",
                h_stack((
                    ProgressCircular::new().value(40.0).view().into_any(),
                    ProgressCircular::new().indeterminate(true).view().into_any(),
                ))
                .style(|s| s.gap(24.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 25. Card
    pages.push((
        "Card".to_string(),
        Box::new(|| {
            page(
                "Card",
                Card::new()
                    .elevation(2)
                    .child(
                        v_stack((
                            Text::new("Card title").view().into_any(),
                            Text::new("Cards group related content on a Material surface.")
                                .view()
                                .into_any(),
                        ))
                        .style(|s| s.gap(8.0)),
                    )
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 26. Avatar
    pages.push((
        "Avatar".to_string(),
        Box::new(|| {
            page(
                "Avatar",
                h_stack((
                    Avatar::new().content("AB".to_string()).size(48.0).view().into_any(),
                    Avatar::new().content("CD".to_string()).size(40.0).view().into_any(),
                    Avatar::new().content("EF".to_string()).size(32.0).view().into_any(),
                ))
                .style(|s| s.gap(16.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 27. Alert
    pages.push((
        "Alert".to_string(),
        Box::new(|| {
            page(
                "Alert",
                v_stack((
                    Alert::new().variant(AlertVariant::Info).title("Heads up").message("A new version is available.").view().into_any(),
                    Alert::new().variant(AlertVariant::Success).title("Saved").message("Your changes were stored.").view().into_any(),
                    Alert::new().variant(AlertVariant::Warning).title("Careful").message("This action is not reversible.").view().into_any(),
                    Alert::new().variant(AlertVariant::Error).title("Error").message("Something went wrong.").dismissible(true).on_dismiss(|| {}).view().into_any(),
                ))
                .style(|s| s.gap(10.0).width(420.0))
                .into_any(),
            )
        }),
    ));

    // 28. Rating
    pages.push((
        "Rating".to_string(),
        Box::new(|| {
            let rating_val = RwSignal::new(3.0f32);
            let rating_label = label(move || format!("Rating: {:.0}", rating_val.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let rating_view = Rating::new()
                .value(3.0)
                .max(5)
                .on_change(move |v| rating_val.set(v))
                .view()
                .into_any();
            page(
                "Rating",
                v_stack((rating_view, rating_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 29. Pagination
    pages.push((
        "Pagination".to_string(),
        Box::new(|| {
            let page_sel = RwSignal::new(1usize);
            let page_label = label(move || format!("Page: {}", page_sel.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let pagination_view = Pagination::new()
                .total(7)
                .current(1)
                .on_change(move |p| page_sel.set(p))
                .view()
                .into_any();
            page(
                "Pagination",
                v_stack((pagination_view, page_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 30. Timeline
    pages.push((
        "Timeline".to_string(),
        Box::new(|| {
            page(
                "Timeline",
                Timeline::new()
                    .item("Order placed", "We received your order.")
                    .item("Packed", "Items were packed at the warehouse.")
                    .item("Shipped", "Your package is on the way.")
                    .item("Delivered", "Arrived at your address.")
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 31. Sheet (overlay surface lives in `overlays`)
    pages.push((
        "Sheet".to_string(),
        Box::new(move || {
            page(
                "Sheet",
                v_stack((
                    Button::new("Open sheet")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || sheet_open.set(true))
                        .view()
                        .into_any(),
                    Text::new(
                        "A bottom sheet anchored to the screen edge. The sheet surface renders at \
                         the window root.",
                    )
                    .view(),
                ))
                .style(|s| s.gap(10.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 32. Image
    pages.push((
        "Image".to_string(),
        Box::new(|| {
            page(
                "Image",
                Image::new()
                    .src("preview.png")
                    .alt("Image preview")
                    .width(160.0)
                    .height(100.0)
                    .rounded(true)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 33. Banner
    pages.push((
        "Banner".to_string(),
        Box::new(|| {
            page(
                "Banner",
                v_stack((
                    Banner::new().variant(BannerVariant::Info).message("No connection — changes will sync later.").action("Retry").on_action(|| {}).view().into_any(),
                    Banner::new().variant(BannerVariant::Error).message("Failed to load resource.").view().into_any(),
                    Banner::new().variant(BannerVariant::Success).icon("check_circle").message("Upload complete.").view().into_any(),
                ))
                .style(|s| s.gap(8.0).width(420.0))
                .into_any(),
            )
        }),
    ));

    // 34. SkeletonLoader
    pages.push((
        "SkeletonLoader".to_string(),
        Box::new(|| {
            page(
                "SkeletonLoader",
                h_stack((
                    SkeletonLoader::new().skeleton_type(SkeletonType::Text).lines(3).view().into_any(),
                    SkeletonLoader::new().skeleton_type(SkeletonType::Circle).width(56.0).height(56.0).view().into_any(),
                    SkeletonLoader::new().skeleton_type(SkeletonType::Card).view().into_any(),
                    SkeletonLoader::new().skeleton_type(SkeletonType::Avatar).view().into_any(),
                ))
                .style(|s| s.gap(16.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 35. Toolbar
    pages.push((
        "Toolbar".to_string(),
        Box::new(|| {
            page(
                "Toolbar",
                Toolbar::new()
                    .title("Editor")
                    .leading("menu")
                    .actions(
                        h_stack((
                            Icon::new("search").view().into_any(),
                            Icon::new("more_vert").view().into_any(),
                        ))
                        .style(|s| s.gap(8.0).items_center()),
                    )
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 36. SystemBar
    pages.push((
        "SystemBar".to_string(),
        Box::new(|| {
            page(
                "SystemBar",
                SystemBar::new()
                    .dark(true)
                    .child(
                        h_stack((
                            Text::new("Wi-Fi").view().style(|s| s.font_size(12.0)).into_any(),
                            Spacer::new().grow(1.0).view().into_any(),
                            Text::new("100%").view().style(|s| s.font_size(12.0)).into_any(),
                        ))
                        .style(|s| s.gap(8.0).width(240.0)),
                    )
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 37. Grid
    pages.push((
        "Grid".to_string(),
        Box::new(|| {
            let theme = theme_signal();
            let cell = |_i: usize| {
                floem::views::Container::new(floem::views::empty()).style(move |s| {
                    s.height(40.0)
                        .background(theme.get().colors().primary_container.to_peniko())
                })
            };
            page(
                "Grid",
                Grid::new()
                    .cols(3)
                    .gap(8.0)
                    .row_gap(8.0)
                    .child(cell(0))
                    .child(cell(1))
                    .child(cell(2))
                    .child(cell(3))
                    .child(cell(4))
                    .child(cell(5))
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 38. Spacer
    pages.push((
        "Spacer".to_string(),
        Box::new(|| {
            page(
                "Spacer",
                floem::views::Container::new(
                    h_stack((
                        Text::new("Left").view().into_any(),
                        Spacer::new().grow(1.0).view().into_any(),
                        Text::new("Right").view().into_any(),
                    ))
                    .style(|s| s.gap(8.0)),
                )
                .style(|s| s.width(320.0).border(1.0))
                .into_any(),
            )
        }),
    ));

    // 39. Footer
    pages.push((
        "Footer".to_string(),
        Box::new(|| {
            page(
                "Footer",
                Footer::new()
                    .bordered(true)
                    .child(
                        h_stack((
                            Text::new("© 2026 Acme").view().style(|s| s.font_size(12.0)).into_any(),
                            Spacer::new().grow(1.0).view().into_any(),
                            Text::new("Privacy").view().style(|s| s.font_size(12.0)).into_any(),
                            Text::new("Terms").view().style(|s| s.font_size(12.0)).into_any(),
                        ))
                        .style(|s| s.gap(12.0).width_pct(100.0)),
                    )
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 40. Container
    pages.push((
        "Container".to_string(),
        Box::new(|| {
            page(
                "Container",
                Container::new()
                    .child(
                        v_stack((
                            Text::new("Inside a centered Container").view().into_any(),
                            Text::new("Content is capped to a max width and centered.")
                                .view()
                                .into_any(),
                        ))
                        .style(|s| s.gap(4.0)),
                    )
                    .max_width(420.0)
                    .padding(16.0)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 41. Combobox
    pages.push((
        "Combobox".to_string(),
        Box::new(|| {
            let combo_sel = RwSignal::new(Some(1usize));
            let combo_label = label(move || format!("Combobox: {:?}", combo_sel.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let combobox_view = Combobox::new()
                .options(vec![
                    "Apple".to_string(),
                    "Banana".to_string(),
                    "Cherry".to_string(),
                    "Date".to_string(),
                    "Elderberry".to_string(),
                ])
                .selected(combo_sel.get())
                .placeholder("Pick a fruit…")
                .on_change(move |i| combo_sel.set(i))
                .view()
                .into_any();
            page(
                "Combobox",
                v_stack((combobox_view, combo_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 42. Autocomplete
    pages.push((
        "Autocomplete".to_string(),
        Box::new(|| {
            let autocomplete_val = RwSignal::new(String::new());
            let autocomplete_label =
                label(move || format!("Autocomplete: {:?}", autocomplete_val.get()))
                    .style(|s| s.font_size(14.0))
                    .into_any();
            let autocomplete_view = Autocomplete::new()
                .options(vec![
                    "Rust".to_string(),
                    "Ruby".to_string(),
                    "Racket".to_string(),
                    "R".to_string(),
                    "Python".to_string(),
                ])
                .selected(None)
                .placeholder("Search a language…")
                .on_change(move |s: Option<String>| autocomplete_val.set(s.unwrap_or_default()))
                .view()
                .into_any();
            page(
                "Autocomplete",
                v_stack((autocomplete_view, autocomplete_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 43. DatePicker
    pages.push((
        "DatePicker".to_string(),
        Box::new(|| {
            let date_val = RwSignal::new(DateValue { year: 2026, month: 8, day: 10 });
            let date_label = label(move || {
                let d = date_val.get();
                format!("Date: {:04}-{:02}-{:02}", d.year, d.month, d.day)
            })
            .style(|s| s.font_size(14.0))
            .into_any();
            let datepicker_view = DatePicker::new()
                .value(date_val.get())
                .placeholder("Select a date")
                .on_change(move |d| date_val.set(d))
                .view()
                .into_any();
            page(
                "DatePicker",
                v_stack((datepicker_view, date_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 44. TimePicker
    pages.push((
        "TimePicker".to_string(),
        Box::new(|| {
            let time_val = RwSignal::new(TimeValue { hour: 9, minute: 30 });
            let time_label = label(move || {
                let t = time_val.get();
                format!("Time: {:02}:{:02}", t.hour, t.minute)
            })
            .style(|s| s.font_size(14.0))
            .into_any();
            let timepicker_view = TimePicker::new()
                .value(time_val.get())
                .placeholder("Select a time")
                .on_change(move |t| time_val.set(t))
                .view()
                .into_any();
            page(
                "TimePicker",
                v_stack((timepicker_view, time_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 45. EmptyState
    pages.push((
        "EmptyState".to_string(),
        Box::new(|| {
            page(
                "EmptyState",
                EmptyState::new()
                    .icon("inbox")
                    .title("No items yet")
                    .description("When you create your first item, it will show up here.")
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 46. DataTable
    pages.push((
        "DataTable".to_string(),
        Box::new(|| {
            let dt_row = RwSignal::new(0usize);
            let dt_label = label(move || format!("Selected row: {}", dt_row.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let dt_columns = vec![
                DataTableColumn { title: "Name".into(), align: ColumnAlign::Start },
                DataTableColumn { title: "Role".into(), align: ColumnAlign::Start },
                DataTableColumn { title: "Status".into(), align: ColumnAlign::End },
            ];
            let dt_rows = vec![
                vec!["Alice".to_string(), "Admin".to_string(), "Active".to_string()],
                vec!["Bob".to_string(), "Editor".to_string(), "Active".to_string()],
                vec!["Carol".to_string(), "Viewer".to_string(), "Invited".to_string()],
                vec!["Dave".to_string(), "Guest".to_string(), "Revoked".to_string()],
            ];
            let dt_view = DataTable::new()
                .columns(dt_columns)
                .rows(dt_rows)
                .striped(true)
                .on_row_click(move |i| dt_row.set(i))
                .view()
                .into_any();
            page(
                "DataTable",
                v_stack((dt_view, dt_label))
                    .style(|s| s.gap(8.0).items_start().width(420.0))
                    .into_any(),
            )
        }),
    ));

    // 47. Treeview
    pages.push((
        "Treeview".to_string(),
        Box::new(|| {
            let tv_label = RwSignal::new(String::new());
            let tv_text = label(move || {
                let l = tv_label.get();
                if l.is_empty() {
                    "Selected node: —".to_string()
                } else {
                    format!("Selected node: {}", l)
                }
            })
            .style(|s| s.font_size(14.0))
            .into_any();
            let tree_nodes = vec![
                TreeviewNode {
                    label: "Documents".into(),
                    icon: Some("folder".into()),
                    expanded: true,
                    children: vec![
                        TreeviewNode {
                            label: "Work".into(),
                            icon: Some("folder".into()),
                            expanded: false,
                            children: vec![TreeviewNode {
                                label: "Report.pdf".into(),
                                icon: Some("description".into()),
                                expanded: false,
                                children: vec![],
                            }],
                        },
                        TreeviewNode {
                            label: "Personal".into(),
                            icon: Some("folder".into()),
                            expanded: false,
                            children: vec![],
                        },
                    ],
                },
                TreeviewNode {
                    label: "Pictures".into(),
                    icon: Some("photo_library".into()),
                    expanded: false,
                    children: vec![],
                },
            ];
            let tree_view = Treeview::new()
                .nodes(tree_nodes)
                .on_node_click(move |lbl| tv_label.set(lbl))
                .view()
                .into_any();
            page(
                "Treeview",
                v_stack((tree_view, tv_text))
                    .style(|s| s.gap(8.0).items_start().width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 48. Calendar
    pages.push((
        "Calendar".to_string(),
        Box::new(|| {
            let cal_val = RwSignal::new(CalendarDate { year: 2026, month: 8, day: 10 });
            let cal_label = label(move || {
                let d = cal_val.get();
                format!("Date: {:04}-{:02}-{:02}", d.year, d.month, d.day)
            })
            .style(|s| s.font_size(14.0))
            .into_any();
            let cal_view = Calendar::new()
                .value(cal_val.get())
                .show_week_numbers(true)
                .on_change(move |d| cal_val.set(d))
                .view()
                .into_any();
            page(
                "Calendar",
                v_stack((cal_view, cal_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 49. Carousel
    pages.push((
        "Carousel".to_string(),
        Box::new(|| {
            let theme = theme_signal();
            let slides: Vec<AnyView> = (0..3)
                .map(|i| {
                    let colors = [
                        theme.get().colors().primary,
                        theme.get().colors().secondary,
                        theme.get().colors().tertiary,
                    ];
                    let c = colors[i % colors.len()];
                    floem::views::Container::new(
                        text(format!("Slide {}", i + 1))
                            .style(move |s| s.font_size(22.0).color(theme.get().colors().on_primary.to_peniko())),
                    )
                    .style(move |s| {
                        s.width_pct(100.0)
                            .height_pct(100.0)
                            .items_center()
                            .justify_center()
                            .background(c.to_peniko())
                    })
                    .into_any()
                })
                .collect();
            page(
                "Carousel",
                Carousel::new()
                    .slides(slides)
                    .height(180.0)
                    .show_arrows(true)
                    .show_dots(true)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 50. VirtualScroller
    pages.push((
        "VirtualScroller".to_string(),
        Box::new(|| {
            let vscroll_items: Vec<Box<dyn Fn() -> Box<dyn View>>> = (0..50)
                .map(|i| {
                    Box::new(move || -> Box<dyn View> {
                        Box::new(
                            text(format!("Virtual row item {}", i))
                                .style(move |s| {
                                    s.padding_horiz(16.0)
                                        .font_size(14.0)
                                        .color(theme_signal().get().colors().on_surface.to_peniko())
                                })
                                .into_any(),
                        )
                    }) as Box<dyn Fn() -> Box<dyn View>>
                })
                .collect();
            page(
                "VirtualScroller",
                VirtualScroller::new()
                    .items(vscroll_items)
                    .height(260.0)
                    .on_reach_end(|| {})
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 51. App
    pages.push((
        "App".to_string(),
        Box::new(|| {
            let shell_drawer = NavigationDrawer::new()
                .header(Text::new("Navigation").view())
                .child(
                    List::new()
                        .item("Home", || {})
                        .item("Components", || {})
                        .item("About", || {})
                        .view(),
                );
            let shell_drawer_open = shell_drawer.open_signal();
            let shell_drawer_view = shell_drawer.view();

            let app_bar_overlay = floem::views::Container::new(
                AppBar::new()
                    .title("App + Main shell")
                    .leading(Icon::new("menu").view())
                    .actions(vec![Icon::new("search").view().into_any()])
                    .view()
                    .into_any(),
            )
            .style(|s| s.absolute().inset_top(0.0).inset_left(0.0).width_full());

            let footer_overlay = floem::views::Container::new(
                Footer::new()
                    .bordered(true)
                    .child(
                        h_stack((
                            Text::new("© 2026 Acme").view().style(|s| s.font_size(12.0)).into_any(),
                            Spacer::new().grow(1.0).view().into_any(),
                            Text::new("Docs").view().style(|s| s.font_size(12.0)).into_any(),
                        ))
                        .style(|s| s.gap(12.0).width_pct(100.0)),
                    )
                    .view()
                    .into_any(),
            )
            .style(|s| s.absolute().inset_bottom(0.0).inset_left(0.0).width_full());

            let main_content = v_stack((
                Text::new("Main content region").view().into_any(),
                Text::new(
                    "Main reads the process-wide layout signal and insets its content from the \
                     AppBar, Footer, and an open NavigationDrawer.",
                )
                .view()
                .style(|s| s.font_size(13.0))
                .into_any(),
                Spacer::new().grow(1.0).view().into_any(),
                Button::new("Toggle drawer")
                    .variant(ButtonVariant::Secondary)
                    .on_click(move || shell_drawer_open.set(!shell_drawer_open.get()))
                    .view()
                    .into_any(),
            ))
            .style(|s| s.gap(10.0).padding(16.0).items_start());

            let shell_inner = v_stack((
                Main::new().child(main_content).view().into_any(),
                app_bar_overlay.into_any(),
                footer_overlay.into_any(),
                shell_drawer_view.into_any(),
            ))
            .style(|s| s.width_pct(100.0).height_pct(100.0));

            let app_shell = App::new().child(shell_inner).view();

            page(
                "App",
                floem::views::Container::new(app_shell)
                    .style(|s| s.height(460.0).width_pct(100.0))
                    .into_any(),
            )
        }),
    ));

    // 52. Main
    pages.push((
        "Main".to_string(),
        Box::new(|| {
            let main_view = Main::new()
                .child(
                    v_stack((
                        Text::new("Main content region").view().into_any(),
                        Text::new(
                            "Main auto-insets from the AppBar, Footer, and an open NavigationDrawer \
                             via the layout signal — driven entirely by the layout signal.",
                        )
                        .view()
                        .style(|s| s.font_size(13.0))
                        .into_any(),
                    ))
                    .style(|s| s.gap(8.0).padding(16.0).items_start()),
                )
                .view()
                .into_any();
            page(
                "Main",
                floem::views::Container::new(main_view)
                    .style(|s| s.height(220.0).width_pct(100.0))
                    .into_any(),
            )
        }),
    ));

    // 53. Form (pre-built, SharedView)
    pages.push((
        "Form".to_string(),
        Box::new(move || page("Form", form_shared.clone().into_any())),
    ));

    // 54. ValidationMessages
    pages.push((
        "ValidationMessages".to_string(),
        Box::new(|| {
            let vm = ValidationMessages::new();
            let vm_sig = vm.messages_signal();
            let vm_view = vm.view();
            let vm_toggle = Button::new("Toggle errors")
                .variant(ButtonVariant::Secondary)
                .on_click(move || {
                    let cur = vm_sig.get();
                    if cur.is_empty() {
                        vm_sig.set(vec!["Name is required.".to_string()]);
                    } else {
                        vm_sig.set(vec![]);
                    }
                })
                .view()
                .into_any();
            page(
                "ValidationMessages",
                v_stack((vm_toggle, vm_view.into_any()))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 55. Stepper
    pages.push((
        "Stepper".to_string(),
        Box::new(|| {
            let stepper_sel = RwSignal::new(1usize);
            let stepper = Stepper::new()
                .items(vec![
                    "Cart".to_string(),
                    "Shipping".to_string(),
                    "Payment".to_string(),
                    "Review".to_string(),
                ])
                .selected(1)
                .editable(true)
                .on_change(move |i| stepper_sel.set(i));
            let stepper_view = stepper.view();
            page(
                "Stepper",
                v_stack((
                    stepper_view.into_any(),
                    label(move || format!("Step: {}", stepper_sel.get()))
                        .style(|s| s.font_size(14.0))
                        .into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 56. Breadcrumbs
    pages.push((
        "Breadcrumbs".to_string(),
        Box::new(|| {
            page(
                "Breadcrumbs",
                Breadcrumbs::new()
                    .items(vec![
                        "Home".to_string(),
                        "Products".to_string(),
                        "Category".to_string(),
                        "Item".to_string(),
                    ])
                    .on_click(move |_i| {})
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 57. ThemeProvider
    pages.push((
        "ThemeProvider".to_string(),
        Box::new(|| {
            let theme_provider = ThemeProvider::new().dark(false);
            let dark = theme_provider.dark_signal();
            let provider_card = theme_provider
                .child(
                    floem::views::Container::new(
                        v_stack((
                            text("Themed card — driven by ThemeProvider.dark_signal()"),
                            label(move || format!("dark mode = {}", dark.get())),
                        ))
                        .style(|s| s.gap(8.0).padding(16.0)),
                    ),
                )
                .view();
            let dark_toggle = Button::new("Toggle dark")
                .variant(ButtonVariant::Secondary)
                .on_click(move || dark.set(!dark.get()))
                .view()
                .into_any();
            page(
                "ThemeProvider",
                h_stack((provider_card.into_any(), dark_toggle))
                    .style(|s| s.gap(16.0).items_center())
                    .into_any(),
            )
        }),
    ));

    // 58. LocaleProvider
    pages.push((
        "LocaleProvider".to_string(),
        Box::new(|| {
            let locale_provider = LocaleProvider::new().locale("zh-CN");
            let locale_sig = locale_provider.locale_signal();
            page(
                "LocaleProvider",
                v_stack((
                    label(move || format!("locale = {}", locale_sig.get())).style(|s| s.font_size(14.0)).into_any(),
                    locale_provider.view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 59. DisplayProvider
    pages.push((
        "DisplayProvider".to_string(),
        Box::new(|| {
            let display_provider = DisplayProvider::new().dense(false);
            let dense_sig = display_provider.density_signal();
            page(
                "DisplayProvider",
                v_stack((
                    label(move || format!("dense = {}", dense_sig.get())).style(|s| s.font_size(14.0)).into_any(),
                    display_provider.view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 60. DefaultsProvider
    pages.push((
        "DefaultsProvider".to_string(),
        Box::new(|| {
            let defaults_provider = DefaultsProvider::new().defaults(HashMap::new());
            let defaults_sig = defaults_provider.defaults_signal();
            page(
                "DefaultsProvider",
                v_stack((
                    label(move || format!("defaults keys = {}", defaults_sig.get().len())).style(|s| s.font_size(14.0)).into_any(),
                    defaults_provider.view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 61. IconsProvider
    pages.push((
        "IconsProvider".to_string(),
        Box::new(|| {
            let icons_provider = IconsProvider::new().icons(HashMap::new());
            let icons_sig = icons_provider.icons_signal();
            page(
                "IconsProvider",
                v_stack((
                    label(move || format!("icons keys = {}", icons_sig.get().len())).style(|s| s.font_size(14.0)).into_any(),
                    icons_provider.view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 62. DateAdapterProvider
    pages.push((
        "DateAdapterProvider".to_string(),
        Box::new(|| {
            let date_provider = DateAdapterProvider::new().adapter("native");
            let adapter_sig = date_provider.adapter_signal();
            page(
                "DateAdapterProvider",
                v_stack((
                    label(move || format!("date adapter = {}", adapter_sig.get())).style(|s| s.font_size(14.0)).into_any(),
                    date_provider.view().into_any(),
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 63. Window
    pages.push((
        "Window".to_string(),
        Box::new(|| {
            let win_sel = RwSignal::new(0usize);
            let slides: Vec<AnyView> = (0..3)
                .map(|i| text(format!("Slide {i}")).style(|s| s.font_size(16.0).padding(24.0)).into_any())
                .collect();
            page(
                "Window",
                Window::new()
                    .items(slides)
                    .height(160.0)
                    .selected(0)
                    .show_arrows(true)
                    .show_dots(true)
                    .on_change(move |i| win_sel.set(i))
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 64. SlideGroup
    pages.push((
        "SlideGroup".to_string(),
        Box::new(|| {
            let sg_sel = RwSignal::new(0usize);
            let sg_items: Vec<AnyView> = ["One", "Two", "Three", "Four"]
                .iter()
                .map(|l| text(*l).style(|s| s.padding(10.0)).into_any())
                .collect();
            page(
                "SlideGroup",
                SlideGroup::new()
                    .items(sg_items)
                    .selected(0)
                    .show_arrows(true)
                    .on_change(move |i| sg_sel.set(i))
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 65. ItemGroup
    pages.push((
        "ItemGroup".to_string(),
        Box::new(|| {
            let ig_sel = RwSignal::new(0usize);
            let ig_items: Vec<AnyView> = ["Apple", "Banana", "Cherry"]
                .iter()
                .map(|l| text(*l).style(|s| s.padding(10.0)).into_any())
                .collect();
            page(
                "ItemGroup",
                ItemGroup::new()
                    .items(ig_items)
                    .selected(0)
                    .on_change(move |i| ig_sel.set(i))
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 66. ColorPicker
    pages.push((
        "ColorPicker".to_string(),
        Box::new(|| {
            let picked = RwSignal::new(Color::rgb(99, 102, 241));
            let color_picker = ColorPicker::new()
                .value(Color::rgb(99, 102, 241))
                .show_alpha(true)
                .on_change(move |c| picked.set(c));
            let color_sig = color_picker.color_signal();
            let color_picker_view = color_picker.view();
            let color_label = label(move || format!("picked = {}", color_sig.get().to_css()))
                .style(|s| s.font_size(14.0))
                .into_any();
            page(
                "ColorPicker",
                h_stack((color_picker_view.into_any(), color_label))
                    .style(|s| s.gap(16.0).items_center())
                    .into_any(),
            )
        }),
    ));

    // 67. ButtonGroup
    pages.push((
        "ButtonGroup".to_string(),
        Box::new(|| {
            page(
                "ButtonGroup",
                ButtonGroup::new()
                    .joined(true)
                    .child(Button::new("One").variant(ButtonVariant::Primary).view())
                    .child(Button::new("Two").variant(ButtonVariant::Primary).view())
                    .child(Button::new("Three").variant(ButtonVariant::Primary).view())
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 68. FloatingActionButton
    pages.push((
        "FloatingActionButton".to_string(),
        Box::new(|| {
            let fab_click = RwSignal::new(0usize);
            let fab_label = label(move || format!("FAB clicked: {}", fab_click.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let fab_view = FloatingActionButton::new()
                .child(Icon::new("add").view())
                .on_click(move || fab_click.update(|c| *c += 1))
                .view()
                .into_any();
            let fab_ext_view = FloatingActionButton::new()
                .label("Create")
                .child(Icon::new("add").view())
                .extended(true)
                .on_click(move || fab_click.update(|c| *c += 1))
                .view()
                .into_any();
            page(
                "FloatingActionButton",
                v_stack((
                    h_stack((fab_view, fab_ext_view)).style(|s| s.gap(16.0).items_center()),
                    fab_label,
                ))
                .style(|s| s.gap(8.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 69. RangeSlider
    pages.push((
        "RangeSlider".to_string(),
        Box::new(|| {
            let rs_val = RwSignal::new((25.0f32, 75.0f32));
            let rs_label = label(move || {
                let (l, h) = rs_val.get();
                format!("Range: {:.0}–{:.0}", l, h)
            })
            .style(|s| s.font_size(14.0))
            .into_any();
            let range_slider_view = RangeSlider::new()
                .low(25.0)
                .high(75.0)
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .on_change(move |(l, h)| rs_val.set((l, h)))
                .view()
                .into_any();
            page(
                "RangeSlider",
                v_stack((range_slider_view, rs_label))
                    .style(|s| s.gap(8.0).width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 70. FileInput
    pages.push((
        "FileInput".to_string(),
        Box::new(|| {
            let file_input = FileInput::new().label("Upload");
            let fi_sig = file_input.path_signal();
            let fi_label = label(move || {
                format!(
                    "Selected: {}",
                    fi_sig.get().clone().unwrap_or_else(|| "none".to_string())
                )
            })
            .style(|s| s.font_size(14.0))
            .into_any();
            let file_input_view = file_input
                .on_change(move |_p: Option<String>| fi_sig.set(Some("example_report.pdf".to_string())))
                .view()
                .into_any();
            page(
                "FileInput",
                v_stack((file_input_view, fi_label))
                    .style(|s| s.gap(8.0).width(360.0))
                    .into_any(),
            )
        }),
    ));

    // 71. OtpInput
    pages.push((
        "OtpInput".to_string(),
        Box::new(|| {
            let otp_code = RwSignal::new(String::new());
            let otp_label = label(move || format!("OTP: {}", otp_code.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let otp = OtpInput::new().length(6).masked(false);
            let otp_sig = otp.code_signal();
            let otp_view = otp
                .on_change(move |code: String| otp_code.set(code))
                .view()
                .into_any();
            let otp_masked_view = OtpInput::new().length(4).masked(true).view().into_any();
            page(
                "OtpInput",
                v_stack((otp_view, otp_masked_view, otp_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 72. ListItem
    pages.push((
        "ListItem".to_string(),
        Box::new(|| {
            let theme = theme_signal();
            page(
                "ListItem",
                v_stack((
                    ListItem::new().title("Inbox").subtitle("12 new messages").leading(Icon::new("email").view()).view(),
                    ListItem::new().title("Sent").subtitle("Yesterday").leading(Icon::new("send").view()).view(),
                    ListItem::new().title("Trash").leading(Icon::new("delete").view()).view(),
                ))
                .style(move |s| {
                    s.width(360.0)
                        .background(theme.get().colors().surface_variant.to_peniko())
                        .border_radius(12.0)
                })
                .into_any(),
            )
        }),
    ));

    // 73. Table
    pages.push((
        "Table".to_string(),
        Box::new(|| {
            page(
                "Table",
                Table::new()
                    .headers(vec!["Name".into(), "Role".into(), "Status".into()])
                    .rows(vec![
                        vec!["Ada".to_string(), "Engineer".to_string(), "Active".to_string()],
                        vec!["Linus".to_string(), "Maintainer".to_string(), "Active".to_string()],
                        vec!["Grace".to_string(), "Architect".to_string(), "Away".to_string()],
                    ])
                    .striped(true)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 74. DataIterator
    pages.push((
        "DataIterator".to_string(),
        Box::new(|| {
            page(
                "DataIterator",
                DataIterator::new()
                    .divided(true)
                    .item(Text::new("Record 1 — updated 2m ago").view())
                    .item(Text::new("Record 2 — updated 14m ago").view())
                    .item(Text::new("Record 3 — updated 1h ago").view())
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 75. Sparkline
    pages.push((
        "Sparkline".to_string(),
        Box::new(|| {
            page(
                "Sparkline",
                Sparkline::new()
                    .values(vec![3.0, 7.0, 4.0, 9.0, 6.0, 11.0, 8.0, 13.0])
                    .width(240.0)
                    .height(48.0)
                    .fill(true)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 76. Kbd
    pages.push((
        "Kbd".to_string(),
        Box::new(|| {
            page(
                "Kbd",
                h_stack((
                    Kbd::new().content("Ctrl").view(),
                    Kbd::new().content("Shift").view(),
                    Kbd::new().content("P").view(),
                ))
                .style(|s| s.gap(6.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 77. Code
    pages.push((
        "Code".to_string(),
        Box::new(|| page("Code", Code::new().content("let x = 42;").view().into_any())),
    ));

    // 78. Radio
    pages.push((
        "Radio".to_string(),
        Box::new(|| {
            page(
                "Radio",
                v_stack((
                    Radio::new("Enabled, unchecked").view(),
                    Radio::new("Enabled, checked").checked(true).view(),
                    Radio::new("Disabled, unchecked").disabled(true).view(),
                    Radio::new("Disabled, checked").disabled(true).checked(true).view(),
                ))
                .style(|s| s.gap(6.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 79. Overlay (overlay surface lives in `overlays`)
    pages.push((
        "Overlay".to_string(),
        Box::new(move || {
            page(
                "Overlay",
                v_stack((
                    Button::new("Show overlay")
                        .variant(ButtonVariant::Secondary)
                        .on_click(move || overlay_show.set(true))
                        .view()
                        .into_any(),
                    Text::new(
                        "A generic scrim + content surface. The floating surface renders at the \
                         window root.",
                    )
                    .view(),
                ))
                .style(|s| s.gap(10.0).items_start())
                .into_any(),
            )
        }),
    ));

    // 80. Parallax
    pages.push((
        "Parallax".to_string(),
        Box::new(|| {
            let parallax_scroll = RwSignal::new(0.0_f32);
            let parallax = Parallax::new()
                .factor(0.4)
                .scroll_signal(parallax_scroll)
                .child(
                    label(move || {
                        format!(
                            "Background layer · translated by -scroll×factor = {:.0}px",
                            -parallax_scroll.get() * 0.4
                        )
                    })
                    .style(|s| s.font_size(22.0).padding(28.0))
                    .into_any(),
                );
            let parallax_view = parallax.view();
            let parallax_slider = Slider::new()
                .value(0.0)
                .min(0.0)
                .max(300.0)
                .step(1.0)
                .on_change(move |v| parallax_scroll.set(v))
                .view()
                .into_any();
            let parallax_caption = label(move || {
                format!(
                    "Drag the slider to scroll; the background lags (factor 0.4). Host owns the \
                     scroll signal."
                )
            });
            page(
                "Parallax",
                v_stack((parallax_view, parallax_slider, parallax_caption))
                    .style(|s| s.gap(10.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 81. InfiniteScroller
    pages.push((
        "InfiniteScroller".to_string(),
        Box::new(|| {
            let theme = theme_signal();
            let inf_count = RwSignal::new(15usize);
            let inf_item_h = 36.0;
            let inf_content_h = RwSignal::new(15.0 * inf_item_h);
            let inf_child = floem::views::dyn_view(move || {
                let n = inf_count.get();
                let mut rows: Vec<AnyView> = Vec::new();
                for i in 0..n {
                    let row = Container::new()
                        .child(
                            label(move || format!("Row {}", i))
                                .style(|s| s.font_size(14.0))
                                .into_any(),
                        )
                        .view()
                        .style(move |s| {
                            s.height(inf_item_h)
                                .width_pct(100.0)
                                .items_center()
                                .padding_horiz(12.0)
                                .background(
                                    with_alpha(
                                        theme.get().colors().on_surface,
                                        if i % 2 == 0 { 0 } else { 6 },
                                    )
                                    .to_peniko(),
                                )
                        });
                    rows.push(row.into_any());
                }
                v_stack_from_iter(rows).style(|s| s.width_pct(100.0)).into_any()
            });
            let inf_scroller = InfiniteScroller::new()
                .viewport(240.0)
                .content_height_signal(inf_content_h)
                .on_load_more(move || {
                    let n = inf_count.get() + 15;
                    inf_count.set(n);
                    inf_content_h.set(n as f32 * inf_item_h);
                })
                .child(inf_child);
            let inf_loading_sig = inf_scroller.loading_signal();
            let inf_scroller_view = inf_scroller.view();
            let inf_status = label(move || {
                format!(
                    "{} rows loaded · loading: {}  (scroll to the bottom to append 15 more)",
                    inf_count.get(),
                    inf_loading_sig.get()
                )
            });
            page(
                "InfiniteScroller",
                v_stack((inf_scroller_view, inf_status))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 82. Hover
    pages.push((
        "Hover".to_string(),
        Box::new(|| {
            let hover = Hover::new().child(
                Text::new("Hover over this card")
                    .view()
                    .style(|s| s.padding(20.0).font_size(16.0)),
            );
            let hover_sig = hover.hovered_signal();
            let hover_status = label(move || format!("hovered: {}", hover_sig.get()));
            page(
                "Hover",
                v_stack((hover.view(), hover_status))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 83. FadeTransition
    pages.push((
        "FadeTransition".to_string(),
        Box::new(|| {
            let fade = FadeTransition::new().child(|| {
                label(|| "Content that fades in / out".to_string())
                    .style(|s| s.padding(16.0).font_size(16.0))
            });
            let fade_show = fade.show_signal();
            let fade_view = fade.view();
            let fade_btn = Button::new("Toggle fade")
                .variant(ButtonVariant::Secondary)
                .on_click(move || fade_show.update(|v| *v = !*v))
                .view()
                .into_any();
            page(
                "FadeTransition",
                v_stack((fade_btn, fade_view)).style(|s| s.gap(10.0).items_start()).into_any(),
            )
        }),
    ));

    // 84. ScaleTransition
    pages.push((
        "ScaleTransition".to_string(),
        Box::new(|| {
            let scale = ScaleTransition::new().child(|| {
                label(|| "Content that scales in / out".to_string())
                    .style(|s| s.padding(16.0).font_size(16.0))
            });
            let scale_show = scale.show_signal();
            let scale_view = scale.view();
            let scale_btn = Button::new("Toggle scale")
                .variant(ButtonVariant::Secondary)
                .on_click(move || scale_show.update(|v| *v = !*v))
                .view()
                .into_any();
            page(
                "ScaleTransition",
                v_stack((scale_btn, scale_view)).style(|s| s.gap(10.0).items_start()).into_any(),
            )
        }),
    ));

    // 85. ExpandTransition
    pages.push((
        "ExpandTransition".to_string(),
        Box::new(|| {
            let expand = ExpandTransition::new().child(|| {
                label(|| "Content that expands / collapses (height 0 -> natural)".to_string())
                    .style(|s| s.padding(16.0).font_size(16.0))
            });
            let expand_show = expand.show_signal();
            let expand_view = expand.view();
            let expand_btn = Button::new("Toggle expand")
                .variant(ButtonVariant::Secondary)
                .on_click(move || expand_show.update(|v| *v = !*v))
                .view()
                .into_any();
            page(
                "ExpandTransition",
                v_stack((expand_btn, expand_view)).style(|s| s.gap(10.0).items_start()).into_any(),
            )
        }),
    ));

    // 86. SlideTransition
    pages.push((
        "SlideTransition".to_string(),
        Box::new(|| {
            let slide = SlideTransition::new()
                .direction(SlideDirection::X)
                .child(|| {
                    label(|| "Content that slides in / out".to_string())
                        .style(|s| s.padding(16.0).font_size(16.0))
                });
            let slide_show = slide.show_signal();
            let slide_view = slide.view();
            let slide_btn = Button::new("Toggle slide")
                .variant(ButtonVariant::Secondary)
                .on_click(move || slide_show.update(|v| *v = !*v))
                .view()
                .into_any();
            page(
                "SlideTransition",
                v_stack((slide_btn, slide_view)).style(|s| s.gap(10.0).items_start()).into_any(),
            )
        }),
    ));

    // 87. IconButton
    pages.push((
        "IconButton".to_string(),
        Box::new(|| {
            let ib_click = RwSignal::new(0usize);
            let ib_label = label(move || format!("IconButton clicks: {}", ib_click.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let ib_view = h_stack((
                IconButton::new(HEART_SVG).view().into_any(),
                IconButton::new(HEART_SVG)
                    .variant(IconButtonVariant::Contained)
                    .on_click(move || ib_click.update(|c| *c += 1))
                    .view()
                    .into_any(),
                IconButton::new(HEART_SVG)
                    .variant(IconButtonVariant::Outlined)
                    .view()
                    .into_any(),
                IconButton::new(HEART_SVG).disabled(true).view().into_any(),
            ))
            .style(|s| s.gap(12.0).items_center());
            page(
                "IconButton",
                v_stack((ib_label, ib_view.into_any())).style(|s| s.gap(8.0).items_start()).into_any(),
            )
        }),
    ));

    // 88. ExpansionPanels
    pages.push((
        "ExpansionPanels".to_string(),
        Box::new(|| {
            page(
                "ExpansionPanels",
                ExpansionPanels::new()
                    .panel("What is floem-ui?", || {
                        Text::new("A cross-design-system Rust UI library built on Floem.")
                            .view()
                            .into_any()
                    })
                    .panel("How many components?", || {
                        Text::new("95 components × 3 design systems.").view().into_any()
                    })
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 89. SpeedDial
    pages.push((
        "SpeedDial".to_string(),
        Box::new(|| {
            page(
                "SpeedDial",
                SpeedDial::new()
                    .fab_icon(DIAL_SVG.to_string())
                    .action(EDIT_SVG.to_string(), Some("Edit".to_string()))
                    .action(SHARE_SVG.to_string(), Some("Share".to_string()))
                    .open(true)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 90. ChipGroup
    pages.push((
        "ChipGroup".to_string(),
        Box::new(|| {
            page(
                "ChipGroup",
                ChipGroup::new()
                    .chip("Apple")
                    .chip("Banana")
                    .chip("Cherry")
                    .chip("Date")
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 91. ListGroup
    pages.push((
        "ListGroup".to_string(),
        Box::new(|| {
            page(
                "ListGroup",
                ListGroup::new()
                    .title("Fruits")
                    .item("Apple")
                    .item("Banana")
                    .item("Cherry")
                    .expanded(true)
                    .view()
                    .into_any(),
            )
        }),
    ));

    // 92. Counter
    pages.push((
        "Counter".to_string(),
        Box::new(|| {
            let counter_val = RwSignal::new(3i64);
            let counter_label = label(move || format!("Counter value: {}", counter_val.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let counter_view = Counter::new()
                .value(3)
                .min(0)
                .max(10)
                .step(1)
                .on_change(move |v: i64| counter_val.set(v))
                .view()
                .into_any();
            page(
                "Counter",
                v_stack((counter_view, counter_label))
                    .style(|s| s.gap(8.0).items_start())
                    .into_any(),
            )
        }),
    ));

    // 93. Responsive
    pages.push((
        "Responsive".to_string(),
        Box::new(|| {
            page(
                "Responsive",
                Responsive::new(16.0f32 / 9.0f32)
                    .child(|| {
                        Text::new("16:9 responsive box")
                            .view()
                            .style(|s| s.font_size(14.0))
                            .into_any()
                    })
                    .view()
                    .style(|s| s.width(320.0))
                    .into_any(),
            )
        }),
    ));

    // 94. CheckboxBtn
    pages.push((
        "CheckboxBtn".to_string(),
        Box::new(|| {
            page(
                "CheckboxBtn",
                h_stack((
                    CheckboxBtn::new().checked(true).label("Enabled").view().into_any(),
                    CheckboxBtn::new().label("Unchecked").view().into_any(),
                    CheckboxBtn::new().label("Disabled").disabled(true).view().into_any(),
                ))
                .style(|s| s.gap(16.0).items_center())
                .into_any(),
            )
        }),
    ));

    // 95. ListSubheader
    pages.push((
        "ListSubheader".to_string(),
        Box::new(|| {
            page(
                "ListSubheader",
                v_stack((
                    ListSubheader::new("Section A").view().into_any(),
                    ListSubheader::new("Section B (inset)").inset(true).view().into_any(),
                ))
                .style(|s| s.gap(0.0).width(360.0))
                .into_any(),
            )
        }),
    ));

    // 96. NumberInput
    pages.push((
        "NumberInput".to_string(),
        Box::new(|| {
            let ni_val = RwSignal::new(5i64);
            let ni_label = label(move || format!("NumberInput: {}", ni_val.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let ni_view = NumberInput::new()
                .value(5)
                .min(0)
                .max(10)
                .step(1)
                .label("Quantity")
                .on_change(move |v: i64| ni_val.set(v))
                .view()
                .into_any();
            page(
                "NumberInput",
                v_stack((ni_view, ni_label)).style(|s| s.gap(8.0).items_start()).into_any(),
            )
        }),
    ));

    // 97. PullToRefresh (pre-built, SharedView)
    pages.push((
        "PullToRefresh".to_string(),
        Box::new(move || page("PullToRefresh", ptr_shared.clone().into_any())),
    ));

    // 98. ConfirmEdit
    pages.push((
        "ConfirmEdit".to_string(),
        Box::new(|| {
            let ce_val = RwSignal::new(String::from("Click the ✎ to edit this text"));
            let ce_label = label(move || format!("saved value: {}", ce_val.get()))
                .style(|s| s.font_size(14.0))
                .into_any();
            let ce_view = ConfirmEdit::new()
                .value("Click the ✎ to edit this text")
                .on_save(move |s: String| ce_val.set(s))
                .child(|| {
                    Text::new("Click the ✎ to edit this text")
                        .view()
                        .style(|s| s.font_size(14.0))
                        .into_any()
                })
                .view()
                .into_any();
            page(
                "ConfirmEdit",
                v_stack((ce_view, ce_label)).style(|s| s.gap(8.0).items_start()).into_any(),
            )
        }),
    ));

    Gallery { pages, overlays }
}
