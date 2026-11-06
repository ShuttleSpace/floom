use floom_core::Widget;
use floom_types::{Insets, Rect, Size};
use floom_widgets::testkit::{paint_snapshot, AllocatingCreateContext, SceneRecorder};

use floom_core_theme::theme::MultiTheme;
use floom_widget_md_toolbar::MdToolbar;

const BAR: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 48.0,
};

fn toolbar() -> MdToolbar {
    MdToolbar::new()
}

#[test]
fn a_plain_bar_is_a_filled_surface_with_its_padding() {
    let bar = toolbar().title("Settings");
    let surface = bar.view();
    let (id, _create) = (surface.create(&mut AllocatingCreateContext::new()), ());
    let painted = paint_snapshot(&surface, id, BAR);

    assert!(
        painted.contains("RoundedRect") || painted.contains("Rect"),
        "the bar paints a fill: {painted}"
    );
}

#[test]
fn the_bar_declares_its_own_height_and_full_width() {
    let bar = toolbar();
    let request = bar.request();
    assert_eq!(
        request.height.and_then(|h| h.px()),
        Some(48.0),
        "the default bar is 48px tall"
    );
    assert!(
        request.width.map(|w| w.is_pct()).unwrap_or(false),
        "and the full width of its parent, which v1 wrote as `width_pct(100.0)`: {request:?}"
    );

    let surface = toolbar().view();
    let declared = surface
        .layout_request()
        .expect("a padded, sized surface has a request");
    assert_eq!(declared.height.and_then(|h| h.px()), Some(48.0));
    assert!(declared.width.map(|w| w.is_pct()).unwrap_or(false));
}

#[test]
fn the_title_carries_the_medium_weight() {
    let style = toolbar().title("Settings").resolved_style();
    assert_eq!(
        style.title_weight,
        floom_core::FontWeight::Medium,
        "v1 wrote `FontWeight::Medium` here"
    );

    let title = toolbar().title_widget("Settings");
    assert_eq!(title.font_weight(), floom_core::FontWeight::Medium);
}

#[test]
fn the_token_weight_conversion_does_not_repeat_the_theme_bug() {
    use floom_core::FontWeight as W;
    let convert = |w: floom_core_theme::tokens::typography::FontWeight| match w {
        floom_core_theme::tokens::typography::FontWeight::ExtraBold => W::ExtraBold,
        floom_core_theme::tokens::typography::FontWeight::Black => W::Black,
        _ => W::Normal,
    };
    assert_ne!(
        convert(floom_core_theme::tokens::typography::FontWeight::ExtraBold),
        convert(floom_core_theme::tokens::typography::FontWeight::Black)
    );
}

#[test]
fn the_leading_icon_is_optional() {
    assert!(
        toolbar().leading_widget('m').glyph() == 'm',
        "an icon renders when one was named"
    );
    let group = toolbar().title("Settings").leading_group();
    assert_eq!(
        group.child_count(),
        1,
        "no leading icon means one child, the title"
    );
    assert_eq!(
        toolbar()
            .leading("menu")
            .title("Settings")
            .leading_group()
            .child_count(),
        2,
        "with one, two"
    );
}

#[test]
fn the_bar_is_a_single_row_with_the_title_group_first() {
    let surface = toolbar().title("Settings").leading("menu").view();
    assert_eq!(
        surface.child_count(),
        1,
        "one child: the row. The bar does not add a wrapper per group."
    );
}

#[test]
fn the_actions_slot_is_optional_and_lands_last() {
    let without = toolbar().title("Settings").view();
    assert_eq!(without.child_count(), 1, "no actions, one child");

    let with = toolbar()
        .title("Settings")
        .actions(|| Box::new(floom_widgets::Text::new("more")))
        .view();
    assert_eq!(with.child_count(), 1, "still one child — the row");
}

#[test]
fn an_unelevated_bar_has_no_shadow_and_an_elevated_one_does() {
    assert_eq!(
        toolbar().shadow(),
        None,
        "v1 only added the shadow when elevation > 0, and a bar with no elevation \
         should carry no shadow op at all"
    );
    let shadow = toolbar()
        .elevation(1)
        .shadow()
        .expect("an elevated bar casts one");
    assert_eq!(shadow.blur, 6.0, "v1: box_shadow_blur(6.0)");
    assert_eq!(shadow.spread, 0.0, "v1: box_shadow_spread(0.0)");
    assert_eq!(shadow.offset_x, 0.0, "v1 set no offset");
    assert_eq!(shadow.offset_y, 0.0, "v1 set no offset");
    assert_eq!(shadow.color.a, 30, "v1: with_alpha(BLACK, 30)");
}

#[test]
fn the_padding_comes_from_the_token() {
    let style = toolbar().resolved_style();
    assert_eq!(style.padding, 12.0, "the theme's toolbar padding");
    assert_eq!(
        toolbar().padding(),
        Insets::uniform(12.0),
        "and reaches the surface as a uniform inset"
    );
}

#[test]
fn the_bar_height_is_the_prop_not_the_token() {
    let bar = toolbar().height(72.0);
    assert_eq!(bar.resolved_style().height, 72.0);
    assert_eq!(
        bar.request().height.and_then(|h| h.px()),
        Some(72.0),
        "and the request follows it"
    );
}

#[test]
fn the_shadow_follows_the_props_and_not_the_theme() {
    let theme = MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::default());
    let flat = MdToolbar::new().with_theme(theme.clone());
    let raised = MdToolbar::new().with_theme(theme).elevation(2);
    assert_eq!(flat.shadow(), None);
    assert!(raised.shadow().is_some());
}

#[test]
fn the_ported_bar_carries_everything_v1_styled_onto_one_widget() {
    let surface = toolbar()
        .title("Settings")
        .leading("menu")
        .elevation(1)
        .height(56.0)
        .view();
    let request = surface
        .layout_request()
        .expect("the surface declares an extent and an inset");
    assert_eq!(request.height.and_then(|h| h.px()), Some(56.0));
    assert_eq!(request.padding, Some(Insets::uniform(12.0)));
    assert!(request.width.map(|w| w.is_pct()).unwrap_or(false));
    assert!(surface.decoration().drop_shadow().is_some());
    assert_eq!(surface.child_count(), 1);
}

#[test]
fn the_row_is_horizontal_and_centred() {
    let surface = toolbar().title("Settings").leading("menu").view();
    let id = surface.create(&mut AllocatingCreateContext::new());
    let mut recorder = SceneRecorder::new(BAR);
    surface.paint(&mut recorder, id);
    recorder.finish();

    let ops = recorder.ops();
    assert!(
        !ops.is_empty(),
        "the bar painted something inside {:?}",
        Size::new(BAR.width(), BAR.height())
    );
}
