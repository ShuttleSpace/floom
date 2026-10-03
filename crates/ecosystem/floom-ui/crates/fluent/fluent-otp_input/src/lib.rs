use std::rc::Rc;

use floom::prelude::*;
use floom::view::AnyView;
use floom::views::{Decorators, TextInput, Stack};
use floom::reactive::{RwSignal, SignalGet, create_effect};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::otp_input::OtpInputProps;
use floom_core_theme::theme::{MultiTheme, Theme};

/// Material 3 OTP input: `length` single-character boxes bound to a master
/// `code` string. Typing in a box writes that character into the code; the
/// host can also read/write the whole code through `code_signal()`. When
/// `masked`, the visible glyph is replaced by a dot while the real value is
/// retained in `code`.
pub struct FluentOtpInput {
    props: OtpInputProps,
    theme: RwSignal<MultiTheme>,
    code: RwSignal<String>,
}

impl FluentOtpInput {
    pub fn new() -> Self {
        Self {
            props: OtpInputProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            code: RwSignal::new(String::new()),
        }
    }

    pub fn length(mut self, length: usize) -> Self {
        self.props.length = length;
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.props.masked = masked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &OtpInputProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    /// Reactive source of truth for the whole code; the host can read or
    /// overwrite it and the boxes re-sync.
    pub fn code_signal(&self) -> RwSignal<String> {
        self.code
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let length = self.props.length.max(1);
        let masked = self.props.masked;
        let disabled = RwSignal::new(self.props.disabled);
        let on_change: Rc<Option<Box<dyn Fn(String)>>> = Rc::new(self.props.on_change);
        let code = self.code;

        let tokens = match theme.get().component(ComponentId::OtpInput) {
            ComponentTokens::OtpInput(t) => t,
            _ => unreachable!(),
        };
        let box_size = tokens.box_size;
        let radius = tokens.border_radius;
        let gap = tokens.gap;
        let font_size = tokens.font_size;

        // Per-box signals: `real` holds the true char, `shown` is what the
        // TextInput renders (a dot when masked).
        let reals: Vec<RwSignal<String>> = (0..length)
            .map(|i| {
                RwSignal::new(
                    code.get()
                        .chars()
                        .nth(i)
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                )
            })
            .collect();
        let showns: Vec<RwSignal<String>> = reals
            .iter()
            .map(|r| {
                let ch = r.get();
                RwSignal::new(if masked && !ch.is_empty() {
                    "•".to_string()
                } else {
                    ch
                })
            })
            .collect();

        // box -> code (user typing)
        for i in 0..length {
            let reals = reals.clone();
            let showns = showns.clone();
            let code = code;
            let on_change = on_change.clone();
            let shown = showns[i].clone();
            let real = reals[i].clone();
            create_effect(move |_| {
                let shown_val = shown.get();
                let ch = shown_val.chars().next().map(|c| c.to_string()).unwrap_or_default();
                if ch != "•" {
                    if ch != real.get() {
                        real.set(ch.clone());
                    }
                    let new_code: String = reals
                        .iter()
                        .map(|r| r.get().chars().next().unwrap_or('\0').to_string())
                        .collect::<String>()
                        .chars()
                        .filter(|&c| c != '\0')
                        .collect();
                    if new_code != code.get() {
                        code.set(new_code.clone());
                        if let Some(cb) = on_change.as_ref() {
                            cb(new_code);
                        }
                    }
                }
                // NOTE: do NOT write `shown` here. `shown` is the TextInput's own
                // buffer and is derived solely from `code` by the `code -> boxes`
                // effect below. Writing it from this effect (the old mask
                // normalization) made the same signal be read+written by two
                // cross-subscribed effects, which under floom's synchronous
                // re-entrant effect dispatch recursed without bound (stack
                // overflow crash). Keeping this effect read-only on `shown`
                // makes the binding converge.
            });
        }

        // code -> boxes (external set / back-sync), guarded
        {
            let reals = reals.clone();
            let showns = showns.clone();
            create_effect(move |_| {
                let c = code.get();
                for i in 0..length {
                    let ch = c.chars().nth(i).map(|x| x.to_string()).unwrap_or_default();
                    if ch != reals[i].get() {
                        reals[i].set(ch.clone());
                    }
                    let target = if masked && !ch.is_empty() {
                        "•".to_string()
                    } else {
                        ch
                    };
                    if target != showns[i].get() {
                        showns[i].set(target);
                    }
                }
            });
        }

        let kids: Vec<AnyView> = (0..length)
            .map(|i| {
                let theme = theme;
                let disabled = disabled;
                let value = showns[i].clone();
                Container::new(
                    TextInput::new(value)
                        .placeholder("")
                        .style(move |s| {
                            let t = theme.get();
                            s.color(t.colors().on_surface.to_peniko())
                                .font_size(font_size)
                                .width_pct(100.0)
                                .justify_center()
                                .items_center()
                        })
                        .style(move |s| s.set_disabled(disabled.get())),
                )
                .style(move |s| {
                    let t = theme.get();
                    s.size(box_size, box_size)
                        .border(1.0)
                        .border_color(t.colors().outline.to_peniko())
                        .border_radius(radius)
                        .background(t.colors().surface.to_peniko())
                        .items_center()
                        .justify_center()
                })
                .into_any()
            })
            .collect();

        Stack::horizontal_from_iter(kids).style(move |s| s.gap(gap).items_center())
    }
}

impl Default for FluentOtpInput {
    fn default() -> Self {
        Self::new()
    }
}
