use floom_types::Color;

/// How a [`StateLayer`] is drawn around the control it belongs to.
///
/// The two controls that use one differ only here: a radio's is a circle
/// centred on its ring, a checkbox's is a rounded box offset outward from its
/// indicator.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LayerShape {
    /// A circle of `size` across, centred on the control's indicator.
    #[default]
    Circle,
    /// A rounded box `size` across, centred on the control's indicator, with the
    /// control's own corner radius plus `padding`.
    ///
    /// The offset, rather than a bigger box drawn from the same corner, is what
    /// v1 did: Compose's absolute inset box with a `translate` back over the
    /// control. The visual result is a box `padding` points outside on every
    /// side.
    Box { padding: f32 },
}

/// The layer behind a control's indicator.
///
/// [`Default`] draws nothing, which is what the engine did before this type
/// existed and what a control with no state layer wants.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StateLayer {
    /// The layer while hovered or focused.
    pub color: Color,
    /// The layer while pressed. Only one of the three kits had one
    /// (`primary`@30); `None` means it keeps [`Self::color`].
    pub color_pressed: Option<Color>,
    /// The layer's own size, in the units [`LayerShape`] names.
    pub size: f32,
    /// How it is drawn.
    pub shape: LayerShape,
    /// Show it while hovered.
    pub on_hover: bool,
    /// Show it while pressed.
    pub on_press: bool,
    /// Show it while focused. Only Material's gates on this.
    pub on_focus: bool,
    /// Show nothing while disabled. Two of the three kits do this; Material's
    /// does not, so Material sets `false`.
    pub suppressed_when_disabled: bool,
}

impl StateLayer {
    /// Whether the layer should be painted at all, given the interaction state.
    ///
    /// Shared by both controls so the gate cannot drift between them -- the gate
    /// is four independent choices, which is the part most likely to be
    /// re-implemented slightly differently.
    pub fn shows(&self, hovered: bool, pressed: bool, focused: bool, disabled: bool) -> bool {
        let gated =
            (hovered && self.on_hover) || (pressed && self.on_press) || (focused && self.on_focus);
        gated && !(self.suppressed_when_disabled && disabled)
    }

    /// The layer's ink for the current state.
    pub fn ink(&self, pressed: bool) -> Color {
        if pressed {
            self.color_pressed.unwrap_or(self.color)
        } else {
            self.color
        }
    }
}
