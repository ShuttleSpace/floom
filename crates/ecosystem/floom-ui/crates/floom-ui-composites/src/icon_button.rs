use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Child, ChildKey, Color, CreateContext, LayoutRequest, UpdateContext, Widget, WidgetAny,
};

use floom_widgets::controls::Pressable;
use floom_widgets::ripple::Ripple;
use floom_widgets::surface::Surface;
use floom_widgets::vector_icon::VectorIcon;

/// Which of the three appearances an icon button takes.
///
/// The variant decides the box's fill, its outline, and the ripple's ink. It does
/// **not** decide the icon's colour, which follows `disabled` alone in all three
/// kits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IconButtonVariant {
    /// No fill, no outline. The icon alone.
    Text,
    /// Filled with the primary colour, outlined in the same colour.
    Contained,
    /// No fill, outlined in the outline colour.
    Outlined,
}

impl Default for IconButtonVariant {
    fn default() -> Self {
        IconButtonVariant::Text
    }
}

/// One variant's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconButtonPaint {
    /// The box's fill. Fully transparent for the text and outlined variants.
    pub background: Color,
    /// The box's outline colour. Fully transparent for the text variant.
    pub border: Color,
    /// The ink the press ripple expands in.
    pub ripple_ink: Color,
}

/// Everything about an icon button that is not its variant, plus the three
/// variants' resolved appearances.
///
/// A kit builds this once from its tokens and hands it over. The fields are named
/// for what they *are* rather than for the token they came from, because the three
/// kits read them from differently-named tokens and a name like
/// `outline_variant_background` would be true of only one of them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconButtonStyle {
    /// Edge length of the square box.
    pub size: f32,
    /// Edge length of the icon inside it.
    pub icon_size: f32,
    /// The box's corner radius, which the ripple is clipped to as well.
    pub radius: f32,
    /// Outline thickness. `0.0` draws none — which is what the text variant wants,
    /// and is why this is a field rather than a constant.
    pub border_width: f32,

    /// The icon's colour while enabled.
    pub foreground: Color,
    /// The icon's colour while disabled: an alpha of `on_surface` that all three
    /// kits agree on.
    pub disabled_foreground: Color,
    /// The box's fill while disabled, regardless of variant.
    pub disabled_background: Color,
    /// The box's outline while disabled, regardless of variant.
    pub disabled_border: Color,

    pub text: IconButtonPaint,
    pub contained: IconButtonPaint,
    pub outlined: IconButtonPaint,
}

impl Default for IconButtonStyle {
    /// Material's light-theme numbers, so an un-styled button is still a button.
    ///
    /// Material because it is the default design system, not because it is
    /// preferred: every kit crate supplies its own, and a default that were wrong
    /// for two of the three would only ever hide a kit that forgot to set one.
    fn default() -> Self {
        // `on_surface` at three alphas: 12 for the disabled fill, 38 for the
        // disabled outline and the disabled glyph, 24 for the ink. Written out
        // rather than derived from one `rgb` because `Color` has no alpha setter,
        // and a helper here would be the fourth spelling of "the same hue, less
        // opaque" in this crate.
        let clear = Color::rgba(0, 0, 0, 0);
        let on_surface_12 = Color::rgba(0x1D, 0x1B, 0x20, 12);
        let on_surface_24 = Color::rgba(0x1D, 0x1B, 0x20, 24);
        let on_surface_38 = Color::rgba(0x1D, 0x1B, 0x20, 38);
        let on_surface_variant = Color::rgb(0x49, 0x4A, 0x5C);
        let primary = Color::rgb(0x2B, 0x6C, 0xB0);
        IconButtonStyle {
            size: 40.0,
            icon_size: 24.0,
            radius: 20.0,
            border_width: 1.0,
            foreground: on_surface_variant,
            disabled_foreground: on_surface_38,
            disabled_background: on_surface_12,
            disabled_border: on_surface_38,
            text: IconButtonPaint {
                background: clear,
                border: clear,
                ripple_ink: on_surface_24,
            },
            contained: IconButtonPaint {
                background: primary,
                border: primary,
                ripple_ink: Color::rgba(0xFF, 0xFF, 0xFF, 24),
            },
            outlined: IconButtonPaint {
                background: clear,
                border: Color::rgb(0x79, 0x79, 0x7A),
                ripple_ink: on_surface_24,
            },
        }
    }
}

impl IconButtonStyle {
    /// The paint for `variant`.
    pub fn paint(&self, variant: IconButtonVariant) -> IconButtonPaint {
        match variant {
            IconButtonVariant::Text => self.text,
            IconButtonVariant::Contained => self.contained,
            IconButtonVariant::Outlined => self.outlined,
        }
    }

    /// The box's fill and outline, honouring `disabled`.
    ///
    /// The disabled values *replace* the variant's rather than blending with it,
    /// which is what v1 wrote. It is why a disabled outlined button does not keep
    /// the outline colour, and the difference is visible, so it is a rule rather than
    /// an accident.
    pub fn box_colors(&self, variant: IconButtonVariant, disabled: bool) -> (Color, Color) {
        if disabled {
            (self.disabled_background, self.disabled_border)
        } else {
            let paint = self.paint(variant);
            (paint.background, paint.border)
        }
    }

    /// The icon's colour, which follows `disabled` and ignores the variant.
    pub fn icon_color(&self, disabled: bool) -> Color {
        if disabled {
            self.disabled_foreground
        } else {
            self.foreground
        }
    }
}

/// A pressable square box with an icon centred in it.
///
/// Assembled from the parts rather than hand-painted, because that is what v1 did
/// and the parts *are* the design: the [`Surface`] is the button, the [`Pressable`]
/// is the click target, and the [`Ripple`] is the ink.
pub struct IconButton {
    icon: IconButtonIcon,
    variant: IconButtonVariant,
    disabled: bool,
    on_click: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
    style: IconButtonStyle,
}

impl std::fmt::Debug for IconButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconButton")
            .field("variant", &self.variant)
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
    }
}

impl IconButton {
    /// A button showing `icon`.
    ///
    /// The icon arrives already built; whatever size and colour it carries are
    /// overridden by [`Self::style`] on the first build, so a kit does not have to
    /// agree with the engine about either.
    pub fn new(icon: VectorIcon) -> Self {
        IconButton {
            icon: IconButtonIcon {
                icon,
                color: None,
                size: None,
            },
            variant: IconButtonVariant::default(),
            disabled: false,
            on_click: None,
            style: IconButtonStyle::default(),
        }
    }

    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: IconButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_click = Some(Arc::new(callback));
        self
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    pub fn variant_now(&self) -> IconButtonVariant {
        self.variant
    }

    /// The icon as built so far. For tests and for a caller reading it back;
    /// it does not reflect the style until the tree has been built once.
    pub fn icon(&self) -> &VectorIcon {
        &self.icon.icon
    }

    /// The style in force, including any the caller overrode.
    pub fn resolved_style(&self) -> IconButtonStyle {
        self.style
    }
}

/// The icon, owned by the button so that its colour can follow `disabled`.
///
/// This is a node with its own `update` rather than a value built inside a `Child`
/// builder, and that is the whole point. A 2.0 slot is replayed once per key: a
/// subtree built from a builder keeps the icon it first built, so a disabled toggle
/// — which changes the icon's colour without changing the tree's shape — would
/// repaint the box and leave the glyph stale. As a node, the icon is asked to
/// restyle itself, which is what [`VectorIcon::set_color`] exists for.
#[derive(Debug, Clone)]
pub struct IconButtonIcon {
    icon: VectorIcon,
    /// The colour and size to apply on the next `update`.
    color: Option<Color>,
    size: Option<f32>,
}

impl IconButtonIcon {
    /// The icon as it currently stands, after however many updates have run.
    pub fn icon(&self) -> &VectorIcon {
        &self.icon
    }

    /// The colour this node will apply on its next `update`.
    pub fn pending_color(&self) -> Option<Color> {
        self.color
    }
}

impl Widget for IconButtonIcon {
    fn create(&self, cx: &mut dyn CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: floom_core::ElementId) {
        // `set_*` are idempotent, so this is cheap on the frames where nothing moved.
        if let Some(color) = self.color {
            self.icon.set_color(color);
        }
        if let Some(size) = self.size {
            self.icon.set_size(size);
        }
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        self.icon.layout_request()
    }

    fn paint(&self, cx: &mut dyn floom_core::PaintContext, id: floom_core::ElementId) {
        self.icon.paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn floom_core::SemanticsContext, id: floom_core::ElementId) {
        self.icon.semantics(cx, id);
    }
}

impl WidgetAny for IconButtonIcon {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The icon's own semantics, forwarded: an unlabelled icon is decorative, and
    /// that is a decision the icon already made from its props.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        Some(self.icon.semantics_hint().into())
    }
}

impl Widget for IconButton {
    fn create(&self, cx: &mut dyn CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty, for the reason [`floom_widgets::compose::SelectGroup::update`]
    /// gives: everything this widget can change is a change *to a child*, and every
    /// node is visited each frame, so the child reports its own change. Marking
    /// dirty here too would be a second report of a change already reported.
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: floom_core::ElementId) {}

    /// The subtree, as one child: `Ripple(Pressable(Surface(icon)))`.
    ///
    /// Declared in one `Child` rather than three so the whole composition is one
    /// value a test can walk, and so the ripple stays outermost — it is what clips
    /// the ink to the box's rounded bounds, and putting anything outside it would
    /// mean the ink is painted over a region the button does not own.
    fn children(&self) -> Vec<Child> {
        let style = self.style;
        let variant = self.variant;
        let disabled = self.disabled;
        let icon = self.icon.clone();
        let on_click = self.on_click.clone();

        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new().fill(),
            move || {
                let paint = style.paint(variant);
                let (background, border) = style.box_colors(variant, disabled);

                let surface = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width(style.size)
                            .height(style.size)
                            // v1's `.items_center().justify_center()`, on the
                            // request that lays the icon out inside the box.
                            .main_align(Align::Center)
                            .cross_align(Align::Center),
                    )
                    .background(background)
                    .radius(style.radius)
                    .border(style.border_width, border)
                    .push({
                        let icon_node = IconButtonIcon {
                            icon: icon.icon.clone(),
                            color: Some(style.icon_color(disabled)),
                            size: Some(style.icon_size),
                        };
                        // Cloned per replay: a builder is an `Fn`, so it may run
                        // again, and a moved widget would be gone the second time.
                        let icon_node = icon_node.clone();
                        move || Box::new(icon_node.clone())
                    });

                // `Pressable` is built *here*, inside the ripple's builder, and not
                // captured into it. Its `on_click` callback is deliberately not
                // `Send + Sync` (a caller may want to capture an `Rc`), so a
                // `Pressable` cannot cross a thread boundary — and a slot builder is
                // required to be `Send + Sync`. Constructing it at call time is the
                // way that holds: the value exists only inside the closure's own
                // stack frame, and what leaves is a `Box<dyn WidgetAny>`, which has
                // no `Send` bound at all.
                Box::new(
                    Ripple::new()
                        .child(LayoutRequest::new().fill(), {
                            // Both clones happen *here*, in the outer `Fn`'s body,
                            // so the closure below owns them outright. Cloning
                            // inside it instead would leave every inner closure
                            // borrowing the outer capture, which an `Fn` cannot
                            // hand out and `'static` cannot outlive.
                            let on_click = on_click.clone();
                            let surface = surface.clone();
                            move || {
                                let mut pressable = Pressable::new(Child::new(
                                    ChildKey::index(0),
                                    LayoutRequest::new().fill(),
                                    {
                                        let surface = surface.clone();
                                        move || Box::new(surface.clone())
                                    },
                                ))
                                .role(SemanticsRole::Button)
                                .disabled(disabled);

                                if let Some(on_click) = on_click.clone() {
                                    pressable = pressable.on_click(move || on_click());
                                }

                                Box::new(pressable)
                            }
                        })
                        .radius(f64::from(style.radius))
                        .ink(paint.ripple_ink)
                        .disabled(disabled),
                )
            },
        )]
    }
}
