//! Reusable Material-style press ripple for any custom component.
#![cfg(any(
    feature = "material",
    feature = "cupertino",
    feature = "fluent"
))]

use std::rc::Rc;
use std::time::Duration;

use floem::animate::Animation;
use floem::context::{LayoutChanged, LayoutChangedListener};
use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::taffy::style::Overflow;
use floem::views::{Empty, Stack};

use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

use crate::theme_signal;

/// A reusable Material-style press ripple that can wrap **any** custom component.
///
/// Build it with [`Ripple::new`] around a child view, then call `.view()` to get
/// the ripple-enhanced view. The ink circle appears at the pointer-down point
/// and is clipped to the wrapper's rounded bounds, so it works equally well for
/// buttons, cards, list items, icon buttons, or any other view.
///
/// On pointer-down the ink expands from the click point (0 → `d`). On
/// pointer-up it fades out by animating its background alpha to 0 while holding
/// the expanded size, then the overlay is hidden — so the ink never pops
/// abruptly.
///
/// # Theming
/// The ink color defaults to `primary` at 24% alpha. Override it with
/// [`Ripple::color`]. The corner radius is theme-aware via [`Ripple::radius_fn`]
/// (e.g. `|t| t.radius().full`) or a fixed [`Ripple::radius`].
///
/// # Example
/// ```ignore
/// use floem_widget_runtime::Ripple;
/// let view = Ripple::new(my_card).radius(12.0).view();
/// ```
pub struct Ripple<V> {
    child: V,
    radius: f32,
    radius_fn: Option<Box<dyn Fn(&MultiTheme) -> f32>>,
    color_fn: Option<Rc<dyn Fn(&MultiTheme) -> Color>>,
    disabled: RwSignal<bool>,
}

impl<V: View + 'static> Ripple<V> {
    /// Wrap `child` with a default ripple (primary ink, sharp corners).
    pub fn new(child: V) -> Self {
        Self {
            child,
            radius: 0.0,
            radius_fn: None,
            color_fn: None,
            disabled: RwSignal::new(false),
        }
    }

    /// Round the wrapper's corners by a fixed `r` px so the ink is clipped to a
    /// rounded shape. Defaults to 0 (sharp corners). Overridden by
    /// [`Ripple::radius_fn`] when set.
    pub fn radius(mut self, r: f32) -> Self {
        self.radius = r;
        self
    }

    /// Round the wrapper's corners using a theme-derived radius, re-evaluated
    /// whenever the theme changes (e.g. `|t| t.radius().full`).
    pub fn radius_fn(mut self, f: impl Fn(&MultiTheme) -> f32 + 'static) -> Self {
        self.radius_fn = Some(Box::new(f));
        self
    }

    /// Override the ink color. By default the ripple uses `primary` at 24 alpha.
    pub fn color(mut self, f: impl Fn(&MultiTheme) -> Color + 'static) -> Self {
        self.color_fn = Some(Rc::new(f));
        self
    }

    /// Suppress the ripple entirely (e.g. when wrapping a disabled control).
    pub fn disabled(self, d: bool) -> Self {
        self.disabled.set(d);
        self
    }

    /// Build the ripple-wrapped view.
    pub fn view(self) -> impl View {
        let theme = theme_signal();
        let disabled = self.disabled;
        let ripple: RwSignal<Option<(f64, f64, f64)>> = RwSignal::new(None);
        let origin = RwSignal::new(Default::default()); // window origin (Point)
        let extent = RwSignal::new((0.0f64, 0.0f64)); // (width, height)

        // Stored as `Rc` so the same closure can be shared by the base style
        // and the expand/fade keyframe builders below.
        let color_fn: Rc<dyn Fn(&MultiTheme) -> Color> = self
            .color_fn
            .unwrap_or_else(|| Rc::new(|t: &MultiTheme| with_alpha(t.colors().primary, 24)));
        let radius_fn = self.radius_fn;
        let fixed_radius = self.radius;

        // Idle animation: invisible zero-size ink. The overlay is hidden while
        // `ripple` is `None`, so this only matters defensively.
        let idle: Animation = Animation::new()
            .duration(Duration::from_millis(400))
            .keyframe(0, |f| f.style(|s| s.size(0.0, 0.0)))
            .keyframe(100, |f| f.style(|s| s.size(0.0, 0.0)).ease_out());
        let anim_sig: RwSignal<Animation> = RwSignal::new(idle);

        // Anchor sits exactly at the click point; the ink circle is centered on
        // it (top-left at -d/2) so it expands symmetrically outward from the
        // pointer as its size animates 0->d.
        let ink_color = color_fn.clone();
        // Cloned so each pointer handler closure owns its own `Rc` (a `move`
        // closure would otherwise move the single `Rc` into the first one).
        let color_fn_down = color_fn.clone();
        let color_fn_up = color_fn.clone();
        let ink = Container::new(Empty::new())
            .style(move |s| {
                let (_, _, d) = ripple.get().unwrap_or((0.0, 0.0, 0.0));
                s.absolute()
                    .inset_left(-d / 2.0)
                    .inset_top(-d / 2.0)
                    .size(d, d)
                    .border_radius(d / 2.0)
                    .background(ink_color(&theme.get()).to_peniko())
            })
            .animation(move |_| anim_sig.get());

        // Visibility / click-point anchor, driven by the `ripple` signal so it
        // shows/hides reactively. `hide()` keeps it out of layout when idle.
        let overlay = Container::new(ink).style(move |s| match ripple.get() {
            Some((x, y, _d)) => s
                .absolute()
                .inset_left(x)
                .inset_top(y)
                .pointer_events_none(),
            None => s.hide(),
        });

        Container::new(Stack::new((self.child, overlay)))
            .style(move |s| {
                let r = radius_fn
                    .as_ref()
                    .map(|f| f(&theme.get()))
                    .unwrap_or(fixed_radius);
                s.overflow_x(Overflow::Clip)
                    .overflow_y(Overflow::Clip)
                    .apply_if(r > 0.0, |s| s.border_radius(r))
            })
            .on_event(LayoutChangedListener, move |_, layout: &LayoutChanged| {
                origin.set(layout.new_window_origin);
                let sz = layout.new_box.size();
                extent.set((sz.width as f64, sz.height as f64));
                EventPropagation::Continue
            })
            .on_event(listener::PointerDown, move |_, e| {
                if disabled.get() {
                    return EventPropagation::Continue;
                }
                let p = e.state.logical_point();
                let o = origin.get();
                let (w, h) = extent.get();
                let lx = p.x - o.x;
                let ly = p.y - o.y;
                let dx = (w - lx).max(lx);
                let dy = (h - ly).max(ly);
                let d = 2.0 * (dx * dx + dy * dy).sqrt();

                // Full-alpha ink at both ends of the expansion so a re-press
                // after a fade starts from a fully opaque circle.
                let full = color_fn_down(&theme.get()).to_peniko();
                // Restart the 0->d expansion for this press.
                anim_sig.set(
                    Animation::new()
                        .duration(Duration::from_millis(400))
                        .keyframe(0, |f| {
                            f.style(|s| s.size(0.0, 0.0).background(full))
                        })
                        .keyframe(100, |f| {
                            f.style(|s| s.size(d, d).background(full)).ease_out()
                        }),
                );
                ripple.set(Some((lx, ly, d)));
                EventPropagation::Continue
            })
            .on_event(listener::PointerUp, move |_, _| {
                if disabled.get() {
                    return EventPropagation::Continue;
                }
                if let Some((_x, _y, d)) = ripple.get() {
                    let c = color_fn_up(&theme.get());
                    let full = c.to_peniko();
                    let transparent = with_alpha(c, 0).to_peniko();
                    // Hold the expanded size and fade the ink's background alpha
                    // to 0, then hide the overlay on completion (no pop).
                    anim_sig.set(
                        Animation::new()
                            .duration(Duration::from_millis(300))
                            .keyframe(0, |f| {
                                f.style(|s| s.size(d, d).background(full))
                            })
                            .keyframe(100, |f| {
                                f.style(|s| s.size(d, d).background(transparent))
                                    .ease_out()
                            })
                            .on_complete(move |_| {
                                ripple.set(None);
                            }),
                    );
                }
                EventPropagation::Continue
            })
    }
}

/// Convenience wrapper equivalent to `Ripple::new(child).view()` for the common
/// case where the default ink (`primary` at 24 alpha) and sharp corners are fine.
///
/// ```ignore
/// use floem_widget_runtime::ripple;
/// let view = ripple(my_button);
/// ```
pub fn ripple<V: View + 'static>(child: V) -> impl View {
    Ripple::new(child).view()
}
