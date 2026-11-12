// SPDX-License-Identifier: MIT
//! The embedded backend, end to end.
//!
//! An embedded target is the one platform with no operating system behind
//! it: a rectangle of memory, and input the board hands over. The point of
//! this file is that an app *runs* there — that the widget tree is laid out,
//! the software rasteriser draws it, and the pixels arrive in the
//! framebuffer the board reads.

#![cfg(feature = "embedded")]

use floom::floom_types::Color;
use floom::floom_widgets::Surface;
use floom::platform::embedded::run_frames;
use floom::prelude::*;
use floom_platform::{Platform, WindowConfig};
use floom_platform_embedded::EmbeddedPlatform;

const SIZE: f64 = 64.0;

/// A platform with one window and a framebuffer.
fn platform() -> EmbeddedPlatform {
    let mut platform = EmbeddedPlatform::new();
    platform.create_window(WindowConfig {
        title: "embedded".into(),
        size: Size::new(SIZE, SIZE),
        resizable: true,
    });
    platform
}

/// An app draws into the framebuffer.
///
/// The app is a real widget tree: a text node, mounted into a host, laid out
/// and rasterised by the software presenter, and copied into the surface the
/// board would scan out. The assertion is that ink arrived — not that a
/// counter went up.
#[test]
fn an_app_draws_into_the_embedded_framebuffer() {
    let mut platform = platform();
    platform.set_max_frames(3);

    let frames = run_frames(
        &mut platform,
        Box::new(|host: &mut WidgetHost| {
            host.set_size(Size::new(SIZE, SIZE));
            // A surface with a background, so the frame has ink in it whether or
            // not a font is loaded: the assertion is about the app reaching the
            // framebuffer, not about text rendering.
            let root = Surface::new()
                .background(Color::rgb(20, 120, 200))
                .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                    Box::new(Text::new("hello"))
                }));
            host.mount_root(LayoutStyle::default(), root);
        }),
    );

    assert_eq!(frames, 3, "the loop served the frames it was given");

    let surface = platform.framebuffer().expect("a framebuffer");
    let inked = surface
        .pixels()
        .chunks(4)
        .filter(|pixel| pixel[3] > 0)
        .count();
    assert!(
        inked > 0,
        "the app drew nothing into the framebuffer: {} pixels of {} are inked",
        inked,
        surface.pixels().len() / 4
    );
}

/// The framebuffer is the window's size, and it follows a resize.
#[test]
fn the_framebuffer_follows_the_window() {
    let mut platform = platform();
    platform.set_max_frames(1);
    run_frames(
        &mut platform,
        Box::new(|host: &mut WidgetHost| {
            host.set_size(Size::new(SIZE, SIZE));
        }),
    );

    let surface = platform.framebuffer().expect("a framebuffer");
    assert_eq!(surface.extent(), (SIZE as u32, SIZE as u32));

    platform.resize(Size::new(32.0, 16.0));
    let surface = platform.framebuffer().expect("a framebuffer");
    assert_eq!(surface.extent(), (32, 16));
    assert_eq!(
        surface.pixels().len(),
        32 * 16 * 4,
        "the buffer is the window's size, not the old one"
    );
}

/// The loop ends when the board asks it to.
#[test]
fn the_loop_ends_when_the_board_asks() {
    let mut platform = platform();
    platform.request_exit();
    let frames = run_frames(&mut platform, Box::new(|_host: &mut WidgetHost| {}));
    assert_eq!(
        frames, 0,
        "a board that says stop before the first frame gets none"
    );
}
