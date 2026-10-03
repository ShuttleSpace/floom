//! # Floom
//! Floom is a cross-platform GUI library for Rust. It aims to be extremely performant while providing world-class developer ergonomics.
//!
//! The following is a simple example to demonstrate Floom's API and capabilities.
//!
//! ## Example: Counter
//! ```rust
//! use floom::prelude::*;
//!
//! let mut counter = RwSignal::new(0);
//!
//! v_stack((
//!     label(move || format!("Value: {counter}")),
//!     h_stack((
//!         button("Increment").action(move || counter += 1),
//!         button("Decrement").action(move || counter -= 1),
//!     )),
//! ));
//! ```
//! This example demonstrates the core concepts of building a reactive GUI with Floom:
//!
//! - State Management: The `RwSignal` provides a reactive way to manage the counter's state.
//! - Widgets: Common UI elements like `stack`, `label`, and `button` are easily implemented.
//! - Reactivity: The label automatically updates when the counter changes.
//! - Event Handling: Button clicks are handled with simple closures that modify the state.
//!
//! Floom's objectives prioritize simplicity and performance, enabling the development of complex graphical user interfaces with minimal boilerplate.
//!
//! ## Views
//! Floom models the UI using a tree of [Views](view::View). Views, such as the `h_stack`, `label`, and
//! `button` elements are the building blocks of UI in Floom.
//!
//! Floom's main view tree is constructed only once.
//! This guards against unnecessary and expensive rebuilds of your views;
//! however, even though the tree is built only once, views can still receive reactive updates.
//!
//! ### Composition and Flexibility
//! Views in Floom are composable, allowing for the construction of complex user interfaces by integrating simpler components.
//! In the counter example, label and button views were combined within vertical (`v_stack`) and horizontal (`h_stack`) layouts to create a more intricate interface.
//!
//! This compositional approach provides the following benefits:
//! - Reusable UI components
//! - Easy and consistent refactoring
//!
//! ### Learn More
//! Floom provides a set of built-in views to help you create UIs quickly.
//! To learn more about the built-in views, check out the [views module](crate::views) documentation.
//!
//! ## State management
//!
//! Floom uses a reactive system built on signals and effects for its state management.
//!
//! Floom uses its own reactive system with an API that is similar to the one in the [leptos_reactive](https://docs.rs/leptos_reactive/latest/leptos_reactive/index.html) crate.
//!
//! ### Signals as State
//!
//! You can create reactive state by creating a signal anywhere in the program using [`RwSignal::new()`](floom_legacy_reactive::RwSignal::new), [`RwSignal::new_split()`](floom_legacy_reactive::RwSignal::new_split), or use a [different signal type](floom_legacy_reactive).
//!
//! When you use a signal by calling the [`get`](floom_legacy_reactive::SignalGet::get) or [`with`](floom_legacy_reactive::SignalWith::with) methods, (which are also called when you use an operator such as `==`)
//! the runtime will automatically subscribe the correct side effects
//! to changes in that signal, creating reactivity. To the programmer this is transparent.
//! By simply accessing the value where you want to use it, the reactivity will "just work" and
//! your views will stay in sync with changes to that signal.
//!
//!
//! #### Example: Changing Text
//! ```rust
//! # use floom::reactive::*;
//! # use floom::views::*;
//! # use floom::IntoView;
//! fn app_view() -> impl IntoView {
//!
//!     // All signal types implement `Copy`, so they can be easily used without needing to manually clone them.
//!     let text = RwSignal::new("Hello, World!".to_string());
//!
//!     let label_view = label(move || text.get());
//!
//!     let button = button("Change Text").action(move || text.set("Hello, Floom!".to_string()));
//!
//!     v_stack((button, label_view))
//! }
//! ```
//!
//! In this example, `text` is a signal containing a `String` that can be both read from and written to.
//! The button, when clicked, changes the text in the signal.
//! The label view is subscribed to changes in the `text` signal and will automatically trigger a re-render with the updated text value whenever the signal changes.
//!
//! ### Functions as a Primitive of Reactivity
//!
//! The most fundamental primitive of reactivity is a function that can be re-run in response to changes in a signal.
//! For this reason, many of Floom's APIs accept functions as arguments (such as in the label view in the `Changing Text` example above).
//! Most of the functions is Floom's API will
//! update reactively, but not all of them do. For this reason, all arguments in Floom's API that are functions will
//! be marked with a `# Reactivity` section that will inform you if the function will be re-run in response to reactive updates.
//!
//! ### Learn More
//! To learn more about signals and
//! effects, you may want to explore the Leptos [documentation](https://docs.rs/leptos_reactive/latest/leptos_reactive/index.html)
//! and the [leptos book](https://leptos-rs.github.io/leptos/).
//!
//! ## Style: Customizing Appearance
//!
//! Floom has a powerful, built-in styling system that allows you to customize the appearance of your UI.
//!
//! Example:
//! ```
//! #  use floom::peniko::color::palette;
//! #  use floom::reactive::*;
//! #  use floom::style::Style;
//! #  use floom::unit::UnitExt;
//! #  use floom::View;
//! #  use floom::views::{text, Decorators};
//! #
//! text("Some text").style(|s| s.font_size(21.).color(palette::css::DARK_GRAY));
//! ```
//!
//! The text view is styled by calling the [`style`](crate::views::Decorators::style) method (you'll need to import the
//! [`Decorators`](crate::views::Decorators) trait to use the it). The `style` method takes a closure that takes and returns a
//! [`Style`](crate::style::Style) value using the builder pattern. Through this value, you can access methods that modify a variety
//! of familiar properties such as width, padding, and background. Some `Style` properties
//! such as font size are `inherited` and will apply to all of a view's children until overridden.
// TODO: Add links on these
//!
//! In this same style value, floom supports:
//! - themeing with [classes](style::Style::class)
//! - [property transitions](style::Style::transition)
//! - defining styles on different [interaction states](style::Style::hover)
//! - reactive updates
//! - applying styles [conditionally](style::Style::apply_if)
//! - setting custom properties
//! - and more
//!
//! For additional information about styling, [see here](crate::style).
//!
//! ## Animation
//!
//! In addition to [property transitions](style::Style::transition) that can be added to `Style`s,
//! Floom has a full keyframe animation system that allows you to animate any property that can be [interpolated](style::StylePropValue::interpolate) and builds on the capabilities and ergonomics of the style system.
//!
//! Animations in Floom, by default, have keyframes ranging from 0-100.
//!
//! #### Example: Rectangle to Square
//!
//! ```
//! #  use floom::peniko::color::palette;
//! #  use floom::reactive::*;
//! #  use floom::style::Style;
//! #  use floom::unit::{UnitExt, DurationUnitExt};
//! #  use floom::View;
//! #  use floom::views::*;
//! #
//! empty()
//!     .style(|s| s.background(palette::css::RED).size(500, 100))
//!     .animation(move |a| {
//!         a.duration(5.seconds())
//!             .keyframe(0, |f| f.computed_style())
//!             .keyframe(50, |f| {
//!                 f.style(|s| s.background(palette::css::BLACK).size(30, 30))
//!                     .ease_in()
//!             })
//!             .keyframe(100, |f| {
//!                 f.style(|s| s.background(palette::css::AQUAMARINE).size(10, 300))
//!                     .ease_out()
//!             })
//!             .auto_reverse(true)
//!             .repeat(true)
//!     });
//! ```
//! - The first keyframe will use the computed style, which will include the red background with size of 500x100.
//! - At 50%, the animation will animate to a black square of 30x30 with a bezier easing of `ease_in`.
//! - At 100% the animation will animate to an aquamarine rectangle of 10x300 with an bezier easing of `ease_out`.
//! - The animation is also set to automatically reverse (the animation will run and reverse in 5 seconds) and repeat forever.
//!
//! You can add animations to a `View` instance by calling the [`animation`](crate::views::Decorators::animation) method from the `Decorators` trait.
//! The `animation` method takes a closure that takes and returns an [`Animation`](crate::animate::Animation) value using the builder pattern.
//!
//! For additional information about animation, [see here](crate::animate::Animation).

pub mod accessibility;
pub mod action;
pub mod animate;
mod app;
mod box_tree;
pub mod context;
pub mod embedder;
pub mod event;
pub mod ext_event;
mod inspector;
pub mod layout;
mod message;
pub mod paint;
pub mod platform;
pub mod text;
/// Re-export easing module from animate for backward compatibility.
pub use animate::easing;
/// Re-export dropped_file module from event for backward compatibility.
pub use event::dropped_file;
/// Re-export responsive module from layout for backward compatibility.
pub use layout::responsive;
/// Re-export file module from platform for backward compatibility.
pub use platform::file;
/// Re-export menu module from platform for backward compatibility.
pub use platform::menu;
/// Re-export view_tuple module from view for backward compatibility.
pub use view::tuple as view_tuple;
pub mod headless;
pub mod style;
pub mod view;
pub mod views;
pub mod window;
pub mod receiver_signal {
    //! Signals from Channels, Futures, and Streams.
    mod channel_signal;
    mod common;
    mod future_signal;
    mod resource;
    mod stream_signal;
    pub use channel_signal::*;
    pub use common::*;
    pub use future_signal::*;
    pub use resource::*;
    pub use stream_signal::*;
}

pub use box_tree::{BoxTree, ElementId, ElementMeta, FocusNavMeta};
pub(crate) use box_tree::{bump_focus_nav_meta_revision, focus_nav_meta_revision};

pub use app::{AppConfig, AppEvent, Application, launch, quit_app, reopen};
pub use floom_legacy_reactive as reactive;
// Re-export the most commonly used reactive primitives at the crate root so they
// can be reached as `floom::Scope`, `floom::create_global_rw_signal`, etc. without
// going through the `reactive` module path. The full set remains available under
// `floom::reactive::*`.
pub use floom_legacy_reactive::{
    Memo, RwSignal, Scope, Trigger, create_effect, create_global_memo, create_global_rw_signal,
    create_global_signal, create_global_trigger, create_memo, create_rw_signal, create_signal,
    create_trigger, root_scope,
};
pub use floom_legacy_renderer::Renderer;
pub use floom_legacy_renderer::Svg as RendererSvg;
pub use floom_legacy_renderer::gpu_resources::GpuResources;
pub use imbl;
pub use layout::ScreenLayout;
#[cfg(not(any(
    target_arch = "wasm32",
    target_os = "android",
    target_os = "ios",
    target_env = "ohos"
)))]
pub use muda;
pub use peniko;
pub use peniko::kurbo;
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub use platform::open_file;
#[cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
pub use platform::save_as;
pub use platform::{Clipboard, ClipboardError, FileDialogOptions, FileInfo, FileSpec};
pub use platform::{Menu, SubMenu};
pub use taffy;
pub use ui_events;
pub use understory_focus;
pub use view::ViewId;
pub use view::{AnyView, HasViewId, IntoView, LazyView, ParentView, View};
pub use view::{Stack, StackOffset};
pub use window::{
    Urgency, WindowIdExt, WindowState, close_window, new_window, request_close_window,
};

#[doc(hidden)]
pub use app::take_close_window_event_count;

/// Re-export unit and theme modules from style for backward compatibility.
pub use style::{theme, unit};

pub mod prelude {
    pub use crate::Renderer;
    pub use crate::event::listener as el;
    pub use crate::event::listener;
    pub use crate::event::listener::EventListenerTrait;
    pub use crate::unit::{DurationUnitExt, UnitExt};
    pub use crate::view::IntoViewIter;
    pub use crate::view::ViewTuple;
    pub use crate::views::*;
    pub use crate::{HasViewId, IntoView, ParentView, View};
    #[allow(deprecated)]
    pub use floom_legacy_reactive::{
        Memo, RwSignal, Scope, SignalGet, SignalTrack, SignalUpdate, SignalWith, Trigger,
        create_effect, create_global_memo, create_global_rw_signal, create_global_signal,
        create_global_trigger, create_memo, create_rw_signal, create_signal, create_trigger,
        root_scope,
    };
    pub use palette::css;
    pub use peniko::Color;
    pub use peniko::color::palette;
    pub use ui_events::{
        keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers, NamedKey},
        pointer::{PointerButtonEvent, PointerEvent},
    };
}

/// Compatibility tier (Phase 29.2): the v1 application surface, preserved
/// verbatim under a stable path.
///
/// [`crate::prelude`] still re-exports exactly this API today, so existing v1
/// applications compile unchanged — that is the preservation policy for the
/// current major version. [`compat::v1_prelude`] pins that path *now*: if a
/// future minor release narrows `prelude` toward the Floom 2.0 surface (see
/// the `floom2` facade crate), this module will not move or change.
///
/// Migration diagnostics: the 2.0 mount point rejects v1 views with a
/// targeted message — `floom_core::Widget` carries an
/// `#[diagnostic::on_unimplemented]` note naming the v1-to-2.0 migration
/// path, so an accidental cross-version mix-up reports where and how to fix
/// it instead of a bare "trait bound not satisfied".
pub mod compat {
    /// The v1 prelude, frozen.
    ///
    /// Import this in v1 application code that should survive a future
    /// narrowing of [`crate::prelude`]. Every name below is the same item
    /// [`crate::prelude`] exports today.
    ///
    /// ```
    /// use floom::compat::v1_prelude::*;
    ///
    /// // The v1 reactive + view API, exactly as it has always been.
    /// let counter = create_rw_signal(0_i32);
    /// let colour: Color = Color::BLACK;
    /// assert_eq!(counter.get(), 0);
    /// ```
    pub mod v1_prelude {
        pub use crate::Renderer;
        pub use crate::event::listener as el;
        pub use crate::event::listener;
        pub use crate::event::listener::EventListenerTrait;
        pub use crate::unit::{DurationUnitExt, UnitExt};
        pub use crate::view::IntoViewIter;
        pub use crate::view::ViewTuple;
        pub use crate::views::*;
        pub use crate::{HasViewId, IntoView, ParentView, View};
        #[allow(deprecated)]
        pub use floom_legacy_reactive::{
            Memo, RwSignal, Scope, SignalGet, SignalTrack, SignalUpdate, SignalWith, Trigger,
            create_effect, create_global_memo, create_global_rw_signal, create_global_signal,
            create_global_trigger, create_memo, create_rw_signal, create_signal, create_trigger,
            root_scope,
        };
        pub use palette::css;
        pub use peniko::Color;
        pub use peniko::color::palette;
        pub use ui_events::{
            keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers, NamedKey},
            pointer::{PointerButtonEvent, PointerEvent},
        };
    }
}
