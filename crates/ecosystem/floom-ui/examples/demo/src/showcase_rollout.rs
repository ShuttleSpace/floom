//! Phase C — Batch 1 rollout showcases (Demo Show Gallery).
//!
//! Upgrades a first batch of the previously-static gallery pages to the unified
//! template: interactive property panel (`demo_ctl`) + static variants (`demo`)
//! + real source snippet (`code_snippet`) + API reference table (`api_table`).
//! Pattern mirrors `showcase_pilots.rs`. All builders come from the neutral
//! `floom_elements::prelude` facade so the page compiles under material /
//! cupertino / fluent / multi.

use floom::IntoView;
use std::collections::HashMap;
use floom::reactive::{RwSignal, SignalGet, SignalUpdate};
use floom::view::AnyView;
use floom::view::View;
use floom::views::{Decorators, Label, Stack, dyn_view};
use floom_core_theme::props::calendar::CalendarDate;
use floom_core_theme::props::date_picker::DateValue;
use floom_core_theme::props::icon_button::IconButtonVariant;
use floom_core_theme::props::skeleton_loader::SkeletonType;
use floom_core_theme::props::slide_transition::SlideDirection;
use floom_core_theme::props::speed_dial::SpeedDialDirection;
use floom_core_theme::props::time_picker::TimeValue;
use floom_core_theme::props::treeview::TreeviewNode;
use floom_elements::button::ButtonVariant;
use floom_elements::prelude::*;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_runtime::theme_signal;

use crate::showcase::{
    api_table, code_snippet, demo, demo_ctl, segmented, showcase, slider_int_row,
    toggle_row, ApiRow,
};

// ===========================================================================
// 1. Checkbox
// ===========================================================================

const CHECKBOX_SRC: &str = r#"let checked = RwSignal::new(true);
let disabled = RwSignal::new(false);

Checkbox::new("Enable notifications")
    .checked(checked.get())
    .disabled(disabled.get())
    .on_change(|v| log::info!("checked = {}", v))
    .view()"#;

fn checkbox_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(label)", ty: "impl Into<String>", default: "—", desc: "构造一个复选框，label 为右侧文字。" },
        ApiRow { name: "checked", ty: "bool", default: "false", desc: "是否选中。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用交互。" },
        ApiRow { name: "on_change", ty: "Fn(bool)", default: "—", desc: "选中态变化时的回调，参数为新值。" },
    ]
}

pub fn checkbox_showcase() -> AnyView {
    let checked = RwSignal::new(true);
    let disabled = RwSignal::new(false);
    let last = RwSignal::new(String::new());

    let comp = {
        let checked = checked;
        let disabled = disabled;
        let last = last;
        dyn_view(move || {
            Checkbox::new("Enable notifications")
                .checked(checked.get())
                .disabled(disabled.get())
                .on_change(move |v| last.set(format!("on_change → {}", v)))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Checked", checked),
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Last event: {}", last.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Checkbox",
        "A two-state selection control for toggling a single option on or off.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Checkbox::new("Unchecked").view()),
            demo("Checked", Checkbox::new("Checked").checked(true).view()),
            demo("Disabled", Checkbox::new("Disabled").disabled(true).view()),
            code_snippet("Checkbox · 基础", CHECKBOX_SRC),
            api_table("Props", checkbox_api()),
        ],
    )
}

// ===========================================================================
// 2. Radio
// ===========================================================================

const RADIO_SRC: &str = r#"Radio::new("Enabled, unchecked").view()
Radio::new("Enabled, checked").checked(true).view()
Radio::new("Disabled, unchecked").disabled(true).view()"#;

fn radio_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(label)", ty: "impl Into<String>", default: "—", desc: "构造单选项，label 为文字。" },
        ApiRow { name: "value", ty: "impl Into<String>", default: "label", desc: "该选项的取值（提交时回传）。" },
        ApiRow { name: "checked", ty: "bool", default: "false", desc: "是否选中。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(bool)", default: "—", desc: "选中态变化回调。" },
    ]
}

pub fn radio_showcase() -> AnyView {
    let disabled = RwSignal::new(false);
    let last = RwSignal::new(String::new());

    let comp = {
        let disabled = disabled;
        let last = last;
        dyn_view(move || {
            Stack::vertical((
                Radio::new("Apple").on_change(move |v| last.set(format!("Apple → {}", v))).view().into_any(),
                Radio::new("Banana").checked(true).on_change(move |v| last.set(format!("Banana → {}", v))).view().into_any(),
                Radio::new("Cherry").disabled(disabled.get()).on_change(move |v| last.set(format!("Cherry → {}", v))).view().into_any(),
            ))
            .style(|s| s.gap(6.0).items_start())
            .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Disable all", disabled),
        Label::derived(move || format!("Last event: {}", last.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Radio",
        "A single radio option. Group several inside RadioGroup for mutually-exclusive selection.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Unchecked", Radio::new("Unchecked").view()),
            demo("Checked", Radio::new("Checked").checked(true).view()),
            demo("Disabled", Radio::new("Disabled").disabled(true).view()),
            code_snippet("Radio · 基础", RADIO_SRC),
            api_table("Props", radio_api()),
        ],
    )
}

// ===========================================================================
// 3. Textarea
// ===========================================================================

const TEXTAREA_SRC: &str = r#"let text = RwSignal::new(String::new());
Textarea::new()
    .value("Edit me…".to_string())
    .rows(4)
    .placeholder("Type here…")
    .on_change(move |v| text.set(v))
    .view()"#;

fn textarea_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造多行文本输入框。" },
        ApiRow { name: "value", ty: "impl Into<String>", default: "空", desc: "初始文本。" },
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "—", desc: "空状态占位提示。" },
        ApiRow { name: "rows", ty: "usize", default: "由内容决定", desc: "可见行数（影响高度）。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(String)", default: "—", desc: "文本变化回调。" },
    ]
}

pub fn textarea_showcase() -> AnyView {
    let text = RwSignal::new(String::new());
    let disabled = RwSignal::new(false);
    let rows = RwSignal::new(4i32);

    let comp = {
        let text = text;
        let disabled = disabled;
        let rows = rows;
        dyn_view(move || {
            Textarea::new()
                .value("Edit me…".to_string())
                .rows(rows.get().max(1) as usize)
                .disabled(disabled.get())
                .on_change(move |v| text.set(v))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Chars: {}", text.get().len()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Textarea",
        "A multi-line text input for longer free-form content.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Textarea::new().value("Plain textarea".to_string()).view()),
            demo("Disabled", Textarea::new().value("Disabled".to_string()).disabled(true).view()),
            code_snippet("Textarea · 基础", TEXTAREA_SRC),
            api_table("Props", textarea_api()),
        ],
    )
}

// ===========================================================================
// 4. Tooltip
// ===========================================================================

const TOOLTIP_SRC: &str = r#"Tooltip::new()
    .text("Tooltip shown on hover")
    .child(Button::new("Hover me").view())
    .view()"#;

fn tooltip_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造提示气泡容器。" },
        ApiRow { name: "text", ty: "impl Into<String>", default: "—", desc: "提示文字。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "被悬浮触发提示的内容。" },
    ]
}

pub fn tooltip_showcase() -> AnyView {
    let comp = Tooltip::new()
        .text("Tooltip shown on hover")
        .child(Button::new("Hover me").variant(ButtonVariant::Secondary).view())
        .view();

    showcase(
        "Tooltip",
        "A small floating label that appears when the user hovers (or long-presses) its child.",
        vec![
            demo_ctl("Interactive", comp, Label::new("Hover the button to reveal the tooltip.").style(|s| s.font_size(13.0))),
            demo("On an icon", Tooltip::new().text("Settings").child(Icon::new("settings").view()).view()),
            code_snippet("Tooltip · 基础", TOOLTIP_SRC),
            api_table("Props", tooltip_api()),
        ],
    )
}

// ===========================================================================
// 5. Badge
// ===========================================================================

const BADGE_SRC: &str = r#"Badge::new()
    .content("3")
    .child(Icon::new("notifications").view())
    .view()
Badge::new().dot(true).child(Button::new("Dot").view()).view()"#;

fn badge_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造徽标。" },
        ApiRow { name: "content", ty: "impl Into<String>", default: "—", desc: "徽标内显示的文字（数量）。" },
        ApiRow { name: "dot", ty: "bool", default: "false", desc: "是否渲染为小圆点（无文字）。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "被标记的内容（图标/按钮）。" },
    ]
}

pub fn badge_showcase() -> AnyView {
    let comp = Stack::horizontal((
        Badge::new().content("3").child(Icon::new("notifications").view()).view().into_any(),
        Badge::new().content("12").child(Button::new("Mail").view()).view().into_any(),
        Badge::new().dot(true).child(Button::new("Dot").view()).view().into_any(),
    ))
    .style(|s| s.gap(16.0).items_center());

    showcase(
        "Badge",
        "A small status descriptor for icons and buttons — counts, dots, or short labels.",
        vec![
            demo_ctl("Interactive", comp, Label::new("Badges overlay their child's top-right corner.").style(|s| s.font_size(13.0))),
            demo("Dot only", Badge::new().dot(true).child(Icon::new("favorite").view()).view()),
            code_snippet("Badge · 基础", BADGE_SRC),
            api_table("Props", badge_api()),
        ],
    )
}

// ===========================================================================
// 6. Avatar
// ===========================================================================

const AVATAR_SRC: &str = r#"Avatar::new()
    .content("AB".to_string())
    .size(48.0)
    .view()"#;

fn avatar_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造头像。" },
        ApiRow { name: "content", ty: "impl Into<String>", default: "—", desc: "头像内文字（通常是首字母）。" },
        ApiRow { name: "size", ty: "f32", default: "40.0", desc: "直径（像素）。" },
    ]
}

pub fn avatar_showcase() -> AnyView {
    let size = RwSignal::new(48i32);

    let comp = {
        let size = size;
        dyn_view(move || {
            Avatar::new()
                .content("AB".to_string())
                .size(size.get() as f32)
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Size (24–72)", 24, 72, size),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Avatar",
        "A circular element representing a user, typically showing initials or an image.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Size 40", Avatar::new().content("CD".to_string()).size(40.0).view()),
            demo("Size 32", Avatar::new().content("EF".to_string()).size(32.0).view()),
            code_snippet("Avatar · 基础", AVATAR_SRC),
            api_table("Props", avatar_api()),
        ],
    )
}

// ===========================================================================
// 7. ProgressLinear
// ===========================================================================

const PROGRESS_LINEAR_SRC: &str = r#"ProgressLinear::new()
    .value(65.0)
    .view()
ProgressLinear::new().indeterminate(true).view()"#;

fn progress_linear_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造线性进度条。" },
        ApiRow { name: "value", ty: "f32", default: "0.0", desc: "进度值，范围 0–100。" },
        ApiRow { name: "indeterminate", ty: "bool", default: "false", desc: "是否无限循环（未知时长）。" },
    ]
}

pub fn progress_linear_showcase() -> AnyView {
    let value = RwSignal::new(65i32);

    let comp = {
        let value = value;
        dyn_view(move || {
            ProgressLinear::new()
                .value(value.get() as f32)
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Value (0–100)", 0, 100, value),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ProgressLinear",
        "A horizontal bar that communicates the progress of a task.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Indeterminate", ProgressLinear::new().indeterminate(true).view()),
            code_snippet("ProgressLinear · 基础", PROGRESS_LINEAR_SRC),
            api_table("Props", progress_linear_api()),
        ],
    )
}

// ===========================================================================
// 8. ProgressCircular
// ===========================================================================

const PROGRESS_CIRCULAR_SRC: &str = r#"ProgressCircular::new()
    .value(40.0)
    .view()
ProgressCircular::new().indeterminate(true).view()"#;

fn progress_circular_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造圆形进度环。" },
        ApiRow { name: "value", ty: "f32", default: "0.0", desc: "进度值，范围 0–100。" },
        ApiRow { name: "indeterminate", ty: "bool", default: "false", desc: "是否无限循环。" },
        ApiRow { name: "size", ty: "f32", default: "由主题决定", desc: "直径（像素）。" },
    ]
}

pub fn progress_circular_showcase() -> AnyView {
    let value = RwSignal::new(40i32);

    let comp = {
        let value = value;
        dyn_view(move || {
            ProgressCircular::new()
                .value(value.get() as f32)
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Value (0–100)", 0, 100, value),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ProgressCircular",
        "A circular indicator that communicates the progress of a task.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Indeterminate", ProgressCircular::new().indeterminate(true).view()),
            code_snippet("ProgressCircular · 基础", PROGRESS_CIRCULAR_SRC),
            api_table("Props", progress_circular_api()),
        ],
    )
}

// ===========================================================================
// 9. Banner
// ===========================================================================

const BANNER_SRC: &str = r#"Banner::new()
    .variant(BannerVariant::Info)
    .message("No connection — changes will sync later.")
    .action("Retry")
    .on_action(|| {})
    .view()"#;

fn banner_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造横幅。" },
        ApiRow { name: "variant", ty: "BannerVariant", default: "Info", desc: "语义色调：Info / Warning / Error / Success。" },
        ApiRow { name: "message", ty: "impl Into<String>", default: "—", desc: "主文案。" },
        ApiRow { name: "icon", ty: "impl Into<String>", default: "—", desc: "前置图标名。" },
        ApiRow { name: "action", ty: "impl Into<String>", default: "—", desc: "右侧操作按钮文字。" },
        ApiRow { name: "on_action", ty: "Fn()", default: "—", desc: "操作按钮点击回调。" },
    ]
}

pub fn banner_showcase() -> AnyView {
    let variant = RwSignal::new(BannerVariant::Info);

    let comp = {
        let variant = variant;
        dyn_view(move || {
            Banner::new()
                .variant(variant.get())
                .message("This is a banner message for the current state.")
                .action("Action")
                .on_action(|| {})
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        segmented(
            "Variant",
            vec![
                ("Info", BannerVariant::Info),
                ("Warning", BannerVariant::Warning),
                ("Error", BannerVariant::Error),
                ("Success", BannerVariant::Success),
            ],
            variant,
        ),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Banner",
        "A prominent, dismissible strip for high-signal messages at the top of a screen.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("With icon", Banner::new().variant(BannerVariant::Success).icon("check_circle").message("Upload complete.").view()),
            code_snippet("Banner · 基础", BANNER_SRC),
            api_table("Props", banner_api()),
        ],
    )
}

// ===========================================================================
// 10. Pagination
// ===========================================================================

const PAGINATION_SRC: &str = r#"let page = RwSignal::new(1usize);
Pagination::new()
    .total(7)
    .current(1)
    .on_change(move |p| page.set(p))
    .view()"#;

fn pagination_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造分页器。" },
        ApiRow { name: "total", ty: "usize", default: "0", desc: "总页数。" },
        ApiRow { name: "current", ty: "usize", default: "0", desc: "当前页（从 0 或 1 起，取决于主题约定）。" },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "翻页回调，参数为目标页。" },
    ]
}

pub fn pagination_showcase() -> AnyView {
    let page = RwSignal::new(1usize);

    let comp = {
        let page = page;
        dyn_view(move || {
            Pagination::new()
                .total(7)
                .current(page.get())
                .on_change(move |p| page.set(p))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        Label::derived(move || format!("Current page: {}", page.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Pagination",
        "A control for splitting content into pages and navigating between them.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Many pages", Pagination::new().total(20).current(5).on_change(|_| {}).view()),
            code_snippet("Pagination · 基础", PAGINATION_SRC),
            api_table("Props", pagination_api()),
        ],
    )
}

// ===========================================================================
// 11. Divider
// ===========================================================================

const DIVIDER_SRC: &str = r#"Stack::vertical((
    Text::new("Above the divider").view(),
    Divider::new().view(),
    Text::new("Below the divider").view(),
))
.style(|s| s.gap(6.0).width(360.0).items_start())"#;

fn divider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造分隔线（无配置属性）。" },
    ]
}

pub fn divider_showcase() -> AnyView {
    let comp = Stack::vertical((
        Text::new("Above the divider").view(),
        Divider::new().view(),
        Text::new("Below the divider").view(),
    ))
    .style(|s| s.gap(6.0).width(360.0).items_start());

    showcase(
        "Divider",
        "A thin line that groups content and separates sections.",
        vec![
            demo_ctl("Interactive", comp, Label::new("Drop a Divider between two blocks of content.").style(|s| s.font_size(13.0))),
            code_snippet("Divider · 用法", DIVIDER_SRC),
            api_table("Props", divider_api()),
        ],
    )
}

// ===========================================================================
// 12. Spacer
// ===========================================================================

const SPACER_SRC: &str = r#"Stack::horizontal((
    Text::new("Left").view(),
    Spacer::new().grow(1.0).view(),
    Text::new("Right").view(),
))
.style(|s| s.gap(8.0))"#;

fn spacer_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造占位空白（无配置属性）。" },
        ApiRow { name: "grow", ty: "f32", default: "0.0", desc: "在主轴方向占满剩余空间的权重。" },
    ]
}

pub fn spacer_showcase() -> AnyView {
    let comp = Stack::horizontal((
        Text::new("Left").view().into_any(),
        Spacer::new().grow(1.0).view().into_any(),
        Text::new("Right").view().into_any(),
    ))
    .style(|s| s.gap(8.0).width(360.0));

    showcase(
        "Spacer",
        "An empty flexible box that pushes siblings apart inside a flex layout.",
        vec![
            demo_ctl("Interactive", comp, Label::new("Spacer.grow(1.0) absorbs the free space between Left and Right.").style(|s| s.font_size(13.0))),
            code_snippet("Spacer · 用法", SPACER_SRC),
            api_table("Props", spacer_api()),
        ],
    )
}

// ===========================================================================
// 13. Counter
// ===========================================================================

const COUNTER_SRC: &str = r#"let value = RwSignal::new(3i64);
Counter::new()
    .value(value.get())
    .min(0)
    .max(10)
    .step(1)
    .on_change(move |v| value.set(v))
    .view()"#;

fn counter_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造计数器。" },
        ApiRow { name: "value", ty: "i64", default: "0", desc: "当前值。" },
        ApiRow { name: "min", ty: "i64", default: "i64::MIN", desc: "下限。" },
        ApiRow { name: "max", ty: "i64", default: "i64::MAX", desc: "上限。" },
        ApiRow { name: "step", ty: "i64", default: "1", desc: "每次增减步长。" },
        ApiRow { name: "on_change", ty: "Fn(i64)", default: "—", desc: "值变化回调，参数为新值。" },
    ]
}

pub fn counter_showcase() -> AnyView {
    let value = RwSignal::new(3i32);
    let min = RwSignal::new(0i32);
    let max = RwSignal::new(10i32);
    let step = RwSignal::new(1i32);

    let comp = {
        let value = value;
        let min = min;
        let max = max;
        let step = step;
        dyn_view(move || {
            Counter::new()
                .value(value.get() as i64)
                .min(min.get() as i64)
                .max(max.get() as i64)
                .step(step.get() as i64)
                .on_change(move |v| value.set(v as i32))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Value", 0, 20, value),
        slider_int_row("Min", -5, 10, min),
        slider_int_row("Max", 5, 30, max),
        slider_int_row("Step", 1, 5, step),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Counter",
        "An increment/decrement stepper for bounded integer values.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Counter::new().view()),
            demo("Bounded", Counter::new().value(5).min(0).max(10).view()),
            code_snippet("Counter · 基础", COUNTER_SRC),
            api_table("Props", counter_api()),
        ],
    )
}

// ===========================================================================
// 14. NumberInput
// ===========================================================================

const NUMBER_INPUT_SRC: &str = r#"let value = RwSignal::new(5i64);
NumberInput::new()
    .value(value.get())
    .min(0)
    .max(10)
    .step(1)
    .label("Quantity")
    .on_change(move |v| value.set(v))
    .view()"#;

fn number_input_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造数字输入框。" },
        ApiRow { name: "value", ty: "i64", default: "0", desc: "当前数值。" },
        ApiRow { name: "min", ty: "i64", default: "i64::MIN", desc: "下限。" },
        ApiRow { name: "max", ty: "i64", default: "i64::MAX", desc: "上限。" },
        ApiRow { name: "step", ty: "i64", default: "1", desc: "步进。" },
        ApiRow { name: "label", ty: "impl Into<String>", default: "—", desc: "字段标签。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(i64)", default: "—", desc: "数值变化回调。" },
    ]
}

pub fn number_input_showcase() -> AnyView {
    let value = RwSignal::new(5i32);
    let min = RwSignal::new(0i32);
    let max = RwSignal::new(10i32);
    let step = RwSignal::new(1i32);
    let disabled = RwSignal::new(false);

    let comp = {
        let value = value;
        let min = min;
        let max = max;
        let step = step;
        let disabled = disabled;
        dyn_view(move || {
            NumberInput::new()
                .value(value.get() as i64)
                .min(min.get() as i64)
                .max(max.get() as i64)
                .step(step.get() as i64)
                .label("Quantity")
                .disabled(disabled.get())
                .on_change(move |v| value.set(v as i32))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Value", 0, 20, value),
        slider_int_row("Min", -5, 10, min),
        slider_int_row("Max", 5, 30, max),
        slider_int_row("Step", 1, 5, step),
        toggle_row("Disabled", disabled),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "NumberInput",
        "A text field restricted to numeric entry, with stepper affordances.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", NumberInput::new().view()),
            demo("With label", NumberInput::new().value(5).label("Quantity").view()),
            demo("Disabled", NumberInput::new().value(5).disabled(true).view()),
            code_snippet("NumberInput · 基础", NUMBER_INPUT_SRC),
            api_table("Props", number_input_api()),
        ],
    )
}

// ===========================================================================
// 15. RangeSlider
// ===========================================================================

const RANGE_SLIDER_SRC: &str = r#"let low = RwSignal::new(25.0f32);
let high = RwSignal::new(75.0f32);
RangeSlider::new()
    .low(low.get())
    .high(high.get())
    .min(0.0)
    .max(100.0)
    .step(1.0)
    .on_change(move |(l, h)| { low.set(l); high.set(h); })
    .view()"#;

fn range_slider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造双滑块范围选择器。" },
        ApiRow { name: "low", ty: "f32", default: "0.0", desc: "下界值。" },
        ApiRow { name: "high", ty: "f32", default: "100.0", desc: "上界值。" },
        ApiRow { name: "min", ty: "f32", default: "0.0", desc: "范围下限。" },
        ApiRow { name: "max", ty: "f32", default: "100.0", desc: "范围上限。" },
        ApiRow { name: "step", ty: "f32", default: "1.0", desc: "步进。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn((f32, f32))", default: "—", desc: "范围变化回调，参数为 (low, high)。" },
        ApiRow { name: "low_signal / high_signal", ty: "RwSignal<f32>", default: "—", desc: "暴露内部信号，可外部绑定。" },
    ]
}

pub fn range_slider_showcase() -> AnyView {
    let low = RwSignal::new(25i32);
    let high = RwSignal::new(75i32);
    let disabled = RwSignal::new(false);

    let comp = {
        let low = low;
        let high = high;
        let disabled = disabled;
        dyn_view(move || {
            RangeSlider::new()
                .low(low.get() as f32)
                .high(high.get() as f32)
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .disabled(disabled.get())
                .on_change(move |(l, h)| {
                    low.set(l as i32);
                    high.set(h as i32);
                })
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Low", 0, 100, low),
        slider_int_row("High", 0, 100, high),
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Current: {}–{}", low.get(), high.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "RangeSlider",
        "A dual-thumb control for selecting a range between a minimum and maximum.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", RangeSlider::new().low(25.0).high(75.0).min(0.0).max(100.0).view()),
            demo("Wide", RangeSlider::new().low(10.0).high(90.0).min(0.0).max(100.0).view()),
            code_snippet("RangeSlider · 基础", RANGE_SLIDER_SRC),
            api_table("Props", range_slider_api()),
        ],
    )
}

// ===========================================================================
// 16. RadioGroup
// ===========================================================================

const RADIO_GROUP_SRC: &str = r#"let selected = RwSignal::new(0usize);
RadioGroup::new()
    .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
    .selected(Some(0))
    .on_change(move |i| selected.set(i))
    .view()"#;

fn radio_group_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造单选组。" },
        ApiRow { name: "option", ty: "impl Into<String>", default: "—", desc: "追加单个选项（链式）。" },
        ApiRow { name: "options", ty: "Vec<String>", default: "空", desc: "一次性设置所有选项。" },
        ApiRow { name: "selected", ty: "Option<usize>", default: "None", desc: "预选中的下标。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否整体禁用。" },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "选中项变化回调，参数为下标。" },
    ]
}

pub fn radio_group_showcase() -> AnyView {
    let selected = RwSignal::new(0usize);
    let disabled = RwSignal::new(false);

    let comp = {
        let selected = selected;
        let disabled = disabled;
        dyn_view(move || {
            RadioGroup::new()
                .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
                .selected(Some(selected.get()))
                .disabled(disabled.get())
                .on_change(move |i| selected.set(i))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Selected index: {}", selected.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "RadioGroup",
        "A set of mutually-exclusive options; exactly one can be selected at a time.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Single", RadioGroup::new().options(vec!["One".into(), "Two".into(), "Three".into()]).view()),
            demo("Preselected", RadioGroup::new().options(vec!["A".into(), "B".into()]).selected(Some(1)).view()),
            code_snippet("RadioGroup · 基础", RADIO_GROUP_SRC),
            api_table("Props", radio_group_api()),
        ],
    )
}

// ===========================================================================
// 17. ListItem
// ===========================================================================

const LIST_ITEM_SRC: &str = r#"ListItem::new()
    .title("Inbox")
    .subtitle("12 new messages")
    .leading(Icon::new("email").view())
    .trailing(Icon::new("chevron_right").view())
    .nav(true)
    .on_click(|| {})
    .view()"#;

fn list_item_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造列表项。" },
        ApiRow { name: "title", ty: "impl Into<String>", default: "—", desc: "主标题。" },
        ApiRow { name: "subtitle", ty: "impl Into<String>", default: "—", desc: "副标题。" },
        ApiRow { name: "leading", ty: "impl IntoView", default: "—", desc: "前置内容（图标等）。" },
        ApiRow { name: "trailing", ty: "impl IntoView", default: "—", desc: "后置内容。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "nav", ty: "bool", default: "false", desc: "是否呈现导航箭头。" },
        ApiRow { name: "on_click", ty: "Fn()", default: "—", desc: "点击回调。" },
    ]
}

pub fn list_item_showcase() -> AnyView {
    let disabled = RwSignal::new(false);
    let nav = RwSignal::new(true);

    let comp = {
        let disabled = disabled;
        let nav = nav;
        dyn_view(move || {
            Stack::vertical((
                ListItem::new()
                    .title("Inbox")
                    .subtitle("12 new messages")
                    .leading(Icon::new("email").view())
                    .trailing(Icon::new("chevron_right").view())
                    .nav(nav.get())
                    .disabled(disabled.get())
                    .on_click(|| {})
                    .view()
                    .into_any(),
                ListItem::new()
                    .title("Sent")
                    .subtitle("Yesterday")
                    .leading(Icon::new("send").view())
                    .nav(nav.get())
                    .view()
                    .into_any(),
            ))
            .style(|s| s.gap(0.0).items_start())
            .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Nav arrow", nav),
        toggle_row("Disabled", disabled),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ListItem",
        "A single row in a list, optionally with leading/trailing content and a navigation affordance.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("With subtitle", ListItem::new().title("Title").subtitle("Subtitle").leading(Icon::new("star").view()).view()),
            demo("Trailing text", ListItem::new().title("Wi-Fi").trailing(Text::new("100%").view()).view()),
            code_snippet("ListItem · 基础", LIST_ITEM_SRC),
            api_table("Props", list_item_api()),
        ],
    )
}

// ===========================================================================
// 18. ChipGroup
// ===========================================================================

const CHIP_GROUP_SRC: &str = r#"let selected = RwSignal::new(Vec::<usize>::new());
ChipGroup::new()
    .chip("Apple")
    .chip("Banana")
    .chip("Cherry")
    .multiple(true)
    .on_change(move |idxs| selected.set(idxs))
    .view()"#;

fn chip_group_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造筛选芯片组。" },
        ApiRow { name: "chip", ty: "impl Into<String>", default: "—", desc: "追加一个芯片。" },
        ApiRow { name: "multiple", ty: "bool", default: "false", desc: "是否允许多选。" },
        ApiRow { name: "mandatory", ty: "bool", default: "false", desc: "是否至少选中一个（不可全部取消）。" },
        ApiRow { name: "on_change", ty: "Fn(Vec<usize>)", default: "—", desc: "选中集合变化回调，参数为下标列表。" },
    ]
}

pub fn chip_group_showcase() -> AnyView {
    let selected = RwSignal::new(Vec::<usize>::new());
    let multiple = RwSignal::new(true);
    let mandatory = RwSignal::new(false);

    let comp = {
        let selected = selected;
        let multiple = multiple;
        let mandatory = mandatory;
        dyn_view(move || {
            ChipGroup::new()
                .chip("Apple")
                .chip("Banana")
                .chip("Cherry")
                .chip("Date")
                .multiple(multiple.get())
                .mandatory(mandatory.get())
                .on_change(move |idxs| selected.set(idxs))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Multiple", multiple),
        toggle_row("Mandatory", mandatory),
        Label::derived(move || format!("Selected indices: {:?}", selected.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ChipGroup",
        "A compact set of filter chips; single or multiple selection.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Single select", ChipGroup::new().chip("Red").chip("Green").chip("Blue").view()),
            demo("Multiple", ChipGroup::new().chip("A").chip("B").chip("C").chip("D").multiple(true).view()),
            code_snippet("ChipGroup · 基础", CHIP_GROUP_SRC),
            api_table("Props", chip_group_api()),
        ],
    )
}

// ===========================================================================
// 19. ListGroup
// ===========================================================================

const LIST_GROUP_SRC: &str = r#"let expanded = RwSignal::new(true);
ListGroup::new()
    .title("Fruits")
    .item("Apple")
    .item("Banana")
    .item("Cherry")
    .expanded(expanded.get())
    .on_toggle(move |e| expanded.set(e))
    .view()"#;

fn list_group_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造可展开列表组。" },
        ApiRow { name: "item", ty: "impl Into<String>", default: "—", desc: "追加组内条目。" },
        ApiRow { name: "title", ty: "impl Into<String>", default: "—", desc: "组标题。" },
        ApiRow { name: "expanded", ty: "bool", default: "false", desc: "初始是否展开。" },
        ApiRow { name: "inset", ty: "bool", default: "false", desc: "子项是否内缩。" },
        ApiRow { name: "on_toggle", ty: "Fn(bool)", default: "—", desc: "展开/折叠切换回调。" },
    ]
}

pub fn list_group_showcase() -> AnyView {
    let expanded = RwSignal::new(true);
    let inset = RwSignal::new(false);

    let comp = {
        let expanded = expanded;
        let inset = inset;
        dyn_view(move || {
            ListGroup::new()
                .title("Fruits")
                .item("Apple")
                .item("Banana")
                .item("Cherry")
                .expanded(expanded.get())
                .inset(inset.get())
                .on_toggle(move |e| expanded.set(e))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Expanded", expanded),
        toggle_row("Inset", inset),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ListGroup",
        "A collapsible group of list items under a titled header.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Collapsed", ListGroup::new().title("Fruits").item("Apple").item("Banana").expanded(false).view()),
            demo("Inset", ListGroup::new().title("Fruits").item("Apple").item("Banana").inset(true).view()),
            code_snippet("ListGroup · 基础", LIST_GROUP_SRC),
            api_table("Props", list_group_api()),
        ],
    )
}

// ===========================================================================
// 20. ListSubheader
// ===========================================================================

const LIST_SUBHEADER_SRC: &str = r#"ListSubheader::new("Section A").view()
ListSubheader::new("Section B").inset(true).view()"#;

fn list_subheader_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(text)", ty: "impl Into<String>", default: "—", desc: "构造小标题，参数为标题文字。" },
        ApiRow { name: "inset", ty: "bool", default: "false", desc: "是否内缩对齐。" },
    ]
}

pub fn list_subheader_showcase() -> AnyView {
    let inset = RwSignal::new(false);

    let comp = {
        let inset = inset;
        dyn_view(move || {
            Stack::vertical((
                ListSubheader::new("Section A").view().into_any(),
                ListSubheader::new("Section B").inset(inset.get()).view().into_any(),
            ))
            .style(|s| s.gap(0.0).items_start())
            .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Inset second", inset),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ListSubheader",
        "A small, uppercased-style heading that separates sections within a list.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Two sections",
                Stack::vertical((
                    ListSubheader::new("Today").view().into_any(),
                    ListSubheader::new("Earlier").inset(true).view().into_any(),
                ))
                .style(|s| s.gap(0.0))
                .into_any(),
            ),
            code_snippet("ListSubheader · 用法", LIST_SUBHEADER_SRC),
            api_table("Props", list_subheader_api()),
        ],
    )
}

// ===========================================================================
// 21. Container
// ===========================================================================

const CONTAINER_SRC: &str = r#"Container::new()
    .child(Text::new("Centered content").view())
    .max_width(420.0)
    .padding(16.0)
    .view()"#;

fn container_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造内容容器。" },
        ApiRow { name: "max_width", ty: "f32", default: "由主题决定", desc: "最大内容宽度（像素）。" },
        ApiRow { name: "padding", ty: "f32", default: "0.0", desc: "内边距。" },
        ApiRow { name: "fluid", ty: "bool", default: "false", desc: "是否撑满父容器（忽略 max_width）。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "唯一子内容。" },
    ]
}

pub fn container_showcase() -> AnyView {
    let max_width = RwSignal::new(420i32);
    let padding = RwSignal::new(16i32);
    let fluid = RwSignal::new(false);

    let comp = {
        let max_width = max_width;
        let padding = padding;
        let fluid = fluid;
        dyn_view(move || {
            Container::new()
                .child(
                    Stack::vertical((
                        Text::new("Inside a Container").view().into_any(),
                        Text::new("Content is capped and centered.").view().into_any(),
                    ))
                    .style(|s| s.gap(4.0)),
                )
                .max_width(max_width.get() as f32)
                .padding(padding.get() as f32)
                .fluid(fluid.get())
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Max width", 200, 640, max_width),
        slider_int_row("Padding", 0, 48, padding),
        toggle_row("Fluid", fluid),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Container",
        "A responsive content wrapper that centers and caps its child's width.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Container::new().child(Text::new("Default container").view()).view()),
            demo("Fluid", Container::new().fluid(true).child(Text::new("Fluid container").view()).view()),
            code_snippet("Container · 基础", CONTAINER_SRC),
            api_table("Props", container_api()),
        ],
    )
}

// ===========================================================================
// 22. Grid
// ===========================================================================

const GRID_SRC: &str = r#"let theme = theme_signal();
let cell = |_i: usize| {
    floom::views::Container::new(floom::views::Empty::new())
        .style(move |s| s.height(40.0).background(theme.get().colors().primary_container.to_peniko()))
};
Grid::new()
    .cols(3)
    .gap(8.0)
    .row_gap(8.0)
    .child(cell(0))
    .child(cell(1))
    .child(cell(2))
    .view()"#;

fn grid_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造网格布局。" },
        ApiRow { name: "cols", ty: "usize", default: "1", desc: "列数。" },
        ApiRow { name: "gap", ty: "f32", default: "0.0", desc: "列间距。" },
        ApiRow { name: "row_gap", ty: "f32", default: "gap", desc: "行间距（缺省等于 gap）。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "追加一个网格单元。" },
    ]
}

fn grid_static(cols: usize, gap: f32) -> impl IntoView {
    let theme = theme_signal();
    let cell = move |_i: usize| {
        floom::views::Container::new(floom::views::Empty::new()).style(move |s| {
            s.height(40.0)
                .background(floom_core_theme::theme::Theme::colors(&theme.get()).primary_container.to_peniko())
        })
    };
    let mut grid = Grid::new().cols(cols).gap(gap).row_gap(gap);
    for _ in 0..(cols * 2) {
        grid = grid.child(cell(0));
    }
    grid.view()
}

pub fn grid_showcase() -> AnyView {
    let cols = RwSignal::new(3i32);
    let gap = RwSignal::new(8i32);

    let comp = {
        let cols = cols;
        let gap = gap;
        dyn_view(move || {
            let theme = theme_signal();
            let c = cols.get().max(1) as usize;
            let g = gap.get() as f32;
            let cell = move |_i: usize| {
                floom::views::Container::new(floom::views::Empty::new()).style(move |s| {
                    s.height(40.0)
                        .background(floom_core_theme::theme::Theme::colors(&theme.get()).primary_container.to_peniko())
                })
            };
            let mut grid = Grid::new().cols(c).gap(g).row_gap(g);
            for _ in 0..(c * 2) {
                grid = grid.child(cell(0));
            }
            grid.view().into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Columns", 1, 6, cols),
        slider_int_row("Gap", 0, 24, gap),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Grid",
        "A regular 2-D layout that places children into a fixed number of columns.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("2 columns", grid_static(2, 8.0)),
            demo("4 columns", grid_static(4, 4.0)),
            code_snippet("Grid · 基础", GRID_SRC),
            api_table("Props", grid_api()),
        ],
    )
}

// ===========================================================================
// 23. OtpInput
// ===========================================================================

const OTP_INPUT_SRC: &str = r#"let code = RwSignal::new(String::new());
OtpInput::new()
    .length(6)
    .masked(false)
    .on_change(move |c| code.set(c))
    .view()"#;

fn otp_input_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造一次性密码输入框。" },
        ApiRow { name: "length", ty: "usize", default: "由主题决定", desc: "输入框数量（位数）。" },
        ApiRow { name: "masked", ty: "bool", default: "false", desc: "是否以圆点遮罩显示。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(String)", default: "—", desc: "输入完成回调，参数为完整验证码。" },
        ApiRow { name: "code_signal", ty: "RwSignal<String>", default: "—", desc: "暴露内部验证码信号。" },
    ]
}

pub fn otp_input_showcase() -> AnyView {
    let length = RwSignal::new(6i32);
    let masked = RwSignal::new(false);
    let disabled = RwSignal::new(false);
    let code = RwSignal::new(String::new());

    let comp = {
        let length = length;
        let masked = masked;
        let disabled = disabled;
        let code = code;
        dyn_view(move || {
            OtpInput::new()
                .length(length.get().max(1) as usize)
                .masked(masked.get())
                .disabled(disabled.get())
                .on_change(move |c| code.set(c))
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        slider_int_row("Length", 4, 8, length),
        toggle_row("Masked", masked),
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Code: {}", code.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "OtpInput",
        "A segmented field for entering a one-time passcode, character by character.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Masked", OtpInput::new().length(4).masked(true).view()),
            demo("Disabled", OtpInput::new().length(6).disabled(true).view()),
            code_snippet("OtpInput · 基础", OTP_INPUT_SRC),
            api_table("Props", otp_input_api()),
        ],
    )
}

// ===========================================================================
// 24. FileInput
// ===========================================================================

const FILE_INPUT_SRC: &str = r#"let path = RwSignal::new(None::<String>);
let input = FileInput::new().label("Upload");
let sig = input.path_signal();
input
    .on_change(move |p: Option<String>| sig.set(p))
    .view()"#;

fn file_input_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造文件选择输入。" },
        ApiRow { name: "label", ty: "impl Into<String>", default: "—", desc: "触发按钮/区域文字。" },
        ApiRow { name: "accept", ty: "impl Into<String>", default: "—", desc: "accept 过滤（如 \".pdf,image/*\"）。" },
        ApiRow { name: "multiple", ty: "bool", default: "false", desc: "是否允许多选。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(Option<String>)", default: "—", desc: "选择结果回调，参数为路径（可空）。" },
        ApiRow { name: "path_signal", ty: "RwSignal<Option<String>>", default: "—", desc: "暴露内部路径信号。" },
    ]
}

pub fn file_input_showcase() -> AnyView {
    let multiple = RwSignal::new(false);
    let disabled = RwSignal::new(false);
    let path = RwSignal::new(String::new());

    let comp = {
        let multiple = multiple;
        let disabled = disabled;
        let path = path;
        dyn_view(move || {
            let input = FileInput::new().label("Upload").accept(".pdf,image/*");
            let sig = input.path_signal();
            input
                .multiple(multiple.get())
                .disabled(disabled.get())
                .on_change(move |p: Option<String>| {
                    sig.set(p.clone());
                    if let Some(p) = p {
                        path.set(p);
                    }
                })
                .view()
                .into_any()
        })
    };

    let controls = Stack::vertical((
        toggle_row("Multiple", multiple),
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Selected: {}", path.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "FileInput",
        "A button or drop zone that opens a file picker and reports the chosen path.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Simple", FileInput::new().label("Choose file").view()),
            demo("Accept images", FileInput::new().label("Image").accept("image/*").view()),
            code_snippet("FileInput · 基础", FILE_INPUT_SRC),
            api_table("Props", file_input_api()),
        ],
    )
}

// ===========================================================================
// 25. Text
// ===========================================================================

const TEXT_SRC: &str = r#"Text::new("The quick brown fox jumps over the lazy dog.").view()"#;

fn text_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(content)", ty: "impl Into<String>", default: "—", desc: "构造一段文本，content 为文字内容。" },
        ApiRow { name: "max_lines", ty: "usize", default: "—", desc: "限制最大行数。仅单 DS 构建可用，multi 运行时分发未转发该 setter，故本页以 API 表说明、不演示。" },
        ApiRow { name: "selectable", ty: "bool", default: "false", desc: "是否允许用户选中复制。同上，multi 下未转发，本页不演示。" },
    ]
}

pub fn text_showcase() -> AnyView {
    let variant = RwSignal::new(0usize);
    let presets = [
        "The quick brown fox jumps over the lazy dog.",
        "Short label",
        "A longer paragraph that wraps across multiple lines to demonstrate inline flow, typographic rhythm and the Material baseline grid in a realistic content block.",
    ];
    let comp = dyn_view(move || Text::new(presets[variant.get()]).view().into_any());
    let controls = Stack::vertical((
        segmented(
            "Variant",
            vec![("Body", 0usize), ("Label", 1usize), ("Paragraph", 2usize)],
            variant,
        ),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Text",
        "The fundamental typographic primitive — paragraphs, labels and inline copy.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Body", Text::new(presets[0]).view()),
            demo("Label", Text::new(presets[1]).view()),
            code_snippet("Text · 基础", TEXT_SRC),
            api_table("Props", text_api()),
        ],
    )
}

// ===========================================================================
// 26. Icon
// ===========================================================================

const ICON_SRC: &str = r#"Icon::new("favorite").size(32.0).label("Favorite").view()"#;

fn icon_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(name)", ty: "impl Into<String>", default: "—", desc: "构造图标，name 为 material 图标名。" },
        ApiRow { name: "size", ty: "f32", default: "24.0", desc: "像素尺寸。" },
        ApiRow { name: "label", ty: "impl Into<String>", default: "—", desc: "无障碍标签；设置后会关闭 decorative。" },
        ApiRow { name: "decorative", ty: "bool", default: "true", desc: "是否为纯装饰图标（screen-reader 忽略）。" },
    ]
}

pub fn icon_showcase() -> AnyView {
    let size = RwSignal::new(32i32);
    let show_label = RwSignal::new(false);
    let comp = dyn_view(move || {
        let icon = Icon::new("favorite")
            .size(size.get() as f32)
            .decorative(!show_label.get());
        let icon = if show_label.get() {
            icon.label("Favorite")
        } else {
            icon
        };
        icon.view().into_any()
    });
    let controls = Stack::vertical((
        slider_int_row("Size", 16, 64, size),
        toggle_row("Show label", show_label),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Icon",
        "A scalable vector glyph with an optional accessible label.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Icon::new("favorite").view()),
            demo("Large", Icon::new("star").size(48.0).view()),
            code_snippet("Icon · 基础", ICON_SRC),
            api_table("Props", icon_api()),
        ],
    )
}

// ===========================================================================
// 27. List
// ===========================================================================

const LIST_SRC: &str = r#"List::new()
    .item("Inbox", || {})
    .item("Starred", || {})
    .view()"#;

fn list_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造列表容器。" },
        ApiRow { name: "item", ty: "impl Into<String>, Fn() + 'static", default: "—", desc: "追加一项，label 为文字，回调在点击时触发。" },
        ApiRow { name: "items", ty: "Vec<MenuItem>", default: "—", desc: "批量追加项。" },
    ]
}

pub fn list_showcase() -> AnyView {
    let picked = RwSignal::new(String::from("—"));
    let list = List::new()
        .item("Inbox", {
            let picked = picked;
            move || picked.set("Inbox".into())
        })
        .item("Starred", {
            let picked = picked;
            move || picked.set("Starred".into())
        })
        .item("Sent", {
            let picked = picked;
            move || picked.set("Sent".into())
        })
        .item("Drafts", {
            let picked = picked;
            move || picked.set("Drafts".into())
        })
        .view();
    let controls = Stack::vertical((
        Label::derived(move || format!("Selected item: {}", picked.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "List",
        "A vertical stack of selectable rows — the basis for menus and navigation.",
        vec![
            demo_ctl("Interactive (click a row)", list, controls),
            demo("Static", List::new().item("One", || {}).item("Two", || {}).view()),
            code_snippet("List · 基础", LIST_SRC),
            api_table("Props", list_api()),
        ],
    )
}

// ===========================================================================
// 28. Toolbar
// ===========================================================================

const TOOLBAR_SRC: &str = r#"Toolbar::new()
    .title("Gallery")
    .elevation(1)
    .height(56.0)
    .leading("menu")
    .actions(Icon::new("more_vert").view())
    .view()"#;

fn toolbar_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造工具栏。" },
        ApiRow { name: "title", ty: "impl Into<String>", default: "—", desc: "标题文字。" },
        ApiRow { name: "leading", ty: "impl Into<String>", default: "—", desc: "左侧图标名。" },
        ApiRow { name: "elevation", ty: "u8", default: "0", desc: "阴影高度（0–4 常用）。" },
        ApiRow { name: "height", ty: "f32", default: "56.0", desc: "工具栏高度。" },
        ApiRow { name: "actions", ty: "impl IntoView", default: "—", desc: "右侧操作区视图。" },
    ]
}

pub fn toolbar_showcase() -> AnyView {
    let title = RwSignal::new(0usize);
    let elevation = RwSignal::new(1i32);
    let height = RwSignal::new(56i32);
    let leading = RwSignal::new(true);
    let titles = ["Gallery", "Settings", "Profile"];
    let comp = dyn_view(move || {
        let mut tb = Toolbar::new()
            .title(titles[title.get()])
            .elevation(elevation.get() as u8)
            .height(height.get() as f32);
        if leading.get() {
            tb = tb.leading("menu");
        }
        tb.actions(Icon::new("more_vert").view()).view().into_any()
    });
    let controls = Stack::vertical((
        segmented(
            "Title",
            vec![("Gallery", 0usize), ("Settings", 1usize), ("Profile", 2usize)],
            title,
        ),
        toggle_row("Leading icon", leading),
        slider_int_row("Elevation", 0, 4, elevation),
        slider_int_row("Height", 48, 80, height),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Toolbar",
        "A top-level bar for titles, navigation and contextual actions.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Toolbar::new().title("Title").view()),
            code_snippet("Toolbar · 基础", TOOLBAR_SRC),
            api_table("Props", toolbar_api()),
        ],
    )
}

// ===========================================================================
// 29. AppBar
// ===========================================================================

const APP_BAR_SRC: &str = r#"AppBar::new()
    .title("Gallery")
    .elevation(2)
    .height(64.0)
    .leading(Icon::new("menu").view())
    .actions(vec![Icon::new("search").view().into_any()])
    .view()"#;

fn app_bar_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造应用栏。" },
        ApiRow { name: "title", ty: "impl Into<String>", default: "—", desc: "标题文字。" },
        ApiRow { name: "elevation", ty: "u8", default: "0", desc: "阴影高度。" },
        ApiRow { name: "height", ty: "f32", default: "64.0", desc: "应用栏高度。" },
        ApiRow { name: "leading", ty: "impl IntoView", default: "—", desc: "左侧视图（图标/按钮）。" },
        ApiRow { name: "actions", ty: "Vec<AnyView>", default: "—", desc: "右侧操作视图集合。" },
    ]
}

pub fn app_bar_showcase() -> AnyView {
    let title = RwSignal::new(0usize);
    let elevation = RwSignal::new(2i32);
    let height = RwSignal::new(64i32);
    let titles = ["Gallery", "Settings", "Profile"];
    let comp = dyn_view(move || {
        AppBar::new()
            .title(titles[title.get()])
            .elevation(elevation.get() as u8)
            .height(height.get() as f32)
            .leading(Icon::new("menu").view())
            .actions(vec![
                Icon::new("search").view().into_any(),
                Icon::new("more_vert").view().into_any(),
            ])
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        segmented(
            "Title",
            vec![("Gallery", 0usize), ("Settings", 1usize), ("Profile", 2usize)],
            title,
        ),
        slider_int_row("Elevation", 0, 6, elevation),
        slider_int_row("Height", 48, 96, height),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "AppBar",
        "A higher-level application bar that hosts leading content and action icons.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", AppBar::new().title("Title").view()),
            code_snippet("AppBar · 基础", APP_BAR_SRC),
            api_table("Props", app_bar_api()),
        ],
    )
}

// ===========================================================================
// 30. BottomNavigation
// ===========================================================================

const BOTTOM_NAV_SRC: &str = r#"let nav = RwSignal::new(0usize);
BottomNavigation::new()
    .labels(vec!["Home".into(), "Search".into(), "Profile".into()])
    .icons(vec![Icon::new("home").view().into_any()])
    .selected(nav.get())
    .on_change(move |i| nav.set(i))
    .view()"#;

fn bottom_navigation_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造底部导航。" },
        ApiRow { name: "labels", ty: "Vec<String>", default: "—", desc: "各项目文字标签。" },
        ApiRow { name: "selected", ty: "usize", default: "0", desc: "当前选中下标。" },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "选中变化回调，参数为下标。" },
        ApiRow { name: "icons", ty: "Vec<AnyView>", default: "—", desc: "各项目图标视图。" },
    ]
}

pub fn bottom_navigation_showcase() -> AnyView {
    let nav = RwSignal::new(0usize);
    let comp = dyn_view(move || {
        let icons = vec![
            Icon::new("home").view().into_any(),
            Icon::new("search").view().into_any(),
            Icon::new("person").view().into_any(),
        ];
        BottomNavigation::new()
            .labels(vec!["Home".into(), "Search".into(), "Profile".into()])
            .icons(icons)
            .selected(nav.get())
            .on_change(move |i| nav.set(i))
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        Label::derived(move || format!("Selected index: {}", nav.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "BottomNavigation",
        "A bottom bar for switching between top-level destinations.",
        vec![
            demo_ctl("Interactive (tap an item)", comp, controls),
            demo(
                "Two items",
                BottomNavigation::new()
                    .labels(vec!["Home".into(), "Settings".into()])
                    .icons(vec![
                        Icon::new("home").view().into_any(),
                        Icon::new("settings").view().into_any(),
                    ])
                    .view(),
            ),
            code_snippet("BottomNavigation · 基础", BOTTOM_NAV_SRC),
            api_table("Props", bottom_navigation_api()),
        ],
    )
}

// ===========================================================================
// 31. Image
// ===========================================================================

const IMAGE_SRC: &str = r#"Image::new()
    .src("assets/preview.png")
    .alt("Image preview")
    .width(160.0)
    .height(120.0)
    .rounded(true)
    .cover(false)
    .view()"#;

fn image_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造图片容器。" },
        ApiRow { name: "src", ty: "impl Into<String>", default: "—", desc: "本地文件路径；远程/缺失时回退到 alt 占位。" },
        ApiRow { name: "alt", ty: "impl Into<String>", default: "—", desc: "占位/无障碍替代文字。" },
        ApiRow { name: "width / height", ty: "f32", default: "—", desc: "像素尺寸。" },
        ApiRow { name: "rounded", ty: "bool", default: "false", desc: "是否圆角。" },
        ApiRow { name: "cover", ty: "bool", default: "false", desc: "是否裁剪填充（cover）。" },
    ]
}

pub fn image_showcase() -> AnyView {
    let rounded = RwSignal::new(true);
    let cover = RwSignal::new(false);
    let width = RwSignal::new(160i32);
    let height = RwSignal::new(120i32);
    let comp = dyn_view(move || {
        Image::new()
            .src("assets/preview.png")
            .alt("Image preview")
            .width(width.get() as f32)
            .height(height.get() as f32)
            .rounded(rounded.get())
            .cover(cover.get())
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Rounded", rounded),
        toggle_row("Cover", cover),
        slider_int_row("Width", 80, 320, width),
        slider_int_row("Height", 60, 240, height),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Image",
        "A raster image surface with placeholder fallback and shape controls.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Rounded", Image::new().src("assets/preview.png").alt("Preview").rounded(true).view()),
            code_snippet("Image · 基础", IMAGE_SRC),
            api_table("Props", image_api()),
        ],
    )
}

// ===========================================================================
// 32. SystemBar
// ===========================================================================

const SYSTEM_BAR_SRC: &str = r#"SystemBar::new()
    .height(24.0)
    .dark(false)
    .pad(8.0)
    .child(Text::new("12:00 · Wi-Fi · Battery").view())
    .view()"#;

fn system_bar_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造系统状态栏。" },
        ApiRow { name: "height", ty: "f32", default: "24.0", desc: "栏高。" },
        ApiRow { name: "dark", ty: "bool", default: "false", desc: "是否深色前景。" },
        ApiRow { name: "pad", ty: "f32", default: "8.0", desc: "内边距。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "栏内内容视图。" },
    ]
}

pub fn system_bar_showcase() -> AnyView {
    let dark = RwSignal::new(false);
    let height = RwSignal::new(24i32);
    let pad = RwSignal::new(8i32);
    let comp = dyn_view(move || {
        SystemBar::new()
            .height(height.get() as f32)
            .dark(dark.get())
            .pad(pad.get() as f32)
            .child(Text::new("12:00 · Wi-Fi · Battery").view())
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Dark", dark),
        slider_int_row("Height", 16, 48, height),
        slider_int_row("Padding", 0, 24, pad),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "SystemBar",
        "A slim status strip for clock, connectivity and battery indicators.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", SystemBar::new().child(Text::new("Status").view()).view()),
            code_snippet("SystemBar · 基础", SYSTEM_BAR_SRC),
            api_table("Props", system_bar_api()),
        ],
    )
}

// ===========================================================================
// 33. Combobox
// ===========================================================================

const COMBOBOX_SRC: &str = r#"let selected = RwSignal::new(1usize);
Combobox::new()
    .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
    .selected(Some(selected.get()))
    .placeholder("Pick a fruit")
    .on_change(move |i: Option<usize>| { /* ... */ })
    .view()"#;

fn combobox_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造下拉选择框。" },
        ApiRow { name: "options", ty: "Vec<String>", default: "—", desc: "可选项。" },
        ApiRow { name: "selected", ty: "Option<usize>", default: "None", desc: "当前选中下标。" },
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "—", desc: "未选时的提示文字。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(Option<usize>)", default: "—", desc: "选中变化回调。" },
    ]
}

pub fn combobox_showcase() -> AnyView {
    let selected = RwSignal::new(1usize);
    let disabled = RwSignal::new(false);
    let picked = RwSignal::new(String::from("Banana"));
    let comp = dyn_view(move || {
        Combobox::new()
            .options(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Cherry".to_string(),
            ])
            .selected(Some(selected.get()))
            .placeholder("Pick a fruit")
            .disabled(disabled.get())
            .on_change(move |i: Option<usize>| {
                let opts = ["Apple", "Banana", "Cherry"];
                if let Some(i) = i {
                    picked.set(opts[i].to_string());
                }
            })
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Selected: {}", picked.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Combobox",
        "A dropdown that selects one option from a static list.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "With selection",
                Combobox::new()
                    .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
                    .selected(Some(0))
                    .view(),
            ),
            code_snippet("Combobox · 基础", COMBOBOX_SRC),
            api_table("Props", combobox_api()),
        ],
    )
}

// ===========================================================================
// 34. Autocomplete
// ===========================================================================

const AUTOCOMPLETE_SRC: &str = r#"let picked = RwSignal::new(String::new());
Autocomplete::new()
    .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
    .placeholder("Type to search")
    .on_change(move |s: Option<String>| { if let Some(s) = s { picked.set(s); } })
    .view()"#;

fn autocomplete_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造自动补全输入框。" },
        ApiRow { name: "options", ty: "Vec<String>", default: "—", desc: "候选项。" },
        ApiRow { name: "selected", ty: "Option<usize>", default: "None", desc: "当前选中下标。" },
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "—", desc: "提示文字。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_change", ty: "Fn(Option<String>)", default: "—", desc: "选中变化回调，参数为选中文字。" },
    ]
}

pub fn autocomplete_showcase() -> AnyView {
    let disabled = RwSignal::new(false);
    let picked = RwSignal::new(String::from("—"));
    let comp = dyn_view(move || {
        Autocomplete::new()
            .options(vec!["Apple".to_string(), "Banana".to_string(), "Cherry".to_string()])
            .placeholder("Type to search")
            .disabled(disabled.get())
            .on_change(move |s: Option<String>| {
                if let Some(s) = s {
                    picked.set(s);
                }
            })
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Selected: {}", picked.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Autocomplete",
        "A text field that filters and completes from a candidate list.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "With placeholder",
                Autocomplete::new()
                    .options(vec!["Apple".into(), "Banana".into(), "Cherry".into()])
                    .placeholder("Type to search")
                    .view(),
            ),
            code_snippet("Autocomplete · 基础", AUTOCOMPLETE_SRC),
            api_table("Props", autocomplete_api()),
        ],
    )
}

// ===========================================================================
// 35. SkeletonLoader
// ===========================================================================

const SKELETON_SRC: &str = r#"SkeletonLoader::new()
    .skeleton_type(SkeletonType::Text)
    .lines(3)
    .animated(true)
    .width(240.0)
    .height(80.0)
    .view()"#;

fn skeleton_loader_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造骨架屏占位。" },
        ApiRow { name: "skeleton_type", ty: "SkeletonType", default: "Text", desc: "形状：Text / Circle / Rect / Avatar。" },
        ApiRow { name: "lines", ty: "usize", default: "3", desc: "文本行数。" },
        ApiRow { name: "animated", ty: "bool", default: "true", desc: "是否显示闪烁动画。" },
        ApiRow { name: "width / height", ty: "f32", default: "—", desc: "像素尺寸。" },
    ]
}

pub fn skeleton_loader_showcase() -> AnyView {
    let kind = RwSignal::new(SkeletonType::Text);
    let lines = RwSignal::new(3i32);
    let animated = RwSignal::new(true);
    let width = RwSignal::new(240i32);
    let height = RwSignal::new(80i32);
    let comp = dyn_view(move || {
        SkeletonLoader::new()
            .skeleton_type(kind.get())
            .lines(lines.get() as usize)
            .animated(animated.get())
            .width(width.get() as f32)
            .height(height.get() as f32)
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        segmented(
            "Type",
            vec![
                ("Text", SkeletonType::Text),
                ("Circle", SkeletonType::Circle),
                ("Rect", SkeletonType::Rect),
                ("Avatar", SkeletonType::Avatar),
            ],
            kind,
        ),
        slider_int_row("Lines", 1, 6, lines),
        toggle_row("Animated", animated),
        slider_int_row("Width", 120, 360, width),
        slider_int_row("Height", 40, 200, height),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "SkeletonLoader",
        "A shimmering placeholder shown while content loads.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Circle", SkeletonLoader::new().skeleton_type(SkeletonType::Circle).view()),
            demo("Avatar", SkeletonLoader::new().skeleton_type(SkeletonType::Avatar).view()),
            code_snippet("SkeletonLoader · 基础", SKELETON_SRC),
            api_table("Props", skeleton_loader_api()),
        ],
    )
}

// ===========================================================================
// 36. Footer
// ===========================================================================

const FOOTER_SRC: &str = r#"Footer::new()
    .height(56.0)
    .elevation(0)
    .bordered(true)
    .pad(16.0)
    .child(Text::new("© 2026 Floom UI").view())
    .view()"#;

fn footer_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造底部栏。" },
        ApiRow { name: "height", ty: "f32", default: "56.0", desc: "栏高。" },
        ApiRow { name: "elevation", ty: "u8", default: "0", desc: "阴影高度。" },
        ApiRow { name: "bordered", ty: "bool", default: "true", desc: "是否显示顶部分隔线。" },
        ApiRow { name: "pad", ty: "f32", default: "16.0", desc: "内边距。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "栏内内容。" },
    ]
}

pub fn footer_showcase() -> AnyView {
    let bordered = RwSignal::new(true);
    let elevation = RwSignal::new(0i32);
    let height = RwSignal::new(56i32);
    let pad = RwSignal::new(16i32);
    let comp = dyn_view(move || {
        Footer::new()
            .height(height.get() as f32)
            .elevation(elevation.get() as u8)
            .bordered(bordered.get())
            .pad(pad.get() as f32)
            .child(Text::new("© 2026 Floom UI · Demo").view())
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Bordered", bordered),
        slider_int_row("Elevation", 0, 8, elevation),
        slider_int_row("Height", 32, 96, height),
        slider_int_row("Padding", 0, 32, pad),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Footer",
        "A bottom content bar for copyright, links and actions.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Default", Footer::new().child(Text::new("Footer").view()).view()),
            code_snippet("Footer · 基础", FOOTER_SRC),
            api_table("Props", footer_api()),
        ],
    )
}

// ===========================================================================
// Batch 4 (12 components) — Stepper / Treeview / Carousel / Calendar /
// Breadcrumbs / ColorPicker / IconButton / ExpansionPanels / SpeedDial /
// EmptyState / Timeline / Tabs
// ===========================================================================

const STEPPER_SRC: &str = r#"Stepper::new()
    .items(vec!["Cart".into(), "Shipping".into(), "Payment".into(), "Review".into()])
    .selected(0)
    .editable(true)
    .non_linear(true)
    .on_change(|i| log::info!("step = {}", i))
    .view()"#;

fn stepper_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "items", ty: "Vec<String>", default: "[]", desc: "步骤标题列表（按顺序）。" },
        ApiRow { name: "selected", ty: "usize", default: "0", desc: "当前高亮步骤索引。" },
        ApiRow { name: "editable", ty: "bool", default: "false", desc: "是否允许点击跳转到任意步骤。" },
        ApiRow { name: "non_linear", ty: "bool", default: "false", desc: "是否允许非线性完成（可跳过步骤）。" },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "切换步骤时回调，参数为新索引。" },
    ]
}

pub fn stepper_showcase() -> AnyView {
    let selected = RwSignal::new(0i32);
    let editable = RwSignal::new(true);
    let non_linear = RwSignal::new(true);
    let picked = RwSignal::new(0usize);
    let comp = {
        let selected = selected;
        let editable = editable;
        let non_linear = non_linear;
        let picked = picked;
        dyn_view(move || {
            Stepper::new()
                .items(vec!["Cart".into(), "Shipping".into(), "Payment".into(), "Review".into()])
                .selected(selected.get() as usize)
                .editable(editable.get())
                .non_linear(non_linear.get())
                .on_change(move |i: usize| picked.set(i))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Editable", editable),
        toggle_row("Non-linear", non_linear),
        slider_int_row("Selected", 0, 3, selected),
        Label::derived(move || format!("Current step: {}", picked.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Stepper",
        "Stepper guides users through a numbered, multi-step flow. Toggle editable / non-linear and \
         drag the selected index to see the active highlight track live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Static",
                Stepper::new()
                    .items(vec!["Cart".into(), "Shipping".into(), "Payment".into()])
                    .view(),
            ),
            code_snippet("Stepper · 基础", STEPPER_SRC),
            api_table("Props", stepper_api()),
        ],
    )
}

const TREEVIEW_SRC: &str = r#"let nodes = vec![
    TreeviewNode { label: "Documents".into(), icon: Some("folder".into()), expanded: true, children: vec![
        TreeviewNode { label: "Resume.pdf".into(), icon: Some("description".into()), ..Default::default() },
    ]},
    TreeviewNode { label: "Pictures".into(), icon: Some("image".into()), ..Default::default() },
];
Treeview::new()
    .nodes(nodes)
    .on_node_click(|label| log::info!("clicked {}", label))
    .view()"#;

fn treeview_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "nodes", ty: "Vec<TreeviewNode>", default: "[]", desc: "根节点列表，可嵌套 children / icon / expanded。" },
        ApiRow { name: "on_node_click", ty: "Fn(String)", default: "—", desc: "点击节点时回调，参数为节点 label。" },
    ]
}

pub fn treeview_showcase() -> AnyView {
    let clicked = RwSignal::new(String::from("—"));
    let comp = {
        let clicked = clicked;
        dyn_view(move || {
            let nodes = vec![
                TreeviewNode {
                    label: "Documents".into(),
                    icon: Some("folder".into()),
                    expanded: true,
                    children: vec![TreeviewNode {
                        label: "Resume.pdf".into(),
                        icon: Some("description".into()),
                        ..Default::default()
                    }],
                },
                TreeviewNode { label: "Pictures".into(), icon: Some("image".into()), ..Default::default() },
                TreeviewNode { label: "Music".into(), icon: Some("music_note".into()), ..Default::default() },
            ];
            Treeview::new()
                .nodes(nodes)
                .on_node_click(move |label: String| clicked.set(label))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        Label::derived(move || format!("Clicked: {}", clicked.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ))
    .style(|s| s.gap(8.0).items_start());

    showcase(
        "Treeview",
        "Treeview shows hierarchical data with expandable branches. Click a node to see the selected \
         path echoed below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("Treeview · 基础", TREEVIEW_SRC),
            api_table("Props", treeview_api()),
        ],
    )
}

const CAROUSEL_SRC: &str = r#"let slide = |c: Color, title: &str| -> AnyView {
    Container::new()
        .child(Label::new(title.to_string()).style(|s| s.color(Color::rgb(255,255,255)).font_size(16.0)))
        .style(move |s| s.background(c.to_peniko()).width(280.0).height(150.0).items_center().justify_center())
        .into_any()
};
Carousel::new()
    .slides(vec![slide(c1, "Slide 1"), slide(c2, "Slide 2"), slide(c3, "Slide 3")])
    .auto_play(true)
    .interval_ms(3000)
    .show_arrows(true)
    .show_dots(true)
    .view()"#;

fn carousel_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "slides", ty: "Vec<AnyView>", default: "[]", desc: "轮播页内容列表。" },
        ApiRow { name: "height", ty: "f32", default: "0.0", desc: "轮播区高度（px）。" },
        ApiRow { name: "auto_play", ty: "bool", default: "false", desc: "是否自动轮播。" },
        ApiRow { name: "interval_ms", ty: "u64", default: "—", desc: "自动轮播间隔（毫秒）。" },
        ApiRow { name: "show_arrows", ty: "bool", default: "—", desc: "是否显示前/后箭头。" },
        ApiRow { name: "show_dots", ty: "bool", default: "—", desc: "是否显示指示点。" },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "切换页时回调，参数为索引。" },
    ]
}

pub fn carousel_showcase() -> AnyView {
    let auto_play = RwSignal::new(true);
    let show_arrows = RwSignal::new(true);
    let show_dots = RwSignal::new(true);
    let interval_ms = RwSignal::new(3000i32);
    let height = RwSignal::new(150i32);
    let idx = RwSignal::new(0usize);
    let comp = {
        let auto_play = auto_play;
        let show_arrows = show_arrows;
        let show_dots = show_dots;
        let interval_ms = interval_ms;
        let height = height;
        let idx = idx;
        dyn_view(move || {
            let slide = |color: floom_core_theme::tokens::color::Color, title: &str| -> AnyView {
                Container::new()
                    .child(
                        Label::new(title.to_string())
                            .style(|s| s.color(floom_core_theme::tokens::color::Color::rgb(255, 255, 255).to_peniko()).font_size(16.0)),
                    )
                    .view()
                    .style(move |s| {
                        s.background(color.to_peniko())
                            .width(280.0)
                            .height(150.0)
                            .items_center()
                            .justify_center()
                    })
                    .into_any()
            };
            let slides: Vec<AnyView> = vec![
                slide(floom_core_theme::tokens::color::Color::rgb(63, 81, 181), "Slide 1"),
                slide(floom_core_theme::tokens::color::Color::rgb(0, 150, 136), "Slide 2"),
                slide(floom_core_theme::tokens::color::Color::rgb(233, 30, 99), "Slide 3"),
            ];
            Carousel::new()
                .slides(slides)
                .auto_play(auto_play.get())
                .interval_ms(interval_ms.get() as u64)
                .show_arrows(show_arrows.get())
                .show_dots(show_dots.get())
                .height(height.get() as f32)
                .on_change(move |i: usize| idx.set(i))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical_from_iter(vec![
        toggle_row("Auto play", auto_play),
        toggle_row("Show arrows", show_arrows),
        toggle_row("Show dots", show_dots),
        slider_int_row("Interval (ms)", 1000, 6000, interval_ms),
        slider_int_row("Height", 80, 240, height),
        Label::derived(move || format!("Current index: {}", idx.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ])
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Carousel",
        "Carousel cycles through a set of slides. Toggle auto-play / arrows / dots and adjust the \
         interval and height to see it react live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("Carousel · 基础", CAROUSEL_SRC),
            api_table("Props", carousel_api()),
        ],
    )
}

const CALENDAR_SRC: &str = r#"let value = RwSignal::new(CalendarDate { year: 2026, month: 8, day: 10 });
Calendar::new()
    .value(value.get())
    .show_week_numbers(true)
    .on_change(|d| log::info!("picked {:?}", d))
    .view()"#;

fn calendar_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "value", ty: "CalendarDate", default: "2026-08-01", desc: "当前选中日期 {year,month,day}。" },
        ApiRow { name: "show_week_numbers", ty: "bool", default: "false", desc: "是否显示周数列。" },
        ApiRow { name: "on_change", ty: "Fn(CalendarDate)", default: "—", desc: "选择日期时回调。" },
    ]
}

pub fn calendar_showcase() -> AnyView {
    let show_weeks = RwSignal::new(true);
    let picked = RwSignal::new(CalendarDate { year: 2026, month: 8, day: 10 });
    let comp = {
        let show_weeks = show_weeks;
        let picked = picked;
        dyn_view(move || {
            Calendar::new()
                .value(picked.get())
                .show_week_numbers(show_weeks.get())
                .on_change(move |d: CalendarDate| picked.set(d))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Show week numbers", show_weeks),
        Label::derived(move || {
            let d = picked.get();
            format!("Selected: {:04}-{:02}-{:02}", d.year, d.month, d.day)
        })
        .style(|s| s.font_size(14.0))
        .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Calendar",
        "Calendar renders a month grid for date selection. Toggle the week-number column and pick a \
         day to see the selected date echoed below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("Calendar · 基础", CALENDAR_SRC),
            api_table("Props", calendar_api()),
        ],
    )
}

const BREADCRUMBS_SRC: &str = r#"Breadcrumbs::new()
    .items(vec!["Home".into(), "Products".into(), "Category".into(), "Item".into()])
    .on_click(|i| log::info!("nav index {}", i))
    .view()"#;

fn breadcrumbs_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "items", ty: "Vec<String>", default: "[]", desc: "面包屑各层级标题。" },
        ApiRow { name: "on_click", ty: "Fn(usize)", default: "—", desc: "点击某层级时回调，参数为索引。" },
    ]
}

pub fn breadcrumbs_showcase() -> AnyView {
    let idx = RwSignal::new(2usize);
    let comp = {
        let idx = idx;
        dyn_view(move || {
            Breadcrumbs::new()
                .items(vec!["Home".into(), "Products".into(), "Category".into(), "Item".into()])
                .on_click(move |i: usize| idx.set(i))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        Label::derived(move || format!("Last clicked index: {}", idx.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ))
    .style(|s| s.gap(8.0).items_start());

    showcase(
        "Breadcrumbs",
        "Breadcrumbs show the current location in a navigation hierarchy. Click a segment to see the \
         clicked index echoed below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Static",
                Breadcrumbs::new()
                    .items(vec!["Docs".into(), "Guide".into(), "Getting started".into()])
                    .view(),
            ),
            code_snippet("Breadcrumbs · 基础", BREADCRUMBS_SRC),
            api_table("Props", breadcrumbs_api()),
        ],
    )
}

const COLOR_PICKER_SRC: &str = r#"let value = RwSignal::new(Color::rgb(33, 150, 243));
ColorPicker::new()
    .value(value.get())
    .show_alpha(true)
    .on_change(|c| log::info!("color {:?}", c))
    .view()"#;

fn color_picker_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "value", ty: "Color", default: "—", desc: "当前颜色。" },
        ApiRow { name: "show_alpha", ty: "bool", default: "false", desc: "是否显示透明度通道。" },
        ApiRow { name: "on_change", ty: "Fn(Color)", default: "—", desc: "颜色变化时回调。" },
        ApiRow { name: "color_signal", ty: "RwSignal<Color>", default: "—", desc: "返回绑定颜色信号，可读取实时值。" },
    ]
}

pub fn color_picker_showcase() -> AnyView {
    let show_alpha = RwSignal::new(true);
    let value = RwSignal::new(floom_core_theme::tokens::color::Color::rgb(33, 150, 243));
    let comp = {
        let show_alpha = show_alpha;
        let value = value;
        dyn_view(move || {
            ColorPicker::new()
                .value(value.get())
                .show_alpha(show_alpha.get())
                .on_change(move |c| value.set(c))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Show alpha", show_alpha),
        Label::derived(move || {
            let c = value.get();
            format!("RGBA: {}, {}, {}, {}", c.r, c.g, c.b, c.a)
        })
        .style(|s| s.font_size(14.0))
        .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ColorPicker",
        "ColorPicker lets users pick a color. Toggle the alpha channel and change the swatch to see \
         the live RGBA values below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("ColorPicker · 基础", COLOR_PICKER_SRC),
            api_table("Props", color_picker_api()),
        ],
    )
}

const ICON_BUTTON_SRC: &str = r#"IconButton::new("favorite")
    .size(32.0)
    .variant(IconButtonVariant::Contained)
    .disabled(false)
    .on_click(|| log::info!("clicked"))
    .view()"#;

fn icon_button_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(icon)", ty: "impl Into<String>", default: "—", desc: "构造图标按钮，icon 为 SVG 名称。" },
        ApiRow { name: "size", ty: "f32", default: "token", desc: "按钮尺寸（px）。" },
        ApiRow { name: "variant", ty: "IconButtonVariant", default: "Text", desc: "视觉变体：Text / Contained / Outlined。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_click", ty: "Fn()", default: "—", desc: "点击回调。" },
    ]
}

pub fn icon_button_showcase() -> AnyView {
    let size = RwSignal::new(32i32);
    let variant = RwSignal::new(IconButtonVariant::Contained);
    let disabled = RwSignal::new(false);
    let clicked = RwSignal::new(0usize);
    let comp = {
        let size = size;
        let variant = variant;
        let disabled = disabled;
        let clicked = clicked;
        dyn_view(move || {
            IconButton::new("favorite")
                .size(size.get() as f32)
                .variant(variant.get())
                .disabled(disabled.get())
                .on_click(move || clicked.set(clicked.get() + 1))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        segmented(
            "Variant",
            vec![
                ("Text", IconButtonVariant::Text),
                ("Contained", IconButtonVariant::Contained),
                ("Outlined", IconButtonVariant::Outlined),
            ],
            variant,
        ),
        slider_int_row("Size", 16, 64, size),
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Clicks: {}", clicked.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "IconButton",
        "IconButton is a compact button that shows only an icon. Switch the variant, size and \
         disabled state, then click to count activations.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Variants",
                Stack::horizontal((
                    IconButton::new("home").view().into_any(),
                    IconButton::new("favorite").variant(IconButtonVariant::Contained).view().into_any(),
                    IconButton::new("settings").variant(IconButtonVariant::Outlined).view().into_any(),
                ))
                .style(|s| s.gap(12.0).items_center())
                .into_any(),
            ),
            code_snippet("IconButton · 基础", ICON_BUTTON_SRC),
            api_table("Props", icon_button_api()),
        ],
    )
}

const EXPANSION_PANELS_SRC: &str = r#"ExpansionPanels::new()
    .panel("Shipping", || Text::new("Ship via courier or pickup").into_any())
    .panel("Payment", || Text::new("Card, wallet or invoice").into_any())
    .multiple(true)
    .on_change(|i, open| log::info!("panel {} open={}", i, open))
    .view()"#;

fn expansion_panels_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "panel", ty: "(title, content)", default: "—", desc: "添加可折叠面板，content 为惰性视图工厂。" },
        ApiRow { name: "multiple", ty: "bool", default: "false", desc: "是否允许同时展开多个面板。" },
        ApiRow { name: "on_change", ty: "Fn(usize, bool)", default: "—", desc: "面板开合回调，参数 (索引, 是否展开)。" },
    ]
}

pub fn expansion_panels_showcase() -> AnyView {
    let multiple = RwSignal::new(true);
    let state = RwSignal::new(String::from("—"));
    let comp = {
        let multiple = multiple;
        let state = state;
        dyn_view(move || {
            ExpansionPanels::new()
                .panel("Shipping", || Text::new("Ship via courier or pickup").view().into_any())
                .panel("Payment", || Text::new("Card, wallet or invoice").view().into_any())
                .panel("Review", || Text::new("Confirm before placing the order").view().into_any())
                .multiple(multiple.get())
                .on_change(move |i: usize, open: bool| {
                    state.set(format!("panel {} {}", i, if open { "opened" } else { "closed" }))
                })
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Multiple", multiple),
        Label::derived(move || format!("Last: {}", state.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ExpansionPanels",
        "ExpansionPanels stacks collapsible sections. Toggle multiple-expand and open/close panels to \
         see the callback echoed below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("ExpansionPanels · 基础", EXPANSION_PANELS_SRC),
            api_table("Props", expansion_panels_api()),
        ],
    )
}

const SPEED_DIAL_SRC: &str = r#"SpeedDial::new()
    .action("edit", Some("Edit".into()))
    .action("delete", Some("Delete".into()))
    .direction(SpeedDialDirection::Up)
    .open(true)
    .on_action(|i| log::info!("action {}", i))
    .view()"#;

fn speed_dial_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "action", ty: "(icon, Option<label>)", default: "—", desc: "添加一个展开动作按钮。" },
        ApiRow { name: "direction", ty: "SpeedDialDirection", default: "Up", desc: "动作展开方向：Up / Down / Left / Right。" },
        ApiRow { name: "open", ty: "bool", default: "false", desc: "是否初始展开。" },
        ApiRow { name: "fab_icon", ty: "impl Into<String>", default: "plus", desc: "FAB 主图标名称。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "on_action", ty: "Fn(usize)", default: "—", desc: "点击动作时回调，参数为索引。" },
    ]
}

pub fn speed_dial_showcase() -> AnyView {
    let direction = RwSignal::new(SpeedDialDirection::Up);
    let open = RwSignal::new(true);
    let disabled = RwSignal::new(false);
    let action = RwSignal::new(0usize);
    let comp = {
        let direction = direction;
        let open = open;
        let disabled = disabled;
        let action = action;
        dyn_view(move || {
            SpeedDial::new()
                .action("edit", Some("Edit".into()))
                .action("delete", Some("Delete".into()))
                .action("share", Some("Share".into()))
                .direction(direction.get())
                .open(open.get())
                .disabled(disabled.get())
                .on_action(move |i: usize| action.set(i))
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        segmented(
            "Direction",
            vec![
                ("Up", SpeedDialDirection::Up),
                ("Down", SpeedDialDirection::Down),
                ("Left", SpeedDialDirection::Left),
                ("Right", SpeedDialDirection::Right),
            ],
            direction,
        ),
        toggle_row("Open", open),
        toggle_row("Disabled", disabled),
        Label::derived(move || format!("Last action index: {}", action.get()))
            .style(|s| s.font_size(14.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "SpeedDial",
        "SpeedDial is a FAB that fans out a set of actions. Change the direction, open state and \
         disabled flag, then trigger an action to see its index echoed below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("SpeedDial · 基础", SPEED_DIAL_SRC),
            api_table("Props", speed_dial_api()),
        ],
    )
}

const EMPTY_STATE_SRC: &str = r#"EmptyState::new()
    .icon("inbox")
    .title("No messages")
    .description("You have no messages in this folder yet.")
    .view()"#;

fn empty_state_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "icon", ty: "impl Into<String>", default: "—", desc: "占位图标名称。" },
        ApiRow { name: "title", ty: "impl Into<String>", default: "—", desc: "主标题。" },
        ApiRow { name: "description", ty: "impl Into<String>", default: "—", desc: "辅助说明文字。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "可选自定义内容（覆盖默认排版）。" },
    ]
}

pub fn empty_state_showcase() -> AnyView {
    let inbox = EmptyState::new()
        .icon("inbox")
        .title("No messages")
        .description("You have no messages in this folder yet.")
        .view()
        .into_any();
    let search = EmptyState::new()
        .icon("search_off")
        .title("Nothing found")
        .description("Try a different search term or filter.")
        .view()
        .into_any();

    showcase(
        "EmptyState",
        "EmptyState fills a region when there is no data to show. Pair it with a clear icon, title and \
         description so users know what to do next.",
        vec![
            demo("Inbox empty", inbox),
            demo("Search empty", search),
            code_snippet("EmptyState · 基础", EMPTY_STATE_SRC),
            api_table("Props", empty_state_api()),
        ],
    )
}

const TIMELINE_SRC: &str = r#"Timeline::new()
    .item("Order placed", "We received your order.")
    .item("Packed", "Items were packed at the warehouse.")
    .item("Shipped", "Your package is on the way.")
    .item("Delivered", "Arrived at your address.")
    .view()"#;

fn timeline_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "item", ty: "(title, description)", default: "—", desc: "追加一个时间线节点。" },
        ApiRow { name: "items", ty: "Vec<TimelineItem>", default: "[]", desc: "批量设置节点列表。" },
    ]
}

pub fn timeline_showcase() -> AnyView {
    let comp = Timeline::new()
        .item("Order placed", "We received your order.")
        .item("Packed", "Items were packed at the warehouse.")
        .item("Shipped", "Your package is on the way.")
        .item("Delivered", "Arrived at your address.")
        .view()
        .into_any();

    showcase(
        "Timeline",
        "Timeline lays out events in chronological order with a connecting rail. Use it for order \
         status, activity feeds and histories.",
        vec![
            demo("Order flow", comp),
            code_snippet("Timeline · 基础", TIMELINE_SRC),
            api_table("Props", timeline_api()),
        ],
    )
}

const CHECKBOX_BTN_SRC: &str = r#"CheckboxBtn::new()
    .label("Enable notifications")
    .checked(true)
    .disabled(false)
    .on_change(|v| log::info!("checked = {}", v))
    .view()"#;

fn checkbox_btn_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造图标复选按钮。" },
        ApiRow { name: "checked", ty: "bool", default: "false", desc: "是否选中。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "是否禁用。" },
        ApiRow { name: "label", ty: "impl Into<String>", default: "—", desc: "右侧文字标签。" },
        ApiRow { name: "on_change", ty: "Fn(bool)", default: "—", desc: "勾选状态变化回调。" },
    ]
}

pub fn checkbox_btn_showcase() -> AnyView {
    let checked = RwSignal::new(true);
    let disabled = RwSignal::new(false);
    let comp = {
        let checked = checked;
        let disabled = disabled;
        dyn_view(move || {
            CheckboxBtn::new()
                .label("Enable notifications")
                .checked(checked.get())
                .disabled(disabled.get())
                .on_change(move |_v: bool| {})
                .view()
                .into_any()
        })
    };
    let controls = Stack::vertical((
        toggle_row("Checked", checked),
        toggle_row("Disabled", disabled),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "CheckboxBtn",
        "CheckboxBtn is a button-styled checkbox — a compact, labeled toggle. Flip checked / disabled \
         to see the state update live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Static",
                CheckboxBtn::new().label("Accept terms").checked(false).view(),
            ),
            code_snippet("CheckboxBtn · 基础", CHECKBOX_BTN_SRC),
            api_table("Props", checkbox_btn_api()),
        ],
    )
}

// ===========================================================================
// 43. DatePicker
// ===========================================================================

const DATE_PICKER_SRC: &str = r#"let date_val = RwSignal::new(DateValue { year: 2026, month: 8, day: 10 });
DatePicker::new()
    .value(date_val.get())
    .placeholder("Select a date")
    .on_change(move |d| date_val.set(d))
    .view()"#;

fn date_picker_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造日期选择器。" },
        ApiRow { name: "value", ty: "DateValue", default: "today", desc: "当前选中日期 {year, month, day}。" },
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "—", desc: "未选择时的提示文字。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "禁用交互。" },
        ApiRow { name: "on_change", ty: "Fn(DateValue) + 'static", default: "—", desc: "选择变化时回调，返回新日期。" },
    ]
}

pub fn date_picker_showcase() -> AnyView {
    let date_val = RwSignal::new(DateValue { year: 2026, month: 8, day: 10 });
    let disabled = RwSignal::new(false);
    let comp = dyn_view(move || {
        DatePicker::new()
            .value(date_val.get())
            .placeholder("Select a date")
            .disabled(disabled.get())
            .on_change(move |d| date_val.set(d))
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || {
            let d = date_val.get();
            format!("Selected: {:04}-{:02}-{:02}", d.year, d.month, d.day)
        })
        .style(|s| s.font_size(13.0))
        .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "DatePicker",
        "Pick a calendar date. The bound value updates live and is echoed below for clear visual feedback.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("With placeholder", DatePicker::new().placeholder("Pick a day").view()),
            demo(
                "Disabled",
                DatePicker::new().value(DateValue { year: 2026, month: 1, day: 1 }).disabled(true).view(),
            ),
            code_snippet("DatePicker · 基础", DATE_PICKER_SRC),
            api_table("Props", date_picker_api()),
        ],
    )
}

// ===========================================================================
// 44. TimePicker
// ===========================================================================

const TIME_PICKER_SRC: &str = r#"let time_val = RwSignal::new(TimeValue { hour: 9, minute: 30 });
TimePicker::new()
    .value(time_val.get())
    .placeholder("Select a time")
    .on_change(move |t| time_val.set(t))
    .view()"#;

fn time_picker_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造时间选择器。" },
        ApiRow { name: "value", ty: "TimeValue", default: "now", desc: "当前选中时间 {hour, minute}。" },
        ApiRow { name: "placeholder", ty: "impl Into<String>", default: "—", desc: "未选择时的提示文字。" },
        ApiRow { name: "disabled", ty: "bool", default: "false", desc: "禁用交互。" },
        ApiRow { name: "on_change", ty: "Fn(TimeValue) + 'static", default: "—", desc: "选择变化时回调，返回新时间。" },
    ]
}

pub fn time_picker_showcase() -> AnyView {
    let time_val = RwSignal::new(TimeValue { hour: 9, minute: 30 });
    let disabled = RwSignal::new(false);
    let comp = dyn_view(move || {
        TimePicker::new()
            .value(time_val.get())
            .placeholder("Select a time")
            .disabled(disabled.get())
            .on_change(move |t| time_val.set(t))
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Disabled", disabled),
        Label::derived(move || {
            let t = time_val.get();
            format!("Selected: {:02}:{:02}", t.hour, t.minute)
        })
        .style(|s| s.font_size(13.0))
        .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "TimePicker",
        "Pick an hour/minute. The bound value updates live and is echoed below for clear visual feedback.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("With placeholder", TimePicker::new().placeholder("Pick a time").view()),
            demo(
                "Disabled",
                TimePicker::new().value(TimeValue { hour: 12, minute: 0 }).disabled(true).view(),
            ),
            code_snippet("TimePicker · 基础", TIME_PICKER_SRC),
            api_table("Props", time_picker_api()),
        ],
    )
}

// ===========================================================================
// 64. SlideGroup
// ===========================================================================

const SLIDE_GROUP_SRC: &str = r#"let sel = RwSignal::new(0usize);
let items: Vec<AnyView> = ["One", "Two", "Three", "Four"]
    .iter().map(|l| Label::new(*l).style(|s| s.padding(10.0)).into_any()).collect();
SlideGroup::new()
    .items(items)
    .selected(0)
    .show_arrows(true)
    .on_change(move |i| sel.set(i))
    .view()"#;

fn slide_group_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造滑动选择组。" },
        ApiRow { name: "items", ty: "Vec<AnyView>", default: "—", desc: "可切换的子项视图。" },
        ApiRow { name: "selected", ty: "usize", default: "0", desc: "初始选中索引。" },
        ApiRow { name: "show_arrows", ty: "bool", default: "false", desc: "是否显示左右翻页箭头。" },
        ApiRow { name: "mandatory", ty: "bool", default: "false", desc: "是否强制至少选中一项。" },
        ApiRow { name: "on_change", ty: "Fn(usize) + 'static", default: "—", desc: "选中变化时回调，返回新索引。" },
    ]
}

pub fn slide_group_showcase() -> AnyView {
    let sel = RwSignal::new(0usize);
    let show_arrows = RwSignal::new(true);
    let comp = dyn_view(move || {
        let items: Vec<AnyView> = ["One", "Two", "Three", "Four"]
            .iter()
            .map(|l| Label::new(*l).style(|s| s.padding(10.0)).into_any())
            .collect();
        SlideGroup::new()
            .items(items)
            .selected(sel.get())
            .show_arrows(show_arrows.get())
            .on_change(move |i| sel.set(i))
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Show arrows", show_arrows),
        Label::derived(move || format!("Selected index: {}", sel.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "SlideGroup",
        "A horizontally-paged selector. Toggle the arrows and watch the selected index echo below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("No arrows", {
                let items: Vec<AnyView> = ["A", "B", "C"]
                    .iter()
                    .map(|l| Label::new(*l).style(|s| s.padding(10.0)).into_any())
                    .collect();
                SlideGroup::new().items(items).selected(0).show_arrows(false).view()
            }),
            code_snippet("SlideGroup · 基础", SLIDE_GROUP_SRC),
            api_table("Props", slide_group_api()),
        ],
    )
}

// ===========================================================================
// 65. ItemGroup
// ===========================================================================

const ITEM_GROUP_SRC: &str = r#"let sel = RwSignal::new(0usize);
let items: Vec<AnyView> = ["Apple", "Banana", "Cherry"]
    .iter().map(|l| Label::new(*l).style(|s| s.padding(10.0)).into_any()).collect();
ItemGroup::new()
    .items(items)
    .selected(0)
    .on_change(move |i| sel.set(i))
    .view()"#;

fn item_group_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造条目选择组。" },
        ApiRow { name: "items", ty: "Vec<AnyView>", default: "—", desc: "可选择的子项视图。" },
        ApiRow { name: "selected", ty: "usize", default: "0", desc: "初始选中索引（单选）。" },
        ApiRow { name: "multiple", ty: "bool", default: "false", desc: "是否允许多选。" },
        ApiRow { name: "mandatory", ty: "bool", default: "false", desc: "是否强制至少选中一项。" },
        ApiRow { name: "on_change", ty: "Fn(usize) + 'static", default: "—", desc: "单选变化时回调。" },
        ApiRow { name: "on_change_multi", ty: "Fn(Vec<usize>) + 'static", default: "—", desc: "多选变化时回调。" },
    ]
}

pub fn item_group_showcase() -> AnyView {
    let sel = RwSignal::new(0usize);
    let comp = dyn_view(move || {
        let items: Vec<AnyView> = ["Apple", "Banana", "Cherry"]
            .iter()
            .map(|l| Label::new(*l).style(|s| s.padding(10.0)).into_any())
            .collect();
        ItemGroup::new()
            .items(items)
            .selected(sel.get())
            .on_change(move |i| sel.set(i))
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        Label::derived(move || format!("Selected index: {}", sel.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ItemGroup",
        "A selectable list of items (single-select here). The chosen index is echoed below.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Multiple select", {
                let items: Vec<AnyView> = ["Red", "Green", "Blue"]
                    .iter()
                    .map(|l| Label::new(*l).style(|s| s.padding(10.0)).into_any())
                    .collect();
                ItemGroup::new().items(items).multiple(true).on_change_multi(|_v: Vec<usize>| {}).view()
            }),
            code_snippet("ItemGroup · 基础", ITEM_GROUP_SRC),
            api_table("Props", item_group_api()),
        ],
    )
}

// ===========================================================================
// 67. ButtonGroup
// ===========================================================================

const BUTTON_GROUP_SRC: &str = r#"let b = |label: &str| Button::new(label).variant(ButtonVariant::Primary).view();
ButtonGroup::new()
    .joined(true)
    .child(b("One"))
    .child(b("Two"))
    .child(b("Three"))
    .view()"#;

fn button_group_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造按钮组容器。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "追加一个子按钮（可多次调用）。" },
        ApiRow { name: "joined", ty: "bool", default: "false", desc: "是否合并为连体样式（去间距/共享边框）。" },
        ApiRow { name: "vertical", ty: "bool", default: "false", desc: "是否垂直排列。" },
    ]
}

pub fn button_group_showcase() -> AnyView {
    let joined = RwSignal::new(true);
    let vertical = RwSignal::new(false);
    let comp = dyn_view(move || {
        let b = |label: &str| Button::new(label).variant(ButtonVariant::Primary).view();
        ButtonGroup::new()
            .joined(joined.get())
            .vertical(vertical.get())
            .child(b("One"))
            .child(b("Two"))
            .child(b("Three"))
            .view()
            .into_any()
    });
    let controls = Stack::vertical((
        toggle_row("Joined", joined),
        toggle_row("Vertical", vertical),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ButtonGroup",
        "Group related buttons. Toggle joined / vertical to see the layout change live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Separated", {
                let b = |label: &str| Button::new(label).variant(ButtonVariant::Primary).view();
                ButtonGroup::new().joined(false).child(b("One")).child(b("Two")).child(b("Three")).view()
            }),
            code_snippet("ButtonGroup · 基础", BUTTON_GROUP_SRC),
            api_table("Props", button_group_api()),
        ],
    )
}

// ===========================================================================
// 73. Table
// ===========================================================================

const TABLE_SRC: &str = r#"Table::new()
    .headers(vec!["Name".into(), "Role".into(), "Status".into()])
    .rows(vec![
        vec!["Ada".to_string(), "Engineer".to_string(), "Active".to_string()],
        vec!["Linus".to_string(), "Maintainer".to_string(), "Active".to_string()],
        vec!["Grace".to_string(), "Architect".to_string(), "Away".to_string()],
    ])
    .striped(true)
    .view()"#;

fn table_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造表格。" },
        ApiRow { name: "headers", ty: "Vec<String>", default: "—", desc: "表头列标题。" },
        ApiRow { name: "rows", ty: "Vec<Vec<String>>", default: "—", desc: "数据行（每行与表头等长）。" },
        ApiRow { name: "dense", ty: "bool", default: "false", desc: "紧凑行高。" },
        ApiRow { name: "striped", ty: "bool", default: "false", desc: " zebra 条纹行背景。" },
    ]
}

pub fn table_showcase() -> AnyView {
    let dense = RwSignal::new(false);
    let striped = RwSignal::new(true);
    let comp = dyn_view(move || {
        Table::new()
            .headers(vec!["Name".into(), "Role".into(), "Status".into()])
            .rows(vec![
                vec!["Ada".to_string(), "Engineer".to_string(), "Active".to_string()],
                vec!["Linus".to_string(), "Maintainer".to_string(), "Active".to_string()],
                vec!["Grace".to_string(), "Architect".to_string(), "Away".to_string()],
            ])
            .dense(dense.get())
            .striped(striped.get())
            .view()
            .into_any()
    });
    let controls = Stack::vertical((toggle_row("Dense", dense), toggle_row("Striped", striped)))
        .style(|s| s.gap(12.0).items_start());

    showcase(
        "Table",
        "A simple data table. Toggle dense / striped to restyle the rows live.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Plain", {
                Table::new()
                    .headers(vec!["Name".into(), "Role".into(), "Status".into()])
                    .rows(vec![
                        vec!["Ada".to_string(), "Engineer".to_string(), "Active".to_string()],
                        vec!["Linus".to_string(), "Maintainer".to_string(), "Active".to_string()],
                        vec!["Grace".to_string(), "Architect".to_string(), "Away".to_string()],
                    ])
                    .view()
            }),
            code_snippet("Table · 基础", TABLE_SRC),
            api_table("Props", table_api()),
        ],
    )
}

// ===========================================================================
// 76. Kbd
// ===========================================================================

const KBD_SRC: &str = r#"Stack::horizontal((
    Kbd::new().content("Ctrl").view(),
    Kbd::new().content("Shift").view(),
    Kbd::new().content("P").view(),
))
.style(|s| s.gap(6.0).items_center())"#;

fn kbd_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造键盘按键样式块。" },
        ApiRow { name: "content", ty: "impl Into<String>", default: "—", desc: "按键文字（如 Ctrl / Shift / P）。" },
        ApiRow { name: "mono", ty: "bool", default: "false", desc: "是否使用等宽字体。" },
    ]
}

pub fn kbd_showcase() -> AnyView {
    let mono = RwSignal::new(false);
    let comp = dyn_view(move || {
        Stack::horizontal((
            Kbd::new().content("Ctrl").mono(mono.get()).view(),
            Kbd::new().content("Shift").mono(mono.get()).view(),
            Kbd::new().content("P").mono(mono.get()).view(),
        ))
        .style(|s| s.gap(6.0).items_center())
        .into_any()
    });
    let controls = Stack::vertical((toggle_row("Monospace", mono),)).style(|s| s.gap(12.0).items_start());

    showcase(
        "Kbd",
        "Render keyboard shortcuts as styled keycaps. Toggle monospace to compare typefaces.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Single key", Kbd::new().content("Esc").view()),
            demo(
                "Combo",
                Stack::horizontal((Kbd::new().content("Cmd").view(), Kbd::new().content("C").view()))
                    .style(|s| s.gap(6.0).items_center())
                    .into_any(),
            ),
            code_snippet("Kbd · 基础", KBD_SRC),
            api_table("Props", kbd_api()),
        ],
    )
}

// ===========================================================================
// 82. Hover
// ===========================================================================

const HOVER_SRC: &str = r#"let hover = Hover::new().child(
    Text::new("Hover over this card").view().style(|s| s.padding(20.0).font_size(16.0)),
);
let hover_sig = hover.hovered_signal();
Stack::vertical((hover.view(), Label::derived(move || format!("hovered: {}", hover_sig.get()))))
    .style(|s| s.gap(8.0).items_start())"#;

fn hover_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造悬停检测容器。" },
        ApiRow { name: "child", ty: "impl IntoView", default: "—", desc: "被检测悬停的子视图。" },
        ApiRow { name: "on_hover_change", ty: "Fn(bool) + 'static", default: "—", desc: "悬停状态变化回调。" },
        ApiRow { name: "hovered_signal", ty: "RwSignal<bool>", default: "—", desc: "只读悬停状态信号，供派生视图使用。" },
    ]
}

pub fn hover_showcase() -> AnyView {
    let hover = Hover::new().child(
        Text::new("Hover over this card")
            .view()
            .style(|s| s.padding(20.0).font_size(16.0)),
    );
    let hover_sig = hover.hovered_signal();
    let comp = Stack::vertical((
        hover.view(),
        Label::derived(move || format!("hovered: {}", hover_sig.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(8.0).items_start());
    let controls = Stack::vertical((
        Label::derived(move || {
            if hover_sig.get() {
                "STATE: HOVERED"
            } else {
                "STATE: idle"
            }
        })
        .style(|s| s.font_size(13.0).font_bold())
        .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Hover",
        "Detect pointer hover and drive derived UI. The status readout flips to HOVERED in bold while hovered.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Static card",
                Hover::new()
                    .child(Text::new("Hover me").view().style(|s| s.padding(20.0).font_size(16.0)))
                    .view(),
            ),
            code_snippet("Hover · 基础", HOVER_SRC),
            api_table("Props", hover_api()),
        ],
    )
}

// ===========================================================================
// 83. FadeTransition
// ===========================================================================

const FADE_TRANSITION_SRC: &str = r#"let fade = FadeTransition::new().child(|| {
    Label::derived(|| "Content that fades in / out".to_string())
        .style(|s| s.padding(16.0).font_size(16.0))
});
let fade_show = fade.show_signal();
Button::new("Toggle fade").on_click(move || fade_show.update(|v| *v = !*v)).view()"#;

fn fade_transition_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造淡入淡出过渡。" },
        ApiRow { name: "child", ty: "Fn() -> impl IntoView", default: "—", desc: "被过渡显隐的子视图（闭包构造）。" },
        ApiRow { name: "show_signal", ty: "RwSignal<bool>", default: "—", desc: "控制显隐的信号；true=显示。" },
    ]
}

pub fn fade_transition_showcase() -> AnyView {
    let fade = FadeTransition::new().child(|| {
        Label::derived(|| "Content that fades in / out".to_string())
            .style(|s| s.padding(16.0).font_size(16.0))
    });
    let fade_show = fade.show_signal();
    let comp = Stack::vertical((
        Button::new("Toggle fade")
            .variant(ButtonVariant::Secondary)
            .on_click(move || fade_show.update(|v| *v = !*v))
            .view(),
        fade.view(),
    ))
    .style(|s| s.gap(10.0).items_start());
    let controls = Stack::vertical((
        Label::derived(move || format!("Visible: {}", fade_show.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "FadeTransition",
        "Animate a child's opacity. Click Toggle to fade the content in / out; the readout confirms the state.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Always visible", FadeTransition::new().child(|| Label::new("Pinned").style(|s| s.padding(16.0))).view()),
            code_snippet("FadeTransition · 基础", FADE_TRANSITION_SRC),
            api_table("Props", fade_transition_api()),
        ],
    )
}

// ===========================================================================
// 84. ScaleTransition
// ===========================================================================

const SCALE_TRANSITION_SRC: &str = r#"let scale = ScaleTransition::new().child(|| {
    Label::derived(|| "Content that scales in / out".to_string())
        .style(|s| s.padding(16.0).font_size(16.0))
});
let scale_show = scale.show_signal();
Button::new("Toggle scale").on_click(move || scale_show.update(|v| *v = !*v)).view()"#;

fn scale_transition_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造缩放过渡。" },
        ApiRow { name: "child", ty: "Fn() -> impl IntoView", default: "—", desc: "被过渡显隐的子视图（闭包构造）。" },
        ApiRow { name: "show_signal", ty: "RwSignal<bool>", default: "—", desc: "控制显隐的信号；true=显示。" },
    ]
}

pub fn scale_transition_showcase() -> AnyView {
    let scale = ScaleTransition::new().child(|| {
        Label::derived(|| "Content that scales in / out".to_string())
            .style(|s| s.padding(16.0).font_size(16.0))
    });
    let scale_show = scale.show_signal();
    let comp = Stack::vertical((
        Button::new("Toggle scale")
            .variant(ButtonVariant::Secondary)
            .on_click(move || scale_show.update(|v| *v = !*v))
            .view(),
        scale.view(),
    ))
    .style(|s| s.gap(10.0).items_start());
    let controls = Stack::vertical((
        Label::derived(move || format!("Visible: {}", scale_show.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ScaleTransition",
        "Animate a child's scale. Click Toggle to grow / shrink the content; the readout confirms the state.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Always visible", ScaleTransition::new().child(|| Label::new("Pinned").style(|s| s.padding(16.0))).view()),
            code_snippet("ScaleTransition · 基础", SCALE_TRANSITION_SRC),
            api_table("Props", scale_transition_api()),
        ],
    )
}

// ===========================================================================
// 85. ExpandTransition
// ===========================================================================

const EXPAND_TRANSITION_SRC: &str = r#"let expand = ExpandTransition::new().child(|| {
    Label::derived(|| "Content that expands / collapses".to_string())
        .style(|s| s.padding(16.0).font_size(16.0))
});
let expand_show = expand.show_signal();
Button::new("Toggle expand").on_click(move || expand_show.update(|v| *v = !*v)).view()"#;

fn expand_transition_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造展开/折叠过渡（高度 0 -> 自然高度）。" },
        ApiRow { name: "child", ty: "Fn() -> impl IntoView", default: "—", desc: "被过渡显隐的子视图（闭包构造）。" },
        ApiRow { name: "show_signal", ty: "RwSignal<bool>", default: "—", desc: "控制显隐的信号；true=展开。" },
    ]
}

pub fn expand_transition_showcase() -> AnyView {
    let expand = ExpandTransition::new().child(|| {
        Label::derived(|| "Content that expands / collapses (height 0 -> natural)".to_string())
            .style(|s| s.padding(16.0).font_size(16.0))
    });
    let expand_show = expand.show_signal();
    let comp = Stack::vertical((
        Button::new("Toggle expand")
            .variant(ButtonVariant::Secondary)
            .on_click(move || expand_show.update(|v| *v = !*v))
            .view(),
        expand.view(),
    ))
    .style(|s| s.gap(10.0).items_start());
    let controls = Stack::vertical((
        Label::derived(move || format!("Expanded: {}", expand_show.get()))
            .style(|s| s.font_size(13.0))
            .into_any(),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "ExpandTransition",
        "Animate a child's height. Click Toggle to expand / collapse; the readout confirms the state.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Always visible", ExpandTransition::new().child(|| Label::new("Pinned").style(|s| s.padding(16.0))).view()),
            code_snippet("ExpandTransition · 基础", EXPAND_TRANSITION_SRC),
            api_table("Props", expand_transition_api()),
        ],
    )
}

// ===========================================================================
// 86. SlideTransition
// ===========================================================================

const SLIDE_TRANSITION_SRC: &str = r#"let slide = SlideTransition::new()
    .direction(SlideDirection::X)
    .child(|| {
        Label::derived(|| "Content that slides in / out".to_string())
            .style(|s| s.padding(16.0).font_size(16.0))
    });
let slide_show = slide.show_signal();
Button::new("Toggle slide").on_click(move || slide_show.update(|v| *v = !*v)).view()"#;

fn slide_transition_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new()", ty: "—", default: "—", desc: "构造滑入/滑出过渡。" },
        ApiRow { name: "child", ty: "Fn() -> impl IntoView", default: "—", desc: "被过渡显隐的子视图（闭包构造）。" },
        ApiRow { name: "direction", ty: "SlideDirection", default: "X", desc: "滑动轴：X 水平 / Y 垂直。" },
        ApiRow { name: "show_signal", ty: "RwSignal<bool>", default: "—", desc: "控制显隐的信号；true=显示。" },
    ]
}

pub fn slide_transition_showcase() -> AnyView {
    let direction = RwSignal::new(SlideDirection::X);
    let comp = dyn_view(move || {
        let slide = SlideTransition::new()
            .direction(direction.get())
            .child(|| {
                Label::derived(|| "Content that slides in / out".to_string())
                    .style(|s| s.padding(16.0).font_size(16.0))
            });
        let slide_show = slide.show_signal();
        let btn = Button::new("Toggle slide")
            .variant(ButtonVariant::Secondary)
            .on_click(move || slide_show.update(|v| *v = !*v))
            .view();
        Stack::vertical((btn, slide.view()))
            .style(|s| s.gap(10.0).items_start())
            .into_any()
    });
    let controls = Stack::vertical((
        segmented(
            "Direction",
            vec![("X (horizontal)", SlideDirection::X), ("Y (vertical)", SlideDirection::Y)],
            direction,
        ),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "SlideTransition",
        "Animate a child sliding along an axis. Pick the direction and click Toggle to slide in / out.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo("Always visible", SlideTransition::new().direction(SlideDirection::X).child(|| Label::new("Pinned").style(|s| s.padding(16.0))).  view()),
            code_snippet("SlideTransition · 基础", SLIDE_TRANSITION_SRC),
            api_table("Props", slide_transition_api()),
        ],
    )
}

// ─────────────────────────────────────────────
// Batch 6 — data-display & provider components
// ─────────────────────────────────────────────

const SPARKLINE_SRC: &str = r#"Sparkline::new()
    .values(vec![3.0, 7.0, 4.0, 9.0, 6.0, 11.0, 8.0, 13.0])
    .width(240.0)
    .height(48.0)
    .fill(true)
    .view()"#;

fn sparkline_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "values", ty: "Vec<f64>", default: "[]", desc: "数据点序列" },
        ApiRow { name: "width", ty: "f32", default: "120.0", desc: "绘图宽度" },
        ApiRow { name: "height", ty: "f32", default: "40.0", desc: "绘图高度" },
        ApiRow { name: "fill", ty: "bool", default: "false", desc: "是否填充曲线下方区域" },
    ]
}

pub fn sparkline_showcase() -> AnyView {
    let fill = RwSignal::new(true);
    let height = RwSignal::new(48_i32);
    let comp = {
        Sparkline::new()
            .values(vec![3.0, 7.0, 4.0, 9.0, 6.0, 11.0, 8.0, 13.0])
            .width(240.0)
            .height(height.get() as f32)
            .fill(fill.get())
            .view()
    };
    let controls = Stack::vertical((toggle_row("Fill", fill), slider_int_row("Height", 32, 96, height)))
        .style(|s| s.gap(12.0).items_start());

    showcase(
        "Sparkline",
        "Compact trend chart rendered from a series of numeric values — perfect for inline metrics.",
        vec![
            demo_ctl("Interactive", comp, controls),
            demo(
                "Filled variant",
                Sparkline::new()
                    .values(vec![2.0, 5.0, 3.0, 8.0, 6.0, 10.0])
                    .width(240.0)
                    .height(48.0)
                    .fill(true)
                    .view(),
            ),
            code_snippet("Sparkline · 基础", SPARKLINE_SRC),
            api_table("Props", sparkline_api()),
        ],
    )
}

const DATA_ITERATOR_SRC: &str = r#"DataIterator::new()
    .divided(true)
    .item(Text::new("Record 1 — updated 2m ago").view())
    .item(Text::new("Record 2 — updated 14m ago").view())
    .view()"#;

fn data_iterator_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "item", ty: "impl IntoView", default: "—", desc: "追加一行数据项" },
        ApiRow { name: "divided", ty: "bool", default: "false", desc: "是否在行间绘制分隔线" },
    ]
}

pub fn data_iterator_showcase() -> AnyView {
    let divided = RwSignal::new(true);
    let comp = {
        DataIterator::new()
            .divided(  divided.get())
            .item(Text::new("Record 1 — updated 2m ago").view())
            .item(Text::new("Record 2 — updated 14m ago").view())
            .item(Text::new("Record 3 — updated 1h ago").view())
            .view()
    };
    let controls = Stack::vertical((toggle_row("Divided", divided),)).style(|s| s.gap(12.0).items_start());

    showcase(
        "DataIterator",
        "A dense, optionally-divided list of rows — a lightweight alternative to a full table.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("DataIterator · 基础", DATA_ITERATOR_SRC),
            api_table("Props", data_iterator_api()),
        ],
    )
}

const RESPONSIVE_SRC: &str = r#"Responsive::new(16.0 / 9.0)
    .child(|| Text::new("16:9 responsive box").view())
    .view()"#;

fn responsive_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "new(aspect_ratio)", ty: "f32", default: "—", desc: "宽高比（宽/高）" },
        ApiRow { name: "child", ty: "Fn() -> AnyView", default: "—", desc: "等比缩放的内容" },
    ]
}

pub fn responsive_showcase() -> AnyView {
    let comp = {
        Responsive::new(16.0 / 9.0)
            .child(|| Text::new("16:9 responsive box").view().style(|s| s.font_size(14.0)).into_any())
            .view()
    };

    showcase(
        "Responsive",
        "Maintain a fixed aspect-ratio box that scales with its container width.",
        vec![
            demo("16:9", comp),
            code_snippet("Responsive · 基础", RESPONSIVE_SRC),
            api_table("Props", responsive_api()),
        ],
    )
}

const CONFIRM_EDIT_SRC: &str = r#"ConfirmEdit::new()
    .value("Click the ✎ to edit this text")
    .on_save(move |s: String| { /* persist s */ })
    .child(|| Text::new("Click the ✎ to edit this text").view())
    .view()"#;

fn confirm_edit_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "value", ty: "impl Into<String>", default: "—", desc: "初始文本" },
        ApiRow { name: "child", ty: "Fn() -> AnyView", default: "—", desc: "可编辑区域内容" },
        ApiRow { name: "on_save", ty: "Fn(String)", default: "—", desc: "保存时的回调" },
    ]
}

pub fn confirm_edit_showcase() -> AnyView {
    let saved = RwSignal::new(String::from("Click the ✎ to edit this text"));
    let comp = {
        ConfirmEdit::new()
            .value("Click the ✎ to edit this text")
            .on_save(move |s: String| saved.set(s))
            .child(|| Text::new("Click the ✎ to edit this text").view().style(|s| s.font_size(14.0)).into_any())
            .view()
    };
    let label = Label::derived(move || format!("saved value: {}", saved.get()))
        .style(|s| s.font_size(14.0))
        .into_any();
    let controls = Stack::vertical((label,)).style(|s| s.gap(12.0).items_start());

    showcase(
        "ConfirmEdit",
        "Inline editable text: click the edit affordance to modify, then save to commit.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("ConfirmEdit · 基础", CONFIRM_EDIT_SRC),
            api_table("Props", confirm_edit_api()),
        ],
    )
}

const DISPLAY_PROVIDER_SRC: &str = r#"DisplayProvider::new()
    .dense(false)
    .view()"#;

fn display_provider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "dense", ty: "bool", default: "false", desc: "紧凑（高密度）模式" },
        ApiRow { name: "density_signal", ty: "fn -> RwSignal<bool>", default: "—", desc: "读取当前密度信号" },
    ]
}

pub fn display_provider_showcase() -> AnyView {
    let dense = RwSignal::new(false);
    let comp = {
        let dp = DisplayProvider::new().dense(dense.get());
        let sig = dp.density_signal();
        Stack::vertical((
            Label::derived(move || format!("dense = {}", sig.get())).style(|s| s.font_size(14.0)).into_any(),
            dp.view().into_any(),
        ))
        .style(|s| s.gap(8.0).items_start())
    };
    let controls = Stack::vertical((toggle_row("Dense", dense),)).style(|s| s.gap(12.0).items_start());

    showcase(
        "DisplayProvider",
        "Sets the global density (comfortable / compact) for descendant components via the layout signal.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("DisplayProvider · 基础", DISPLAY_PROVIDER_SRC),
            api_table("Props", display_provider_api()),
        ],
    )
}

const DEFAULTS_PROVIDER_SRC: &str = r#"DefaultsProvider::new()
    .defaults(HashMap::new())
    .view()"#;

fn defaults_provider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "defaults", ty: "HashMap<String, String>", default: "{}", desc: "默认属性映射" },
        ApiRow { name: "defaults_signal", ty: "fn -> RwSignal<HashMap>", default: "—", desc: "读取默认属性信号" },
    ]
}

pub fn defaults_provider_showcase() -> AnyView {
    let comp = {
        let dp = DefaultsProvider::new().defaults(HashMap::new());
        let sig = dp.defaults_signal();
        Stack::  vertical((
            Label::derived(move || format!("defaults keys = {}", sig.get().len())).style(|s| s.font_size(14.0)).into_any(),
            dp.view().into_any(),
        ))
        .style(|s| s.gap(8.0).items_start())
    };

    showcase(
        "DefaultsProvider",
        "Provides a bag of default prop overrides inherited by descendant components.",
        vec![
            demo("Basic", comp),
            code_snippet("DefaultsProvider · 基础", DEFAULTS_PROVIDER_SRC),
            api_table("Props", defaults_provider_api()),
        ],
    )
}

const ICONS_PROVIDER_SRC: &str = r#"IconsProvider::new()
    .icons(HashMap::new())
    .view()"#;

fn icons_provider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "icons", ty: "HashMap<String, String>", default: "{}", desc: "图标名 → 资源映射" },
        ApiRow { name: "icons_signal", ty: "fn -> RwSignal<HashMap>", default: "—", desc: "读取图标映射信号" },
    ]
}

pub fn icons_provider_showcase() -> AnyView {
    let comp = {
        let dp = IconsProvider::new().icons(HashMap::new());
        let sig = dp.icons_signal();
        Stack::vertical((
            Label::derived(move || format!("icons keys = {}", sig.get().len())).style(|s| s.font_size(14.0)).into_any(),
            dp.view().into_any(),
        ))
        .style(|s| s.gap(8.0).items_start())
    };

    showcase(
        "IconsProvider",
        "Supplies a name → resource icon map consumed by icon-aware descendant components.",
        vec![
            demo("Basic", comp),
            code_snippet("IconsProvider · 基础", ICONS_PROVIDER_SRC),
            api_table("Props", icons_provider_api()),
        ],
    )
}

const DATE_ADAPTER_PROVIDER_SRC: &str = r#"DateAdapterProvider::new()
    .adapter("native")
    .view()"#;

fn date_adapter_provider_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "adapter", ty: "impl Into<String>", default: "native", desc: "日期适配器标识" },
        ApiRow { name: "adapter_signal", ty: "fn -> RwSignal<String>", default: "—", desc: "读取适配器信号" },
    ]
}

pub fn date_adapter_provider_showcase() -> AnyView {
    let adapter: RwSignal<&'static str> = RwSignal::new("native");
    let comp = {
        let dp = DateAdapterProvider::new().adapter(adapter.get());
        let sig = dp.adapter_signal();
        Stack::vertical((
            Label::derived(move || format!("date adapter = {}", sig.get())).style(|s| s.font_size(14.0)).into_any(),
            dp.view().into_any(),
        ))
        .style(|s| s.gap(8.0).items_start())
    };
    let controls = Stack::vertical((
        segmented(
            "Adapter",
            vec![("native", "native"), ("custom", "custom")],
            adapter,
        ),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "DateAdapterProvider",
        "Selects the date-formatting / picker adapter used by descendant date components.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("DateAdapterProvider · 基础", DATE_ADAPTER_PROVIDER_SRC),
            api_table("Props", date_adapter_provider_api()),
        ],
    )
}

const PARALLAX_SRC: &str = r#"Parallax::new()
    .factor(0.4)
    .scroll_signal(scroll)
    .child(|s| /* background layer */)
    .view()"#;

fn parallax_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "factor", ty: "f32", default: "0.0", desc: "视差强度系数" },
        ApiRow { name: "scroll_signal", ty: "RwSignal<f32>", default: "—", desc: "驱动位移的滚动量" },
        ApiRow { name: "child", ty: "Fn() -> AnyView", default: "—", desc: "视差层内容" },
    ]
}

pub fn parallax_showcase() -> AnyView {
    let scroll = RwSignal::new(0.0_f32);
    let factor_i = RwSignal::new(40_i32);
    let comp = {
        Parallax::new()
            .factor(factor_i.get() as f32)
            .scroll_signal(scroll)
            .child(
                Label::derived(move || {
                    format!(
                        "Background · translated by -scroll×factor = {:.0}px",
                        -scroll.get() * factor_i.get() as f32
                    )
                })
                .style(|s| s.font_size(22.0).padding(28.0))
                .into_any(),
            )
            .view()
    };
    let controls = Stack::vertical((
        Slider::new()
            .value(0.0)
            .min(0.0)
            .max(300.0)
            .step(1.0)
            .on_change(move |v: f32| scroll.set(v))
            .view(),
        Label::derived(move || format!("scroll = {:.0}", scroll.get())).style(|s| s.font_size(13.0)),
    ))
    .style(|s| s.gap(12.0).items_start());

    showcase(
        "Parallax",
        "Offset a background layer against a scroll signal to create a depth effect.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("Parallax · 基础", PARALLAX_SRC),
            api_table("Props", parallax_api()),
        ],
    )
}

const VALIDATION_MESSAGES_SRC: &str = r#"let vm = ValidationMessages::new();
let vm_sig = vm.messages_signal();
// toggle: vm_sig.set(vec!["Name is required.".into()]);
vm.view()"#;

fn validation_messages_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "messages", ty: "Vec<String>", default: "[]", desc: "错误信息列表" },
        ApiRow { name: "messages_signal", ty: "fn -> RwSignal<Vec<String>>", default: "—", desc: "读取/写入错误信息信号" },
    ]
}

pub fn validation_messages_showcase() -> AnyView {
    let comp = {
        let vm = ValidationMessages::new();
        let vm_sig = vm.messages_signal();
        let toggle = Button::new("Toggle errors")
            .variant(ButtonVariant::Secondary)
            .on_click(move || {
                if vm_sig.get().is_empty() {
                    vm_sig.set(vec!["Name is required.".to_string()]);
                } else {
                    vm_sig.set(vec![]);
                }
            })
            .view()
            .into_any();
        Stack::vertical((toggle, vm.view().into_any()))
            .style(|s| s.gap(8.0).items_start())
    };
    let note = Text::new("点击按钮切换错误信息的显示与隐藏").view().style(|s| s.font_size(13.0)).into_any();
    let controls = Stack::vertical((note,)).style(|s| s.gap(12.0).items_start());

    showcase(
        "ValidationMessages",
        "Displays a stack of validation error strings, driven reactively by a signal.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("ValidationMessages · 基础", VALIDATION_MESSAGES_SRC),
            api_table("Props", validation_messages_api()),
        ],
    )
}

const WINDOW_SRC: &str = r#"Window::new()
    .items(slides)
    .height(160.0)
    .selected(0)
    .show_arrows(true)
    .show_dots(true)
    .on_change(|i| { /* current index */ })
    .view()"#;

fn window_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "items", ty: "Vec<AnyView>", default: "—", desc: "子页面视图" },
        ApiRow { name: "selected", ty: "usize", default: "0", desc: "初始选中页" },
        ApiRow { name: "show_arrows", ty: "bool", default: "false", desc: "是否显示左右箭头" },
        ApiRow { name: "show_dots", ty: "bool", default: "false", desc: "是否显示页码圆点" },
        ApiRow { name: "on_change", ty: "Fn(usize)", default: "—", desc: "切换回调" },
    ]
}

pub fn window_showcase() -> AnyView {
    let show_arrows = RwSignal::new(true);
    let show_dots = RwSignal::new(true);
    let selected = RwSignal::new(0usize);
    let comp = {
        let slides: Vec<AnyView> = (0..3)
            .map(|i| Label::new(format!("Slide {i}")).style(|s| s.font_size(16.0).padding(24.0)).into_any())
            .collect();
        Window::new()
            .items(slides)
            .height(160.0)
            .selected(selected.get())
            .show_arrows(  show_arrows.get())
            .show_dots(show_dots.get())
            .on_change(move |i: usize| selected.set(i))
            .view()
    };
    let controls = Stack::vertical((toggle_row("Show arrows", show_arrows), toggle_row("Show dots", show_dots)))
        .style(|s| s.gap(12.0).items_start());

    showcase(
        "Window",
        "A paged viewport showing one of several views at a time, with optional arrows / dots.",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("Window · 基础", WINDOW_SRC),
            api_table("Props", window_api()),
        ],
    )
}

const CODE_SRC: &str = r#"Code::new()
    .content("let x = 42;")
    .view()"#;

fn code_api() -> Vec<ApiRow> {
    vec![
        ApiRow { name: "content", ty: "impl Into<String>", default: "—", desc: "要高亮展示的代码文本" },
    ]
}

pub fn code_showcase() -> AnyView {
    let comp = Code::new().content("let x = 42;").view();

    showcase(
        "Code",
        "Renders a snippet of source text with monospace styling and a code surface.",
        vec![
            demo("Basic", comp),
            code_snippet("Code · 基础", CODE_SRC),
            api_table("Props", code_api()),
        ],
    )
}

// ===========================================================================
// 50. VirtualScroller
// ===========================================================================

const VIRTUAL_SCROLLER_SRC: &str = r#"let items: Vec<Box<dyn Fn() -> Box<dyn View>>> = (0..50)
    .map(|i| {
        Box::new(move || -> Box<dyn View> {
            Box::new(Label::new(format!("Row {}", i)).into_any())
        }) as Box<dyn Fn() -> Box<dyn View>>
    })
    .collect();

VirtualScroller::new()
    .items(items)
    .height(260.0)
    .on_reach_end(|| log::info!("reached end"))
    .view()"#;

fn virtual_scroller_api() -> Vec<ApiRow> {
    vec![
        ApiRow {
            name: "items",
            ty: "Vec<ItemFactory>",
            default: "—",
            desc: "行工厂集合，每个工厂惰性生成一行视图（只渲染可视区）。",
        },
        ApiRow {
            name: "height",
            ty: "f32",
            default: "0.0",
            desc: "虚拟列表的可视高度（像素）。",
        },
        ApiRow {
            name: "on_reach_end",
            ty: "Fn()",
            default: "—",
            desc: "滚动到底部时触发，用于懒加载更多。",
        },
    ]
}

pub fn virtual_scroller_showcase() -> AnyView {
    let count = RwSignal::new(50i32);
    let reached = RwSignal::new(0usize);

    let comp = {
        let count = count;
        let reached = reached;
        dyn_view(move || {
            let n = count.get().max(1) as usize;
            let items: Vec<Box<dyn Fn() -> Box<dyn View>>> = (0..n)
                .map(|i| {
                    Box::new(move || -> Box<dyn View> {
                        Box::new(
                            Label::new(format!("Virtual row item {}", i))
                                .style(move |s| {
                                    let t = theme_signal().get();
                                    s.padding_horiz(16.0)
                                        .font_size(14.0)
                                        .color(
                                            floom_core_theme::theme::Theme::colors(&t)
                                                .on_surface
                                                .to_peniko(),
                                        )
                                })
                                .into_any(),
                        )
                    }) as Box<dyn Fn() -> Box<dyn View>>
                })
                .collect();
            VirtualScroller::new()
                .items(items)
                .height(260.0)
                .on_reach_end(move || reached.set(reached.get() + 1))
                .view()
                .into_any()
        })
    };

    let controls = {
        let count = count;
        let reached = reached;
        Stack::vertical((
            slider_int_row("Row count", 5, 200, count),
            Label::derived(move || format!("on_reach_end fired: {}×", reached.get()))
                .style(|s| s.font_size(12.0)),
        ))
        .style(|s| s.gap(12.0).items_start())
    };

    showcase(
        "VirtualScroller",
        "虚拟化滚动容器：仅渲染可视区域内的行，适合超长列表。",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("VirtualScroller · 基础", VIRTUAL_SCROLLER_SRC),
            api_table("Props", virtual_scroller_api()),
        ],
    )
}

// ===========================================================================
// 51. App
// ===========================================================================

const APP_SRC: &str = r#"let app_shell = App::new()
    .child(shell_inner)  // AppBar + Main + Footer + NavigationDrawer
    .view();"#;

fn app_api() -> Vec<ApiRow> {
    vec![ApiRow {
        name: "child",
        ty: "impl IntoView",
        default: "—",
        desc: "应用根内容（shell：AppBar + Main + Footer + NavigationDrawer）。",
    }]
}

pub fn app_showcase() -> AnyView {
    #[cfg(feature = "multi")]
    let shell_drawer = NavigationDrawer::new()
        .header(|| Text::new("Navigation").view())
        .child(|| {
            List::new()
                .item("Home", || {})
                .item("Components", || {})
                .item("About", || {})
                .view()
        });
    #[cfg(not(feature = "multi"))]
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

    let app_bar_overlay = floom::views::Container::new(
        AppBar::new()
            .title("App + Main shell")
            .leading(Icon::new("menu").view())
            .actions(vec![Icon::new("search").view().into_any()])
            .view()
            .into_any(),
    )
    .style(|s| s.absolute().inset_top(0.0).inset_left(0.0).width_full());

    let footer_overlay = floom::views::Container::new(
        Footer::new()
            .bordered(true)
            .child(
                Stack::horizontal((
                    Text::new("© 2026 Acme")
                        .view()
                        .style(|s| s.font_size(12.0))
                        .into_any(),
                    Spacer::new().grow(1.0).view().into_any(),
                    Text::new("Docs")
                        .view()
                        .style(|s| s.font_size(12.0))
                        .into_any(),
                ))
                .style(|s| s.gap(12.0).width_pct(100.0)),
            )
            .view()
            .into_any(),
    )
    .style(|s| s.absolute().inset_bottom(0.0).inset_left(0.0).width_full());

    let main_content = Stack::vertical((
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

    let shell_inner = Stack::vertical((
        Main::new().child(main_content).view().into_any(),
        app_bar_overlay.into_any(),
        footer_overlay.into_any(),
        shell_drawer_view.into_any(),
    ))
    .style(|s| s.width_pct(100.0).height_pct(100.0));

    let app_shell = App::new().child(shell_inner).view();

    let comp = floom::views::Container::new(app_shell)
        .style(|s| s.height(460.0).width_pct(100.0))
        .into_any();

    let controls = Label::new("点击 “Toggle drawer” 打开 / 关闭导航抽屉。")
        .style(|s| s.font_size(12.0))
        .into_any();

    showcase(
        "App",
        "应用根容器：承载 AppBar、Main、Footer 与 NavigationDrawer 的整体布局框架。",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("App · 基础", APP_SRC),
            api_table("Props", app_api()),
        ],
    )
}

// ===========================================================================
// 52. Main
// ===========================================================================

const MAIN_SRC: &str = r#"let main_view = Main::new()
    .child(content_stack)
    .view();"#;

fn main_api() -> Vec<ApiRow> {
    vec![ApiRow {
        name: "child",
        ty: "impl IntoView",
        default: "—",
        desc: "主内容区。自动从 AppBar / Footer / 抽屉读取布局信号做内边距避让。",
    }]
}

pub fn main_showcase() -> AnyView {
    let comp = Main::new()
        .child(
            Stack::vertical((
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

    let controls = Label::new("Main 随布局信号自动避让顶部栏与底部栏。")
        .style(|s| s.font_size(12.0))
        .into_any();

    showcase(
        "Main",
        "主内容区域：依据全局布局信号自动从 AppBar、Footer 与打开的抽屉处做内边距避让。",
        vec![
            demo("Basic", comp),
            code_snippet("Main · 基础", MAIN_SRC),
            api_table("Props", main_api()),
        ],
    )
}

// ===========================================================================
// 81. InfiniteScroller
// ===========================================================================

const INFINITE_SCROLLER_SRC: &str = r#"let scroller = InfiniteScroller::new()
    .viewport(240.0)
    .content_height_signal(content_h)
    .on_load_more(|| append_more())
    .child(content_view)
    .view();"#;

fn infinite_scroller_api() -> Vec<ApiRow> {
    vec![
        ApiRow {
            name: "viewport",
            ty: "f32",
            default: "—",
            desc: "可视视口高度（像素）。",
        },
        ApiRow {
            name: "content_height_signal",
            ty: "RwSignal<f32>",
            default: "—",
            desc: "内容总高度信号，须随每次加载更新。",
        },
        ApiRow {
            name: "on_load_more",
            ty: "Fn()",
            default: "—",
            desc: "滚动接近底部时触发，用于追加内容并刷新高度信号。",
        },
        ApiRow {
            name: "child",
            ty: "impl IntoView",
            default: "—",
            desc: "内容视图（通常是一个 dyn_view）。",
        },
        ApiRow {
            name: "loading_signal",
            ty: "RwSignal<bool>",
            default: "—",
            desc: "读取加载态，避免在加载中重复触发。",
        },
    ]
}

pub fn infinite_scroller_showcase() -> AnyView {
    let theme = theme_signal();
    let inf_count = RwSignal::new(15usize);
    let inf_item_h = 36.0;
    let inf_content_h = RwSignal::new(15.0 * inf_item_h);

    let inf_child = {
        let inf_count = inf_count;
        let theme = theme;
        dyn_view(move || {
            let n = inf_count.get();
            let mut rows: Vec<AnyView> = Vec::new();
            for i in 0..n {
                let row = Container::new()
                    .child(
                        Label::derived(move || format!("Row {}", i))
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
                                    floom_core_theme::theme::Theme::colors(&theme.get())
                                        .on_surface,
                                    if i % 2 == 0 { 0 } else { 6 },
                                )
                                .to_peniko(),
                            )
                    });
                rows.push(row.into_any());
            }
            Stack::vertical_from_iter(rows)
                .style(|s| s.width_pct(100.0))
                .into_any()
        })
    };

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
    let inf_status = Label::derived(move || {
        format!(
            "{} rows loaded · loading: {}  (scroll to the bottom to append 15 more)",
            inf_count.get(),
            inf_loading_sig.get()
        )
    });

    let comp = Stack::vertical((inf_scroller_view, inf_status))
        .style(|s| s.gap(8.0).items_start())
        .into_any();

    let controls = Label::new("向下滚动到底部即可自动追加 15 行。")
        .style(|s| s.font_size(12.0))
        .into_any();

    showcase(
        "InfiniteScroller",
        "无限滚动容器：滚动接近底部时触发 on_load_more 追加内容，适合分页 / 流式加载。",
        vec![
            demo_ctl("Interactive", comp, controls),
            code_snippet("InfiniteScroller · 基础", INFINITE_SCROLLER_SRC),
            api_table("Props", infinite_scroller_api()),
        ],
    )
}
