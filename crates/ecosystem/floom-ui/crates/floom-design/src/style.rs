use floom_style::{StyleProps, StyleSet};
use floom_types::Color;

use crate::variant::InteractionState;

/// A [`StyleProps`] under construction, so a variant table reads as a list of
/// decisions rather than as a struct literal with nine `Option`s in it.
#[derive(Debug, Clone, Default)]
pub struct Props(StyleProps);

impl Props {
    pub fn new() -> Self {
        Self(StyleProps::default())
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.0.background = Some(color);
        self
    }

    /// Fill, unless `color` is fully transparent — an omitted fill and a
    /// transparent one are the same thing to a painter, and leaving it `None`
    /// keeps "this variant has no fill" greppable in a snapshot.
    pub fn bg_if_opaque(self, color: Color) -> Self {
        if color.a == 0 { self } else { self.bg(color) }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.0.foreground = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.0.font_size = Some(size);
        self
    }

    pub fn padding(mut self, padding: f32) -> Self {
        self.0.padding = Some(padding);
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.0.radius = Some(radius);
        self
    }

    /// A border of the given width and colour. A width of zero clears both, so a
    /// variant that removes its outline cannot leave a stray colour behind.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        if width <= 0.0 {
            self.0.border_width = None;
            self.0.border_color = None;
        } else {
            self.0.border_width = Some(width);
            self.0.border_color = Some(color);
        }
        self
    }

    pub fn elevation(mut self, elevation: f32) -> Self {
        self.0.elevation = Some(elevation);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.0.opacity = Some(opacity);
        self
    }

    pub fn build(self) -> StyleProps {
        self.0
    }
}

/// Assemble a [`StyleSet`] from per-state properties.
#[derive(Debug, Clone, Default)]
pub struct StyleSetBuilder {
    set: StyleSet,
}

impl StyleSetBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(mut self, state: InteractionState, props: StyleProps) -> Self {
        self.set.set(state.to_style_state(), props);
        self
    }

    pub fn build(self) -> StyleSet {
        self.set
    }
}

/// Resolve a set with an [`InteractionState`] without repeating the conversion.
pub trait StyleSetExt {
    fn resolve_state(&self, state: InteractionState) -> StyleProps;
}

impl StyleSetExt for StyleSet {
    fn resolve_state(&self, state: InteractionState) -> StyleProps {
        self.resolve(state.to_style_state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floom_style::StyleState;

    #[test]
    fn a_border_cleared_to_zero_leaves_no_colour_behind() {
        // A variant that drops its outline must not keep the colour as a
        // half-set pair, or a later `merge_over` can resurrect a border with
        // the wrong colour.
        let p = Props::new()
            .border(2.0, Color::rgb(1, 2, 3))
            .border(0.0, Color::rgb(9, 9, 9))
            .build();
        assert!(p.border_width.is_none());
        assert!(p.border_color.is_none());
    }

    #[test]
    fn a_transparent_fill_is_recorded_as_no_fill() {
        assert!(
            Props::new()
                .bg_if_opaque(Color::TRANSPARENT)
                .build()
                .background
                .is_none()
        );
        assert_eq!(
            Props::new()
                .bg_if_opaque(Color::rgb(1, 1, 1))
                .build()
                .background,
            Some(Color::rgb(1, 1, 1))
        );
    }

    #[test]
    fn build_is_a_chain_of_decisions() {
        let p = Props::new()
            .bg(Color::rgb(10, 10, 10))
            .fg(Color::WHITE)
            .font_size(14.0)
            .padding(8.0)
            .radius(6.0)
            .border(1.0, Color::rgb(20, 20, 20))
            .elevation(2.0)
            .opacity(0.5)
            .build();
        assert_eq!(p.background, Some(Color::rgb(10, 10, 10)));
        assert_eq!(p.foreground, Some(Color::WHITE));
        assert_eq!(p.font_size, Some(14.0));
        assert_eq!(p.padding, Some(8.0));
        assert_eq!(p.radius, Some(6.0));
        assert_eq!(p.border_width, Some(1.0));
        assert_eq!(p.border_color, Some(Color::rgb(20, 20, 20)));
        assert_eq!(p.elevation, Some(2.0));
        assert_eq!(p.opacity, Some(0.5));
    }

    #[test]
    fn resolve_state_reaches_the_right_style_flag() {
        let set = StyleSetBuilder::new()
            .state(
                InteractionState::Rest,
                Props::new().bg(Color::rgb(1, 1, 1)).radius(4.0).build(),
            )
            .state(
                InteractionState::Hovered,
                Props::new().bg(Color::rgb(2, 2, 2)).build(),
            )
            .build();

        assert_eq!(
            set.resolve_state(InteractionState::Rest).background,
            Some(Color::rgb(1, 1, 1))
        );
        let hover = set.resolve_state(InteractionState::Hovered);
        assert_eq!(hover.background, Some(Color::rgb(2, 2, 2)));
        assert_eq!(hover.radius, Some(4.0), "hover dropped the rest radius");
        assert_eq!(
            set.resolve_state(InteractionState::Disabled).radius,
            Some(4.0)
        );
        // Sanity: the builder really used the mapped flag.
        assert_eq!(
            set.resolve_state(InteractionState::Pressed),
            set.resolve(StyleState::PRESSED)
        );
    }
}
