<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
<div align="center">
<img width=120 height=120 src="https://lap.dev/images/floom.svg"></img>


# Floom

A native Rust UI library with fine-grained reactivity and great ergonomics

[![crates.io](https://img.shields.io/crates/v/floom.svg)](https://crates.io/crates/app/floom)
[![docs.rs](https://docs.rs/floom/badge.svg)](https://docs.rs/floom)
[![Discord](https://img.shields.io/discord/946858761413328946?color=%237289DA&label=discord)](https://discord.gg/RB6cRYerXX)

</div>

_The project is still maturing. We will make occasional breaking changes and add missing features on our way to v1._

## Quickstart

```rust
use floom::prelude::*;

fn main() {
    floom::launch(counter_view);
}

fn counter_view() -> impl IntoView {
    let mut counter = RwSignal::new(0);

    Stack::horizontal((
        Button::new("Increment").action(move || counter += 1),
        Label::derived(move || format!("Value: {counter}")),
        Button::new("Decrement").action(move || counter -= 1),
    ))
    .style(|s| s.size_full().items_center().justify_center().gap(10))
}

```


<img src="../floem-archive/docs/img/quickstart.jpg" width="300"/>


## Features

Inspired by [Xilem](https://github.com/linebender/xilem), [Leptos](https://github.com/leptos-rs/leptos) and [rui](https://github.com/audulus/rui), Floom aims to be a high performance declarative UI library with a highly ergonomic API.

- **Cross-platform**: Floom supports Windows, macOS and Linux with rendering using [wgpu](https://github.com/gfx-rs/wgpu) via [vger](https://github.com/audulus/vger-rs) or [vello](https://github.com/linebender/xilem), or via AnyRender's GPU-backed [Skia](https://github.com/dioxuslabs/anyrender) renderer. In case a GPU is unavailable, a CPU renderer powered by [tiny-skia](https://github.com/RazrFalcon/tiny-skia) will be used.
- **Fine-grained reactivity**: The entire library is built around reactive primitives inspired by [leptos_reactive](https://crates.io/crates/leptos_reactive). The reactive "signals" allow you to keep your UI up-to-date with minimal effort, all while maintaining very high performance.
- **Performance**: The view tree is constructed only once, safeguarding you from accidentally creating a bottleneck in a view generation function that slows down your entire application. Floom also provides tools to help you write efficient UI code, such as a [virtual list](crates/widgets/floom-virtual/src/lib.rs).
- **Flexbox & Grid layout**: Using [Taffy](https://crates.io/crates/taffy), the library provides the Flexbox and Grid layout systems, which can be applied to any View node.
- **Customizable widgets**: Widgets are highly customizable. You can customize both the appearance and behavior of widgets using the styling API, which supports theming with classes. You can also install third-party themes.
- **Transitions and Animations**: Floom supports both transitions and animations. Transitions, like css transitions, can animate any property that can be interpolated and can be applied alongside other styles, including in classes.
    Floom also supports full keyframe animations that build on the ergonomics of the style system. In both transitions and animations, Floom supports easing with spring functions.
- **Element inspector**: Inspired by your browser's developer tools, Floom provides a [diagnostic tool](../floem-archive/docs/img/inspector.jpg) to debug your layout.
- **Global themes**: Support for light and dark themes, built in design system to easily declare your own
- **Localization**: Floom supports localizations via [Fluent](https://github.com/projectfluent/fluent-rs) crate with support for fallbacks, easy switching languages on runtime or even overriding active language for chosen part of you app

## Ecosystem

Design-system widgets live in the separate **floom-ui** project. The rich text/code editor lives in **floom-editor**. Neither is part of the Floom 1.0 core guarantee — see [docs/ecosystem.md](docs/ecosystem.md).

Backend authors can experiment with the public but unstable [`floom::embedder`](docs/embedder.md) API for integrating Floom with platform backends other than the default winit desktop path. Ordinary applications should continue to use `floom::launch(...)`.

To sample Floom's capabilities, check out the repo and run the [showcase](examples/floom-showcase/src/main.rs) example with `cargo run -p floom-showcase`. Its `/components` page mounts every Tier-0 widget; the images below are its widget gallery and inspector views.


<img src="../floem-archive/docs/img/widget-gallery-dark.jpg" width="900"/>

<details>
    <summary>Light Mode</summary>

<img src="../floem-archive/docs/img/widget-gallery-light.jpg" width="900"/>
</details>




<img src="../floem-archive/docs/img/inspector.jpg" width="900"/>

To help you master Floom, we provide [documentation](https://docs.rs/floom) and [code examples](examples/).

## Contributions

<a href="https://ws.lap.dev/#https://github.com/lapce/floom" target="_blank">
      <img src="https://lap.dev/images/open-in-lapdev.svg?version=8" alt="Open in Lapdev">
</a>

[Lapdev](https://lap.dev/), developed by the Lapce team, is a cloud dev env service similar to GitHub Codespaces. By clicking the button above, you'll be taken to a fully set up Floom dev env where you can browse the code and start developing. All dependencies are pre-installed, so you can get straight to code.

[Contributions welcome!](CONTRIBUTING.md) If you'd like to improve how Floom works and fix things, feel free to open an issue or submit a PR. If you'd like a conversation with Floom devs, you can join in the #floom channel on this [Discord](https://discord.gg/RB6cRYerXX).
