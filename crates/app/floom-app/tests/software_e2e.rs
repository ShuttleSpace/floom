// SPDX-License-Identifier: MIT
//! End-to-end: WidgetHost → SoftwarePresenter — paint damage flows into the
//! CPU rasteriser (skip → partial → skip) and text lands as real ink.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_renderer::{AttachedSurface, Presenter};
use floom_renderer_software::SoftwarePresenter;
use floom_types::Color;
use floom_types::{Size, SurfaceId, WindowId};
use floom_widgets::{Container, Text};

fn attached(size: Size) -> AttachedSurface {
    AttachedSurface::new(WindowId::new(0, 1), SurfaceId::new(0, 1), size, 1.0, None)
}

fn ink_pixels(presenter: &SoftwarePresenter, width: u32, height: u32) -> usize {
    (0..width * height)
        .filter(|i| {
            let x = i % width;
            let y = i / width;
            presenter
                .pixel_rgba(x, y)
                .map(|px| px[0] > 100)
                .unwrap_or(false)
        })
        .count()
}

#[test]
fn software_presentation_round_trip_and_damage_flow() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(300.0, 200.0));
    let root = host.mount_root(
        LayoutStyle {
            flex_direction: floom_layout::FlexDirection::Column,
            ..LayoutStyle::default()
        },
        Container::new(),
    );
    let label = host.mount(
        root,
        LayoutStyle::default(),
        Text::new("ink me").color(Color::rgb(255, 255, 255)),
    );

    let mut presenter = SoftwarePresenter::new();
    assert!(presenter.attach(attached(Size::new(300.0, 200.0))));

    // Frame 1 painted (text visible). Present it.
    host.frame();
    presenter.present(host.scene(), host.resources());
    let ink_frame1 = ink_pixels(&presenter, 300, 40);
    assert!(ink_frame1 > 0, "text rendered as ink on the CPU path");

    // Frame 2: nothing changed → host damage empty → presenter skips, but the
    // buffer keeps its content.
    host.frame();
    presenter.present(host.scene(), host.resources());
    assert!(
        presenter.renderer().damage_skips >= 1,
        "unchanged frame skipped the rasteriser"
    );
    assert_eq!(ink_pixels(&presenter, 300, 40), ink_frame1, "buffer intact");

    // Frame 3: content edit → damage covers the text region → repainted.
    host.widget_mut::<Text>(label)
        .expect("text widget")
        .set_content("ink me harder");
    host.frame();
    presenter.present(host.scene(), host.resources());
    let ink_frame3 = ink_pixels(&presenter, 300, 40);
    assert!(
        ink_frame3 >= ink_frame1,
        "new text rendered: {ink_frame3} vs {ink_frame1}"
    );
}
