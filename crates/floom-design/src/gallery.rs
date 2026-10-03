//! A component gallery: every variant × size × state rendered into one scene.
//!
//! The gallery exists to make "this design system's variants are
//! distinguishable" a *measurable* claim. A screenshot of one button proves
//! nothing; a grid in which every cell is derived from the same table, snapped
//! as one scene, turns any accidental collapse (two variants resolving
//! identically, a state overlay that swallows the rest values) into a diff.
//!
//! The cells are painted from [`StyleProps`] only. Text labels are *not* drawn,
//! because the headless renderer has no font resource; the sample's identity is
//! recoverable from [`GallerySample::label`] instead, and the systems snapshot
//! those labels alongside the scene.

use floom_scene::{Scene, SceneNode};
use floom_style::StyleProps;
use floom_types::{Color, Point, Rect, Shadow, Size};

use crate::variant::{ControlSize, InteractionState, Variant};

/// Cell width in logical px.
pub const CELL_W: f64 = 160.0;
/// Cell height in logical px.
pub const CELL_H: f64 = 48.0;
/// Gap between cells in logical px.
pub const GAP: f64 = 12.0;

/// Alpha of the elevation shadow. The systems' tests look for exactly this
/// value to find a shadow rect, so it is a named constant rather than a literal
/// repeated at the construction and assertion sites.
pub const SHADOW_ALPHA: u8 = 40;

/// One swatch the gallery renders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GallerySample {
    pub variant: Variant,
    pub size: ControlSize,
    pub state: InteractionState,
}

impl GallerySample {
    pub fn new(variant: Variant, size: ControlSize, state: InteractionState) -> Self {
        Self {
            variant,
            size,
            state,
        }
    }

    /// Every variant at one size and state.
    pub fn variants_at(size: ControlSize, state: InteractionState) -> Vec<Self> {
        Variant::ALL
            .into_iter()
            .map(|variant| Self::new(variant, size, state))
            .collect()
    }

    /// The full cross product, in a stable order.
    pub fn full_grid() -> Vec<Self> {
        Variant::ALL
            .into_iter()
            .flat_map(|variant| {
                ControlSize::ALL.into_iter().flat_map(move |size| {
                    InteractionState::ALL
                        .into_iter()
                        .map(move |state| Self::new(variant, size, state))
                })
            })
            .collect()
    }

    /// A stable, human-readable identity — also the string a test diffs.
    pub fn label(&self) -> String {
        format!(
            "{}/{}/{}",
            self.variant.label(),
            self.size.label(),
            self.state.label()
        )
    }

    /// The cell's bounds for a grid `columns` wide.
    pub fn cell(&self, index: usize, columns: usize) -> Rect {
        let columns = columns.max(1);
        let col = (index % columns) as f64;
        let row = (index / columns) as f64;
        let x0 = col * (CELL_W + GAP);
        let y0 = row * (CELL_H + GAP);
        Rect {
            x0,
            y0,
            x1: x0 + CELL_W,
            y1: y0 + CELL_H,
        }
    }
}

/// The extent needed for `samples` in a grid `columns` wide.
pub fn gallery_size(samples: &[GallerySample], columns: usize) -> Size {
    if samples.is_empty() {
        return Size::new(CELL_W, CELL_H);
    }
    let columns = columns.max(1);
    let rows = samples.len().div_ceil(columns);
    Size::new(
        columns as f64 * (CELL_W + GAP) - GAP,
        rows as f64 * (CELL_H + GAP) - GAP,
    )
}

/// What one cell paints: the resolved properties, plus the box the control
/// actually occupies.
///
/// The height matters. A design system's size steps often change *only* the
/// box — Material's three button sizes share a fill, a radius and a shadow, and
/// differ purely in height — so a gallery that draws every control in a
/// fixed-size cell renders all three identically and can no longer tell them
/// apart. (That is not hypothetical: it is how this signature came to exist.)
/// Width is the slot's, because a headless gallery has no text to measure and
/// therefore no content width.
#[derive(Debug, Clone, PartialEq)]
pub struct CellPaint {
    pub props: StyleProps,
    pub height: f64,
}

impl CellPaint {
    pub fn new(props: StyleProps, height: f64) -> Self {
        Self { props, height }
    }
}

/// Paint every sample into one scene, resolving it with `style_of`.
///
/// A cell is up to three rects, inserted bottom-to-top: the elevation shadow,
/// the border, and the fill. The border is approximated as a full-slot rect
/// under an inset fill rather than four edge rects — good enough for a fixture,
/// and it keeps the node count a simple function of the control (1 + shadow +
/// border), which is what the tests assert on.
pub fn gallery_scene(
    samples: &[GallerySample],
    page: Color,
    columns: usize,
    style_of: impl Fn(GallerySample) -> CellPaint,
) -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();

    let size = gallery_size(samples, columns);
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(0.0, 0.0), size),
        fill: page.into(),
        radius: 0.0,
        shadow: None,
    });

    for (index, sample) in samples.iter().enumerate() {
        let slot = sample.cell(index, columns);
        let paint = style_of(*sample);
        let props = &paint.props;
        let radius = props.radius.unwrap_or(0.0);
        // The control is centred vertically in its slot, so a short control and
        // a tall one share a baseline and their difference is legible.
        let height = paint.height.clamp(0.0, CELL_H);
        let top = slot.y0 + (CELL_H - height) / 2.0;
        let cell = Rect {
            x0: slot.x0,
            y0: top,
            x1: slot.x1,
            y1: top + height,
        };

        // Elevation becomes a real drop shadow on the cell's own rect: the
        // offset pushes it down, the blur spreads it, and both scale with the
        // elevation level — no pseudo-shadow rect underneath any more.
        let shadow = props.elevation.filter(|e| *e > 0.0).map(|elevation| Shadow {
            color: Color::rgba(0, 0, 0, SHADOW_ALPHA),
            blur: f64::from(elevation),
            offset_x: 0.0,
            offset_y: f64::from(elevation),
        });

        let fill = props.background.unwrap_or(Color::TRANSPARENT);
        let border_width = props.border_width.unwrap_or(0.0);
        if border_width > 0.0 {
            let border = props.border_color.unwrap_or(Color::TRANSPARENT);
            let _ = scene.insert(SceneNode::Rect {
                bounds: cell,
                fill: border.into(),
                radius,
                shadow,
            });
            let inset = border_width as f64;
            let _ = scene.insert(SceneNode::Rect {
                bounds: Rect {
                    x0: cell.x0 + inset,
                    y0: cell.y0 + inset,
                    x1: cell.x1 - inset,
                    y1: cell.y1 - inset,
                },
                fill: fill.into(),
                radius: (radius - border_width).max(0.0),
                shadow: None,
            });
        } else {
            let _ = scene.insert(SceneNode::Rect {
                bounds: cell,
                fill: fill.into(),
                radius,
                shadow,
            });
        }
    }

    scene.finish_update();
    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::lerp;
    use crate::style::Props;
    use floom_types::Brush;

    /// A miniature design system: shape comes from the variant, shade from the
    /// state, radius and box height from the size.
    fn sample_style(sample: GallerySample) -> CellPaint {
        let base = Color::rgb(40, 90, 200);
        let (fill, border, elevation) = match sample.variant {
            Variant::Filled => (base, 0.0, 0.0),
            Variant::Tonal => (Color::rgb(210, 225, 250), 0.0, 0.0),
            Variant::Outlined => (Color::TRANSPARENT, 1.0, 0.0),
            Variant::Text => (Color::TRANSPARENT, 0.0, 0.0),
            Variant::Elevated => (Color::WHITE, 0.0, 2.0),
        };
        let shade = match sample.state {
            InteractionState::Rest => 1.0,
            InteractionState::Hovered => 0.95,
            InteractionState::Pressed => 0.9,
            InteractionState::Focused => 0.97,
            InteractionState::Disabled => 0.6,
            InteractionState::Selected => 0.85,
        };
        let fill = lerp(Color::WHITE, fill, shade);
        let radius = sample.size.pick(3.0, 4.0, 6.0);
        let props = Props::new()
            .bg_if_opaque(fill)
            .radius(radius)
            .border(border, Color::rgb(120, 120, 120))
            .elevation(elevation * radius / 4.0)
            .build();
        // A size step that moves only the box — the case that makes `height`
        // part of the paint rather than the slot's business.
        CellPaint::new(props, sample.size.pick(24.0, 32.0, 40.0))
    }

    fn rects(scene: &Scene) -> Vec<(Rect, Brush, f32)> {
        scene
            .iter()
            .filter_map(|(_, node)| match node {
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                    ..
                } => Some((*bounds, fill.clone(), *radius)),
                _ => None,
            })
            .collect()
    }

    fn shadow_offset_y(scene: &Scene) -> Option<f64> {
        scene.iter().find_map(|(_, node)| match node {
            SceneNode::Rect {
                shadow: Some(shadow),
                ..
            } if shadow.offset_y > 0.0 => Some(shadow.offset_y),
            _ => None,
        })
    }

    #[test]
    fn a_gallery_paints_a_page_node_plus_each_samples_rects() {
        let samples = GallerySample::variants_at(ControlSize::Md, InteractionState::Rest);
        let scene = gallery_scene(&samples, Color::WHITE, samples.len(), sample_style);
        // page
        // + Filled   (fill)
        // + Tonal    (fill)
        // + Outlined (border + fill)
        // + Text     (fill)
        // + Elevated (one rect: the fill carries its shadow on the node)
        assert_eq!(scene.len(), 1 + 1 + 1 + 2 + 1 + 1);
    }

    #[test]
    fn a_bordered_cell_adds_exactly_one_rect_over_a_plain_one() {
        let bordered = [GallerySample::new(
            Variant::Outlined,
            ControlSize::Md,
            InteractionState::Rest,
        )];
        let plain = [GallerySample::new(
            Variant::Text,
            ControlSize::Md,
            InteractionState::Rest,
        )];
        let n_bordered = gallery_scene(&bordered, Color::WHITE, 1, sample_style).len();
        let n_plain = gallery_scene(&plain, Color::WHITE, 1, sample_style).len();
        assert_eq!(
            n_bordered - n_plain,
            1,
            "the outline costs exactly one rect"
        );
    }

    #[test]
    fn a_bigger_elevation_offsets_the_shadow_further() {
        let render = |level: f32| {
            let mut style = sample_style(GallerySample::new(
                Variant::Elevated,
                ControlSize::Md,
                InteractionState::Rest,
            ));
            style.props.elevation = Some(level);
            gallery_scene(
                &[GallerySample::new(
                    Variant::Elevated,
                    ControlSize::Md,
                    InteractionState::Rest,
                )],
                Color::WHITE,
                1,
                |_| style.clone(),
            )
        };
        let low = shadow_offset_y(&render(1.0)).expect("an elevated cell has a shadow");
        let high = shadow_offset_y(&render(4.0)).expect("an elevated cell has a shadow");
        assert!(high > low, "elevation 4 offset {high} <= elevation 1 {low}");
    }

    #[test]
    fn a_samples_radius_and_fill_reach_the_painted_rect() {
        // The gallery must transfer the style, not re-derive it: if it rounded
        // every cell to the same value the grid would look uniform while every
        // style test still passed.
        //
        // Node 0 is the page. For a one-sample grid the page and the cell have
        // the *same* bounds (both are 160×48 at the origin), so geometry cannot
        // tell them apart — insertion order is the only honest discriminator.
        let sample = GallerySample::new(Variant::Filled, ControlSize::Lg, InteractionState::Rest);
        let style = sample_style(sample);
        let scene = gallery_scene(&[sample], Color::WHITE, 1, |_| style.clone());

        let all = rects(&scene);
        assert_eq!(all.len(), 2, "page + one cell");
        assert_eq!(all[0].1.solid(), Some(Color::WHITE), "node 0 is the page");
        let (_, cell_fill, cell_radius) = &all[1];
        assert_eq!(
            *cell_radius,
            style.props.radius.unwrap(),
            "radius did not reach the rect"
        );
        assert_eq!(
            cell_fill.solid().unwrap(),
            style.props.background.unwrap(),
            "fill did not reach the rect"
        );
    }

    #[test]
    fn each_sample_gets_its_own_disjoint_slot() {
        let samples = GallerySample::variants_at(ControlSize::Md, InteractionState::Rest);
        let columns = samples.len();
        let scene = gallery_scene(&samples, Color::WHITE, columns, sample_style);
        // One row of five, so the page is wider than a slot and can be told
        // apart by width alone.
        let all = rects(&scene);
        let pages = all.iter().filter(|(b, _, _)| b.width() > CELL_W).count();
        assert_eq!(pages, 1, "expected exactly one page rect");

        for (i, sample) in samples.iter().enumerate() {
            let slot = sample.cell(i, columns);
            let found = all.iter().any(|(bounds, _, _)| {
                // The slot's x is the control's x; its y is centred, so the
                // control stays inside the slot rather than starting at its top.
                bounds.x0 == slot.x0 && bounds.y0 >= slot.y0 && bounds.y1 <= slot.y1
            });
            assert!(found, "{} paints nothing inside {slot:?}", sample.label());
        }
        let xs: Vec<f64> = samples
            .iter()
            .enumerate()
            .map(|(i, s)| s.cell(i, columns).x0)
            .collect();
        let unique: std::collections::HashSet<_> = xs.iter().map(|x| x.to_bits()).collect();
        assert_eq!(unique.len(), xs.len(), "two samples share a column: {xs:?}");
    }

    #[test]
    fn the_control_height_comes_from_the_paint_not_the_slot() {
        // The regression this guards: a gallery that ignores the control's own
        // height draws every size step identically.
        let sample = GallerySample::new(Variant::Filled, ControlSize::Sm, InteractionState::Rest);
        let tall = GallerySample::new(Variant::Filled, ControlSize::Lg, InteractionState::Rest);
        let short = gallery_scene(&[sample], Color::WHITE, 1, sample_style);
        let high = gallery_scene(&[tall], Color::WHITE, 1, sample_style);

        let cell_height = |scene: &Scene| {
            let all = rects(scene);
            // Node 0 is the page; the shortest sampled cell here is the control.
            all.into_iter()
                .skip(1)
                .map(|(b, _, _)| b.height())
                .fold(f64::INFINITY, f64::min)
        };
        assert!(cell_height(&high) > cell_height(&short), "size was ignored");
    }

    #[test]
    fn gallery_size_grows_with_rows_then_columns() {
        let one = gallery_size(&GallerySample::full_grid()[..1], 1);
        let two = gallery_size(&GallerySample::full_grid()[..2], 1);
        assert!(two.height > one.height, "a second row extends the page");
        assert_eq!(two.width, one.width);

        let two_wide = gallery_size(&GallerySample::full_grid()[..2], 2);
        assert!(two_wide.width > two_wide.height, "two columns, one row");
    }

    #[test]
    fn labels_are_unique_across_the_full_grid() {
        let labels: Vec<String> = GallerySample::full_grid()
            .iter()
            .map(GallerySample::label)
            .collect();
        let unique: std::collections::HashSet<_> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "two samples share a label");
        assert_eq!(
            labels.len(),
            Variant::ALL.len() * ControlSize::ALL.len() * 6
        );
    }
}
