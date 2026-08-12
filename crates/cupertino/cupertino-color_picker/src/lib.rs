use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem::taffy::style::Overflow;
use floem::view::AnyView;
use floem::views::{Container, Decorators, Empty, Stack, TextInput, svg};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::color_picker::ColorPickerProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;


// ----- HSV <-> RGB helpers (pure; safe to swap across design systems) -----

fn rgb_to_hsv(c: Color) -> (f32, f32, f32) {
    let r = c.r as f32 / 255.0;
    let g = c.g as f32 / 255.0;
    let b = c.b as f32 / 255.0;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let v = max;
    let s = if max <= 1e-6 { 0.0 } else { d / max };
    let h = if d <= 1e-6 {
        0.0
    } else if max == r {
        ((g - b) / d) % 6.0
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    let h = h * 60.0;
    let h = if h < 0.0 { h + 360.0 } else { h };
    (h, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Color {
    let h = (h % 360.0 + 360.0) % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    Color::rgb(
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

fn color_to_hex(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b)
}

fn hex_to_color(s: &str) -> Option<Color> {
    let s = s.trim().trim_start_matches('#');
    let s = if s.len() == 8 { &s[0..6] } else { s };
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Color::rgb(r, g, b))
}

fn build_sv(hue: Color, w: f32, h: f32) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>\
         <defs>\
         <linearGradient id='ws' x1='0' y1='0' x2='1' y2='0'>\
         <stop offset='0' stop-color='white'/><stop offset='1' stop-color='white' stop-opacity='0'/></linearGradient>\
         <linearGradient id='vb' x1='0' y1='0' x2='0' y2='1'>\
         <stop offset='0' stop-color='black' stop-opacity='0'/><stop offset='1' stop-color='black'/></linearGradient>\
         </defs>\
         <rect width='{w}' height='{h}' fill='{hue_c}'/>\
         <rect width='{w}' height='{h}' fill='url(#ws)'/>\
         <rect width='{w}' height='{h}' fill='url(#vb)'/></svg>",
        w = w,
        h = h,
        hue_c = hue.to_css()
    )
}

fn build_hue(w: f32, h: f32, r: f32) -> String {
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>\
         <defs><linearGradient id='hue' x1='0' y1='0' x2='1' y2='0'>\
         <stop offset='0' stop-color='#ff0000'/>\
         <stop offset='0.167' stop-color='#ffff00'/>\
         <stop offset='0.333' stop-color='#00ff00'/>\
         <stop offset='0.5' stop-color='#00ffff'/>\
         <stop offset='0.667' stop-color='#0000ff'/>\
         <stop offset='0.833' stop-color='#ff00ff'/>\
         <stop offset='1' stop-color='#ff0000'/></linearGradient></defs>\
         <rect width='{w}' height='{h}' rx='{r}' fill='url(#hue)'/></svg>",
        w = w,
        h = h,
        r = r
    )
}

fn build_alpha(c: Color, w: f32, h: f32, r: f32) -> String {
    let a0 = Color::rgba(c.r, c.g, c.b, 0).to_css();
    let a1 = Color::rgba(c.r, c.g, c.b, 255).to_css();
    format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' viewBox='0 0 {w} {h}'>\
         <defs><linearGradient id='al' x1='0' y1='0' x2='1' y2='0'>\
         <stop offset='0' stop-color='{a0}'/><stop offset='1' stop-color='{a1}'/></linearGradient></defs>\
         <rect width='{w}' height='{h}' rx='{r}' fill='url(#al)'/></svg>",
        w = w,
        h = h,
        r = r,
        a0 = a0,
        a1 = a1
    )
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoColorPickerStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub outline: Color,
    pub on_surface: Color,
    pub sv_width: f32,
    pub sv_height: f32,
    pub slider_height: f32,
    pub thumb_size: f32,
    pub radius: f32,
}

pub struct CupertinoColorPicker {
    props: ColorPickerProps,
    theme: RwSignal<MultiTheme>,
    color: RwSignal<Color>,
    hex: RwSignal<String>,
    on_change: Option<Box<dyn Fn(Color)>>,
}

impl CupertinoColorPicker {
    pub fn new() -> Self {
        let c = Color::rgb(33, 150, 243);
        Self {
            props: ColorPickerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            color: RwSignal::new(c),
            hex: RwSignal::new(color_to_hex(c)),
            on_change: None,
        }
    }

    pub fn value(mut self, value: Color) -> Self {
        self.color = RwSignal::new(value);
        self.hex = RwSignal::new(color_to_hex(value));
        self
    }

    pub fn show_alpha(mut self, show_alpha: bool) -> Self {
        self.props.show_alpha = show_alpha;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(Color) + 'static) -> Self {
        self.on_change = Some(Box::new(callback));
        self
    }

    pub fn color_signal(&self) -> RwSignal<Color> {
        self.color
    }

    pub fn resolved_style(&self) -> CupertinoColorPickerStyle {
        let theme = self.theme.get();
        let ComponentTokens::ColorPicker(tokens) =
            theme.component(ComponentId::ColorPicker)
        else {
            unreachable!("ComponentId::ColorPicker must resolve to ColorPickerTokens")
        };
        CupertinoColorPickerStyle {
            design_system: "cupertino",
            surface: theme.colors().surface,
            outline: theme.colors().outline,
            on_surface: theme.colors().on_surface,
            sv_width: tokens.sv_width,
            sv_height: tokens.sv_height,
            slider_height: tokens.slider_height,
            thumb_size: tokens.thumb_size,
            radius: tokens.radius,
        }
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let color = self.color;
        let hex = self.hex;
        let on_change: Rc<Option<Box<dyn Fn(Color)>>> = Rc::new(self.on_change);
        let show_alpha = self.props.show_alpha;

        let tokens = match theme.get().component(ComponentId::ColorPicker) {
            ComponentTokens::ColorPicker(t) => t,
            _ => unreachable!(),
        };
        let sv_w = tokens.sv_width;
        let sv_h = tokens.sv_height;
        let slider_h = tokens.slider_height;
        let thumb = tokens.thumb_size;
        let radius = tokens.radius;

        let commit = Rc::new({
            let color = color;
            let hex = hex;
            let on_change = on_change.clone();
            move |c: Color| {
                color.set(c);
                hex.set(color_to_hex(c));
                if let Some(cb) = on_change.as_ref() {
                    cb(c);
                }
            }
        });

        // --- SV square ---
        let sv_svg = svg(move || {
            let (h, _, _) = rgb_to_hsv(color.get());
            build_sv(hsv_to_rgb(h, 1.0, 1.0), sv_w, sv_h)
        })
        .update_value(move || {
            let (h, _, _) = rgb_to_hsv(color.get());
            build_sv(hsv_to_rgb(h, 1.0, 1.0), sv_w, sv_h)
        })
        .style(move |s| s.absolute().width_pct(100.0).height_pct(100.0));

        let sv_thumb = Container::new(Empty::new()).style(move |s| {
            let (_, sat, val) = rgb_to_hsv(color.get());
            let cur = color.get();
            s.absolute()
                .size(thumb, thumb)
                .inset_left((sat * sv_w - thumb / 2.0) as f64)
                .inset_top(((1.0 - val) * sv_h - thumb / 2.0) as f64)
                .border_radius(9999.0)
                .border(2.0)
                .border_color(WHITE.to_peniko())
                .background(cur.to_peniko())
        });

        let mut sv_area = Stack::new((sv_svg.into_any(), sv_thumb.into_any())).style(move |s| {
            s.width(sv_w)
                .height(sv_h)
                .border_radius(radius)
                .overflow_x(Overflow::Clip)
                .overflow_y(Overflow::Clip)
                .cursor(CursorStyle::Pointer)
        });
        let sv_id = sv_area.id();
        let sv_dragging = RwSignal::new(false);
        sv_area = sv_area
            .on_event(listener::PointerDown, {
                let commit = commit.clone();
                let sv_id = sv_id;
                let color = color;
                let dragging = sv_dragging;
                move |cx, pbe| {
                    if let Some(sz) = sv_id.get_size() {
                        let w = sz.width as f32;
                        let h = sz.height as f32;
                        if w > 0.0 && h > 0.0 {
                            let p = pbe.state.logical_point();
                            let sat = (p.x as f32 / w).clamp(0.0, 1.0);
                            let val = (1.0 - (p.y as f32 / h)).clamp(0.0, 1.0);
                            let (hh, _, _) = rgb_to_hsv(color.get());
                            let mut c = hsv_to_rgb(hh, sat, val);
                            c.a = color.get().a;
                            commit(c);
                        }
                    }
                    if let Some(pid) = pbe.pointer.pointer_id {
                        cx.request_pointer_capture(pid);
                    }
                    dragging.set(true);
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerMove, {
                let commit = commit.clone();
                let sv_id = sv_id;
                let color = color;
                let dragging = sv_dragging;
                move |_, pu| {
                    if dragging.get() {
                        if let Some(sz) = sv_id.get_size() {
                            let w = sz.width as f32;
                            let h = sz.height as f32;
                            if w > 0.0 && h > 0.0 {
                                let p = pu.current.logical_point();
                                let sat = (p.x as f32 / w).clamp(0.0, 1.0);
                                let val = (1.0 - (p.y as f32 / h)).clamp(0.0, 1.0);
                                let (hh, _, _) = rgb_to_hsv(color.get());
                                let mut c = hsv_to_rgb(hh, sat, val);
                                c.a = color.get().a;
                                commit(c);
                            }
                        }
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerUp, {
                let dragging = sv_dragging;
                move |_, _| {
                    dragging.set(false);
                    EventPropagation::Continue
                }
            });

        // --- Hue slider ---
        let hue_svg = svg(move || build_hue(sv_w, slider_h, radius))
            .update_value(move || build_hue(sv_w, slider_h, radius))
            .style(move |s| s.absolute().width_pct(100.0).height_pct(100.0));
        let hue_thumb = Container::new(Empty::new()).style(move |s| {
            let (h, _, _) = rgb_to_hsv(color.get());
            let pure = hsv_to_rgb(h, 1.0, 1.0);
            s.absolute()
                .size(thumb, thumb)
                .inset_left((h / 360.0 * sv_w - thumb / 2.0) as f64)
                .inset_top((slider_h / 2.0 - thumb / 2.0) as f64)
                .border_radius(9999.0)
                .border(2.0)
                .border_color(WHITE.to_peniko())
                .background(pure.to_peniko())
        });
        let mut hue_area =
            Stack::new((hue_svg.into_any(), hue_thumb.into_any())).style(move |s| {
                s.width(sv_w)
                    .height(slider_h)
                    .margin_top(12.0)
                    .border_radius(radius)
                    .overflow_x(Overflow::Clip)
                    .overflow_y(Overflow::Clip)
                    .cursor(CursorStyle::Pointer)
            });
        let hue_id = hue_area.id();
        let hue_dragging = RwSignal::new(false);
        hue_area = hue_area
            .on_event(listener::PointerDown, {
                let commit = commit.clone();
                let hue_id = hue_id;
                let color = color;
                let dragging = hue_dragging;
                move |cx, pbe| {
                    if let Some(sz) = hue_id.get_size() {
                        let w = sz.width as f32;
                        if w > 0.0 {
                            let p = pbe.state.logical_point();
                            let ratio = (p.x as f32 / w).clamp(0.0, 1.0);
                            let (_, s, v) = rgb_to_hsv(color.get());
                            let mut c = hsv_to_rgb(ratio * 360.0, s, v);
                            c.a = color.get().a;
                            commit(c);
                        }
                    }
                    if let Some(pid) = pbe.pointer.pointer_id {
                        cx.request_pointer_capture(pid);
                    }
                    dragging.set(true);
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerMove, {
                let commit = commit.clone();
                let hue_id = hue_id;
                let color = color;
                let dragging = hue_dragging;
                move |_, pu| {
                    if dragging.get() {
                        if let Some(sz) = hue_id.get_size() {
                            let w = sz.width as f32;
                            if w > 0.0 {
                                let p = pu.current.logical_point();
                                let ratio = (p.x as f32 / w).clamp(0.0, 1.0);
                                let (_, s, v) = rgb_to_hsv(color.get());
                                let mut c = hsv_to_rgb(ratio * 360.0, s, v);
                                c.a = color.get().a;
                                commit(c);
                            }
                        }
                    }
                    EventPropagation::Continue
                }
            })
            .on_event(listener::PointerUp, {
                let dragging = hue_dragging;
                move |_, _| {
                    dragging.set(false);
                    EventPropagation::Continue
                }
            });

        let mut stack_children: Vec<AnyView> = Vec::new();
        stack_children.push(sv_area.into_any());
        stack_children.push(hue_area.into_any());

        // --- Alpha slider ---
        if show_alpha {
            let alpha_svg = svg(move || build_alpha(color.get(), sv_w, slider_h, radius))
                .update_value(move || build_alpha(color.get(), sv_w, slider_h, radius))
                .style(move |s| s.absolute().width_pct(100.0).height_pct(100.0));
            let alpha_thumb = Container::new(Empty::new()).style(move |s| {
                let cur = color.get();
                let ratio = cur.a as f32 / 255.0;
                s.absolute()
                    .size(thumb, thumb)
                    .inset_left((ratio * sv_w - thumb / 2.0) as f64)
                    .inset_top((slider_h / 2.0 - thumb / 2.0) as f64)
                    .border_radius(9999.0)
                    .border(2.0)
                    .border_color(WHITE.to_peniko())
                    .background(cur.to_peniko())
            });
            let mut alpha_area =
                Stack::new((alpha_svg.into_any(), alpha_thumb.into_any())).style(move |s| {
                    s.width(sv_w)
                        .height(slider_h)
                        .margin_top(12.0)
                        .border_radius(radius)
                        .overflow_x(Overflow::Clip)
                        .overflow_y(Overflow::Clip)
                        .cursor(CursorStyle::Pointer)
                });
            let alpha_id = alpha_area.id();
            let alpha_dragging = RwSignal::new(false);
            alpha_area = alpha_area
                .on_event(listener::PointerDown, {
                    let commit = commit.clone();
                    let alpha_id = alpha_id;
                    let color = color;
                    let dragging = alpha_dragging;
                    move |cx, pbe| {
                        if let Some(sz) = alpha_id.get_size() {
                            let w = sz.width as f32;
                            if w > 0.0 {
                                let p = pbe.state.logical_point();
                                let ratio = (p.x as f32 / w).clamp(0.0, 1.0);
                                let mut c = color.get();
                                c.a = (ratio * 255.0).round().clamp(0.0, 255.0) as u8;
                                commit(c);
                            }
                        }
                        if let Some(pid) = pbe.pointer.pointer_id {
                            cx.request_pointer_capture(pid);
                        }
                        dragging.set(true);
                        EventPropagation::Continue
                    }
                })
                .on_event(listener::PointerMove, {
                    let commit = commit.clone();
                    let alpha_id = alpha_id;
                    let color = color;
                    let dragging = alpha_dragging;
                    move |_, pu| {
                        if dragging.get() {
                            if let Some(sz) = alpha_id.get_size() {
                                let w = sz.width as f32;
                                if w > 0.0 {
                                    let p = pu.current.logical_point();
                                    let ratio = (p.x as f32 / w).clamp(0.0, 1.0);
                                    let mut c = color.get();
                                    c.a = (ratio * 255.0).round().clamp(0.0, 255.0) as u8;
                                    commit(c);
                                }
                            }
                        }
                        EventPropagation::Continue
                    }
                })
                .on_event(listener::PointerUp, {
                    let dragging = alpha_dragging;
                    move |_, _| {
                        dragging.set(false);
                        EventPropagation::Continue
                    }
                });
            stack_children.push(alpha_area.into_any());
        }

        // --- preview + hex input ---
        let preview = Container::new(Empty::new()).style(move |s| {
            let cur = color.get();
            s.size(40.0, 40.0)
                .border_radius(radius)
                .border(1.0)
                .border_color(theme.get().colors().outline.to_peniko())
                .background(cur.to_peniko())
        });
        let hex_input = TextInput::new(hex)
            .placeholder("#rrggbb")
            .style(move |s| {
                s.color(theme.get().colors().on_surface.to_peniko())
                    .font_size(14.0)
                    .width(120.0)
            })
            .on_event(listener::FocusLost, move |_, _| {
                let txt = hex.get();
                if let Some(c) = hex_to_color(&txt) {
                    commit(c);
                }
                EventPropagation::Continue
            });
        let bottom = h_stack((preview.into_any(), hex_input.into_any())).style(move |s| {
            s.margin_top(12.0).gap(12.0).items_center()
        });
        stack_children.push(bottom.into_any());

        Stack::new(stack_children).style(move |s| {
            s.width(sv_w)
                .padding(12.0)
                .border_radius(radius)
                .background(theme.get().colors().surface.to_peniko())
                .border(1.0)
                .border_color(with_alpha(theme.get().colors().outline, 80).to_peniko())
        })
    }
}

impl Default for CupertinoColorPicker {
    fn default() -> Self {
        Self::new()
    }
}
