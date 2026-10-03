//! Phase B pilot component showcases (Demo Show Gallery).
//!
//! Each function below is a complete, "hands-on" component page built from the
//! helpers in `showcase.rs`: an interactive property panel (`demo_ctl`),
//! method / event demos (`demo_method` + `status_badge`), static examples
//! (`demo` / `demo_note`), an API reference table (`api_table`), and a real
//! source snippet (`code_snippet`). This is the reference template the rest
//! of the 98 components will be migrated to.
//!
//! Floating surfaces (Dialog / Snackbar / NavigationDrawer) take the persistent
//! window-root `open` signal produced by `gallery::build()` — their method demo
//! toggles that signal and the actual surface (rendered at the window root)
//! opens, exactly like the production app.

use floom::IntoView;
use floom::reactive::{RwSignal, SignalGet, SignalUpdate};
use floom::view::AnyView;
use floom::views::{Decorators, Label, Stack, dyn_view};
use floom_core_theme::props::alert::AlertVariant;
use floom_core_theme::props::datatable::{ColumnAlign, DataTableColumn};
use floom_elements::button::ButtonVariant;
use floom_elements::prelude::*;

use crate::showcase::{
    api_table, code_snippet, demo, demo_ctl, demo_method, demo_note, segmented, showcase,
    status_badge, toggle_row, ApiRow,
};

// ===========================================================================
// TextField
// ===========================================================================

const TEXTFIELD_SRC: &str = r#"TextField::new()
    .placeholder("Enter text…")
    .password(false)
    .on_change(|s: &str| println!("{s}"))
    .view();"#;

fn textfield_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "\"\"", desc: "Hint shown when empty." },
        ApiRow { name: "initial_value", ty: "impl Into<String>", default: "\"\"", desc: "Starting text." },
        ApiRow { name: "password", ty: "bool", default: "false", desc: "Mask input characters." },
        ApiRow { name: "max_length", ty: "usize", default: "none", desc: "Cap input length." },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "Block interaction." },
        ApiRow { name: "on_change", ty: "Fn(&str)", default: "—", desc: "Fires on every edit (event)." },
    ]
}

pub fn text_field_showcase() -> AnyView {
    let password = RwSignal::new(false);
    let disabled = RwSignal::new(false);
    let comp = {
        let password = password;
        let disabled = disabled;
        dyn_view(move || {
            TextField::new()
                .placeholder("Enter text…")
                .password(password.get())
                .disabled(disabled.get())
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((toggle_row("Password", password), toggle_row("Disabled", disabled)))
        .style(|s| s.gap(12.0).items_start());

    let typed = RwSignal::new(String::new());
    let event_comp = dyn_view(move || {
        TextField::new()
            .placeholder("Type to fire on_change")
            .on_change(move |s: &str| typed.set(s.to_string()))
            .view()
            .into_any()
    });
    let event_view = Stack::vertical((
        event_comp,
        Label::derived(move || format!("on_change → “{}”", typed.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(8.0).items_start())
    .into_any();

    showcase(
        "TextField",
        "Text fields capture user input. Toggle password masking and disabled state, and watch the change event echo live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Event · on_change", event_view),
            demo_note("Basic", TextField::new().placeholder("Basic").view(), "Standard single-line input."),
            demo_note("Password", TextField::new().placeholder("Secret").password(true).view(), "Masks entered characters."),
            demo_note("Disabled", TextField::new().placeholder("Disabled").disabled(true).view(), "Non-interactive."),
            demo_note("Max length 5", TextField::new().placeholder("≤ 5 chars").max_length(5).view(), "Caps input length."),
            code_snippet("TextField · 基础", TEXTFIELD_SRC),
            api_table("Props", textfield_api()),
        ],
    )
}

// ===========================================================================
// Select
// ===========================================================================

const SELECT_SRC: &str = r#"let selected = RwSignal::new(Some(0usize));
Select::new()
    .options(vec!["Apple".into(), "Banana".into()])
    .selected(selected.get())
    .on_change(move |i| selected.set(i))
    .view();"#;

fn select_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "options", ty: "Vec<String>", default: "[]", desc: "Items to choose from." },
        ApiRow { name: "selected", ty: "Option<usize>", default: "None", desc: "Index of chosen item." },
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "\"\"", desc: "Shown when nothing selected." },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "Block interaction." },
        ApiRow { name: "on_change", ty: "Fn(Option<usize>)", default: "—", desc: "Fires on selection (event)." },
    ]
}

pub fn select_showcase() -> AnyView {
    let selected = RwSignal::new(Some(1usize));
    let disabled = RwSignal::new(false);
    let comp = {
        let selected = selected;
        let disabled = disabled;
        dyn_view(move || {
            Select::new()
                .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
                .selected(selected.get())
                .disabled(disabled.get())
                .on_change(move |i: Option<usize>| selected.set(i))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("selected = {:?}", selected.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Select",
        "Selects pick one option from a dropdown. The chosen index is tracked in a signal and echoed live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Basic", Select::new().options(vec!["A".into(), "B".into()]).selected(Some(0)).view()),
            demo_note("Placeholder", Select::new().options(vec!["X".into()]).placeholder("Pick…").view(), "Empty selection shows placeholder."),
            demo_note("Disabled", Select::new().options(vec!["X".into()]).disabled(true).view(), "Non-interactive."),
            code_snippet("Select · 基础", SELECT_SRC),
            api_table("Props", select_api()),
        ],
    )
}

// ===========================================================================
// Alert
// ===========================================================================

const ALERT_SRC: &str = r#"Alert::new()
    .variant(AlertVariant::Warning)
    .title("Heads up")
    .message("This action cannot be undone.")
    .dismissible(true)
    .on_dismiss(|| println!("dismissed"))
    .view();"#;

fn alert_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "variant", ty: "AlertVariant", default: "Info", desc: "Info / Success / Warning / Error." },
        ApiRow { name: "title", ty: "impl Into<String>", default: "\"\"", desc: "Bold heading." },
        ApiRow { name: "message", ty: "impl Into<String>", default: "\"\"", desc: "Body text." },
        ApiRow { name: "dismissible", ty: "bool", default: "false", desc: "Show a close button." },
        ApiRow { name: "on_dismiss", ty: "Fn()", default: "—", desc: "Fires when closed (event)." },
    ]
}

pub fn alert_showcase() -> AnyView {
    let variant = RwSignal::new(AlertVariant::Info);
    let dismissible = RwSignal::new(true);
    let comp = {
        let variant = variant;
        let dismissible = dismissible;
        dyn_view(move || {
            Alert::new()
                .variant(variant.get())
                .title("Storage almost full")
                .message("You have used 92% of your quota.")
                .dismissible(dismissible.get())
                .on_dismiss(|| {})
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        segmented(
            "Variant",
            vec![
                ("Info", AlertVariant::Info),
                ("Success", AlertVariant::Success),
                ("Warning", AlertVariant::Warning),
                ("Error", AlertVariant::Error),
            ],
            variant,
        ),
        toggle_row("Dismissible", dismissible),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Alert",
        "Alerts convey status and important messages. Switch the variant and dismissible flag live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Info", Alert::new().variant(AlertVariant::Info).title("Info").message("Informational message.").view()),
            demo("Success", Alert::new().variant(AlertVariant::Success).title("Saved").message("Your changes were saved.").view()),
            demo("Warning", Alert::new().variant(AlertVariant::Warning).title("Careful").message("This may take a while.").view()),
            demo("Error", Alert::new().variant(AlertVariant::Error).title("Failed").message("Something went wrong.").view()),
            code_snippet("Alert · 基础", ALERT_SRC),
            api_table("Props", alert_api()),
        ],
    )
}

// ===========================================================================
// Rating
// ===========================================================================

const RATING_SRC: &str = r#"let value = RwSignal::new(3.0f32);
Rating::new()
    .value(value.get())
    .max(5)
    .on_change(move |v| value.set(v))
    .view();"#;

fn rating_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "value", ty: "f32", default: "0.0", desc: "Current rating." },
        ApiRow { name: "max", ty: "usize", default: "5", desc: "Number of items." },
        ApiRow { name: "readonly", ty: "bool", default: "false", desc: "Block changes." },
        ApiRow { name: "on_change", ty: "Fn(f32)", default: "—", desc: "Fires on rating (event)." },
    ]
}

pub fn rating_showcase() -> AnyView {
    let value = RwSignal::new(3.0f32);
    let readonly = RwSignal::new(false);
    let comp = {
        let value = value;
        let readonly = readonly;
        dyn_view(move || {
            Rating::new()
                .value(value.get())
                .max(5)
                .readonly(readonly.get())
                .on_change(move |v: f32| value.set(v))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        Label::derived(move || format!("value = {:.1}", value.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
        toggle_row("Readonly", readonly),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Rating",
        "Ratings collect a 0–max score. Drag to change the value; flip readonly to lock it.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Readonly", Rating::new().value(4.0).max(5).readonly(true).view()),
            code_snippet("Rating · 基础", RATING_SRC),
            api_table("Props", rating_api()),
        ],
    )
}

// ===========================================================================
// DataTable
// ===========================================================================

const DATATABLE_SRC: &str = r#"let clicked = RwSignal::new(usize::MAX);
DataTable::new()
    .columns(vec![
        DataTableColumn { title: "Name".into(), align: ColumnAlign::Start },
        DataTableColumn { title: "Role".into(), align: ColumnAlign::Start },
    ])
    .rows(vec![vec!["Ada".into(), "Engineer".into()]])
    .striped(true)
    .on_row_click(move |i| clicked.set(i))
    .view();"#;

fn datatable_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "columns", ty: "Vec<DataTableColumn>", default: "[]", desc: "Header definitions." },
        ApiRow { name: "rows", ty: "Vec<Vec<String>>", default: "[]", desc: "Cell data, one Vec per row." },
        ApiRow { name: "striped", ty: "bool", default: "false", desc: "Alternate row tinting." },
        ApiRow { name: "on_row_click", ty: "Fn(usize)", default: "—", desc: "Fires with row index (event)." },
    ]
}

pub fn datatable_showcase() -> AnyView {
    let columns = vec![
        DataTableColumn { title: "Name".into(), align: ColumnAlign::Start },
        DataTableColumn { title: "Role".into(), align: ColumnAlign::Start },
    ];
    let rows = vec![
        vec!["Ada".into(), "Engineer".into()],
        vec!["Linus".into(), "Kernel".into()],
        vec!["Grace".into(), "Admiral".into()],
    ];
    let striped = RwSignal::new(true);
    let clicked = RwSignal::new(usize::MAX);
    let comp = {
        let striped = striped;
        let clicked = clicked;
        let columns = columns.clone();
        let rows = rows.clone();
        dyn_view(move || {
            DataTable::new()
                .columns(columns.clone())
                .rows(rows.clone())
                .striped(striped.get())
                .on_row_click(move |i: usize| clicked.set(i))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Striped", striped),
        Label::derived(move || {
            let i = clicked.get();
            if i == usize::MAX { "click a row".to_string() } else { format!("on_row_click → {i}") }
        })
        .style(|s| s.font_size(13.0))
        .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "DataTable",
        "DataTables render tabular data. Toggle striped rows and click a row to see the event.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("DataTable · 基础", DATATABLE_SRC),
            api_table("Props", datatable_api()),
        ],
    )
}

// ===========================================================================
// FloatingActionButton
// ===========================================================================

const FAB_SRC: &str = r#"FloatingActionButton::new()
    .label("Create")
    .extended(true)
    .child(Text::new("+").view())
    .on_click(|| println!("create"))
    .view();"#;

fn fab_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "Icon / content." },
        ApiRow { name: "label", ty: "impl Into<String>", default: "None", desc: "Optional text (extended)." },
        ApiRow { name: "extended", ty: "bool", default: "false", desc: "Show label pill." },
        ApiRow { name: "on_click", ty: "Fn()", default: "—", desc: "Fires on activation (event)." },
    ]
}

pub fn fab_showcase() -> AnyView {
    let extended = RwSignal::new(false);
    let comp = {
        let extended = extended;
        dyn_view(move || {
            FloatingActionButton::new()
                .label("Create")
                .extended(extended.get())
                .child(Text::new("+").view())
                .on_click(|| {})
                .view()
                .into_any()
        })
    };
    let controls = toggle_row("Extended", extended);

    showcase(
        "FloatingActionButton",
        "A FAB promotes the primary action. Toggle extended to reveal its label.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Icon only", FloatingActionButton::new().child(Text::new("+").view()).view()),
            demo("Extended", FloatingActionButton::new().label("Edit").extended(true).child(Text::new("✎").view()).view()),
            code_snippet("FAB · 基础", FAB_SRC),
            api_table("Props", fab_api()),
        ],
    )
}

// ===========================================================================
// Dialog  (floating — uses the window-root open signal)
// ===========================================================================

const DIALOG_SRC: &str = r#"let open = RwSignal::new(false);
Dialog::new()
    .title("Confirm")
    .content("Delete this item?")
    .action("Cancel", || {})
    .action("Confirm", || {})
    .with_open_signal(open)
    .view();
// open.set(true) shows it; on_close sets it false."#;

fn dialog_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "title", ty: "impl Into<String>", default: "\"\"", desc: "Heading." },
        ApiRow { name: "content", ty: "impl Into<String>", default: "\"\"", desc: "Body text." },
        ApiRow { name: "action", ty: "Fn()", default: "—", desc: "Add an action button (method)." },
        ApiRow { name: "open / open_signal", ty: "bool / RwSignal<bool>", default: "false", desc: "Show / bind visibility (method)." },
        ApiRow { name: "on_close", ty: "Fn()", default: "—", desc: "Fires on dismiss (event)." },
    ]
}

pub fn dialog_showcase(open: RwSignal<bool>) -> AnyView {
    showcase(
        "Dialog",
        "A modal surface for decisions. The trigger toggles the window-root overlay's open signal; the badge shows its live state. It scales + fades in, and animates out on dismiss.",
        vec![
            demo_method(
                "方法 · open()",
                "Open dialog",
                move || open.set(true),
                status_badge(open, "open = true", "open = false"),
            ),
            demo_method(
                "方法 · (close)",
                "Close dialog",
                move || open.set(false),
                status_badge(open, "open = true", "open = false"),
            ),
            demo_note("Notes", Text::new("The floating surface is rendered at the window root, so it always covers the full window and re-skins on a Design System switch.").view().into_any(), "Architecture constraint."),
            code_snippet("Dialog · 基础", DIALOG_SRC),
            api_table("Props / Methods", dialog_api()),
        ],
    )
}

// ===========================================================================
// Snackbar  (floating — uses the window-root open signal)
// ===========================================================================

const SNACKBAR_SRC: &str = r#"let open = RwSignal::new(false);
Snackbar::new()
    .message("Item saved")
    .action_label(Some("Undo".into()))
    .duration_ms(4000)
    .with_open_signal(open)
    .view();
open.set(true); // show it"#;

fn snackbar_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "message", ty: "impl Into<String>", default: "\"\"", desc: "Body text." },
        ApiRow { name: "action_label", ty: "Option<String>", default: "None", desc: "Optional action text." },
        ApiRow { name: "duration_ms", ty: "u32", default: "4000", desc: "Auto-dismiss delay." },
        ApiRow { name: "on_action", ty: "Fn()", default: "—", desc: "Fires on action click (event)." },
        ApiRow { name: "with_open_signal", ty: "RwSignal<bool>", default: "—", desc: "Bind visibility (method)." },
    ]
}

pub fn snackbar_showcase(open: RwSignal<bool>) -> AnyView {
    showcase(
        "Snackbar",
        "Snackbars surface brief, transient messages. Trigger opens the window-root overlay; the badge reflects its live visibility.",
        vec![
            demo_method(
                "方法 · show()",
                "Show snackbar",
                move || open.set(true),
                status_badge(open, "visible", "hidden"),
            ),
            demo_note("Notes", Text::new("Auto-dismisses after duration_ms. The actual snackbar is a window-root overlay.").view().into_any(), "Transient surface."),
            code_snippet("Snackbar · 基础", SNACKBAR_SRC),
            api_table("Props / Methods", snackbar_api()),
        ],
    )
}

// ===========================================================================
// NavigationDrawer  (floating — uses the window-root open + side signals)
// ===========================================================================

const DRAWER_SRC: &str = r#"let open = RwSignal::new(false);
NavigationDrawer::new()
    .side(NavigationDrawerSide::Left)
    .header(|| Text::new("Nav").view())
    .child(|| List::new().item("Home", || {}).view())
    .with_open_signal(open)
    .view();"#;

fn drawer_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "side", ty: "NavigationDrawerSide", default: "Left", desc: "Screen edge to anchor." },
        ApiRow { name: "header", ty: "Fn() -> impl IntoView", default: "—", desc: "Top content (method)." },
        ApiRow { name: "child", ty: "Fn() -> impl IntoView", default: "—", desc: "Body content (method)." },
        ApiRow { name: "open / open_signal", ty: "bool / RwSignal<bool>", default: "false", desc: "Show / bind visibility (method)." },
        ApiRow { name: "width / elevation", ty: "f32 / u8", default: "—", desc: "Sizing." },
    ]
}

pub fn navigation_drawer_showcase(open: RwSignal<bool>, side: RwSignal<NavigationDrawerSide>) -> AnyView {
    showcase(
        "NavigationDrawer",
        "A drawer holds navigation. Toggle it open/closed and flip the side; the window-root overlay animates in from the chosen edge.",
        vec![
            demo_method(
                "方法 · open()",
                "Open drawer",
                move || open.set(true),
                status_badge(open, "open = true", "open = false"),
            ),
            demo_method(
                "方法 · close()",
                "Close drawer",
                move || open.set(false),
                status_badge(open, "open = true", "open = false"),
            ),
            demo(
                "方法 · side()",
                Stack::horizontal((
                    Button::new("Left").variant(ButtonVariant::Text).on_click(move || side.set(NavigationDrawerSide::Left)).view().into_any(),
                    Button::new("Right").variant(ButtonVariant::Text).on_click(move || side.set(NavigationDrawerSide::Right)).view().into_any(),
                ))
                .style(|s| s.gap(8.0))
                .into_any(),
            ),
            demo_note("Notes", Text::new("Rendered at the window root; animates from the selected side with a scrim fade.").view().into_any(), "Floating surface."),
            code_snippet("NavigationDrawer · 基础", DRAWER_SRC),
            api_table("Props / Methods", drawer_api()),
        ],
    )
}

// ===========================================================================
// ThemeProvider
// ===========================================================================

const THEMEPROVIDER_SRC: &str = r#"let dark = RwSignal::new(false);
ThemeProvider::new()
    .dark(dark.get())
    .child(/* themed content */)
    .view();
// dark_signal() exposes the process-wide dark state."#;

fn theme_provider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "dark", ty: "bool", default: "false", desc: "Force dark palette on subtree (method)." },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "Themed subtree." },
        ApiRow { name: "dark_signal()", ty: "RwSignal<bool>", default: "—", desc: "Read process-wide dark state (method)." },
    ]
}

pub fn theme_provider_showcase() -> AnyView {
    let dark = RwSignal::new(false);
    let comp = {
        let dark = dark;
        dyn_view(move || {
            let inner = Stack::vertical((
                Text::new("Themed surface").view().into_any(),
                Button::new("Primary action").variant(ButtonVariant::Primary).view().into_any(),
            ))
            .style(|s| s.gap(8.0))
            .into_any();
            ThemeProvider::new().dark(dark.get()).child(inner).view().into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("dark()", dark),
        status_badge(dark, "dark = true", "dark = false"),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ThemeProvider",
        "ThemeProvider forces a light/dark palette on its subtree without touching the rest of the app. Toggle dark() live.",
        vec![
            demo_ctl("方法 · dark()", comp, controls),
            demo_note("Process-wide", Text::new("The top-right Dark toggle calls set_dark(), which flips the global dark_signal() that every component reads.").view().into_any(), "Global theming."),
            code_snippet("ThemeProvider · 基础", THEMEPROVIDER_SRC),
            api_table("Props / Methods", theme_provider_api()),
        ],
    )
}

// ===========================================================================
// LocaleProvider
// ===========================================================================

const LOCALEPROVIDER_SRC: &str = r#"LocaleProvider::new()
    .locale("zh-CN")
    .child(/* localized content */)
    .view();
// locale_signal() exposes the current BCP-47 tag."#;

fn locale_provider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "locale", ty: "impl Into<String>", default: "\"en-US\"", desc: "BCP-47 tag for subtree (method)." },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "Localized subtree." },
        ApiRow { name: "locale_signal()", ty: "RwSignal<String>", default: "—", desc: "Read current locale (method)." },
    ]
}

pub fn locale_provider_showcase() -> AnyView {
    let zh = Stack::vertical((
        Text::new("语言：简体中文").view().into_any(),
        Button::new("确定").variant(ButtonVariant::Primary).view().into_any(),
    ))
    .style(|s| s.gap(8.0))
    .into_any();
    let en = Stack::vertical((
        Text::new("Language: English").view().into_any(),
        Button::new("OK").variant(ButtonVariant::Primary).view().into_any(),
    ))
    .style(|s| s.gap(8.0))
    .into_any();

    showcase(
        "LocaleProvider",
        "LocaleProvider seeds a BCP-47 locale for its subtree. Below, two providers render the same intent in different locales.",
        vec![
            demo("zh-CN", LocaleProvider::new().locale("zh-CN").child(zh).view()),
            demo("en-US", LocaleProvider::new().locale("en-US").child(en).view()),
            demo_note("方法 · locale()", Text::new("Pass a BCP-47 tag; reading it back is available via locale_signal() on the instance.").view().into_any(), "Localization."),
            code_snippet("LocaleProvider · 基础", LOCALEPROVIDER_SRC),
            api_table("Props / Methods", locale_provider_api()),
        ],
    )
}
