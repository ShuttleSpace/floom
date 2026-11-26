//! What a phone-sized scale does to where the ink lands.
//!
//! The OpenHarmony emulator reports a 2210x2416 surface at density 3.125 and the
//! widget tree drew into a 49x24 corner of it. This reproduces the same numbers on
//! a desk, so the question "is the scale applied once too often" is answered by a
//! test rather than by another ten-minute install.

use floom_app::{AppRunner, WidgetHost};
use floom_core::{Axis, Child, ChildKey, LayoutRequest};
use floom_layout::{FlexDirection, LayoutStyle};
use floom_platform::{HostPlatform, Platform, WindowConfig};
use floom_renderer_software::SoftwarePresenter;
use floom_types::{Rect, Size};
use floom_widgets::{Button, Stack};

const LOGICAL: Size = Size {
    width: 707.2,
    height: 706.56,
};
const SCALE: f64 = 3.125;

fn ink_box(host: WidgetHost, platform: &mut HostPlatform) -> Rect {
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());
    let window = platform.create_window(WindowConfig {
        title: "scale".to_string(),
        size: LOGICAL,
        resizable: false,
    });

    for _ in 0..6 {
        runner.pump(platform);
    }
    let presenter = runner.presenter();
    eprintln!("presenter {}x{}", presenter.width(), presenter.height());
    let width = presenter.width() as usize;
    let height = presenter.height() as usize;
    let pixels = presenter.pixels();
    let mut bbox = Rect {
        x0: f64::MAX,
        y0: f64::MAX,
        x1: 0.0,
        y1: 0.0,
    };
    for y in 0..height {
        for x in 0..width {
            if pixels[(y * width + x) * 4 + 3] > 0 {
                bbox.x0 = bbox.x0.min(x as f64);
                bbox.y0 = bbox.y0.min(y as f64);
                bbox.x1 = bbox.x1.max(x as f64);
                bbox.y1 = bbox.y1.max(y as f64);
            }
        }
    }
    bbox
}

fn tree(host: &mut WidgetHost) {
    let column = host.mount_root(
        LayoutStyle {
            flex_direction: FlexDirection::Column,
            ..LayoutStyle::default()
        },
        floom_widgets::Container::new(),
    );
    host.mount(
        column,
        LayoutStyle::default(),
        Stack::new(Axis::Vertical).child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            || Box::new(Button::new("Floom on OpenHarmony")),
        )),
    );
}

#[test]
fn a_scaled_surface_paints_the_button_at_logical_size_times_scale() {
    let mut host = WidgetHost::new();
    tree(&mut host);
    let mut platform = HostPlatform::new().with_scale_factor(SCALE);
    let bbox = ink_box(host, &mut platform);
    // The button is laid out in logical units; a 3.125 display is 3.125x wider.
    assert!(
        bbox.width() > 300.0,
        "the button should span hundreds of device pixels, got {bbox:?}"
    );
}
