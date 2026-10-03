//! Experimental platform embedder API.
//!
//! This module is public so backend authors can prototype integrations, but it
//! is **not stable yet**. The API has two layers:
//!
//! - A data-oriented protocol ([`EmbedderEvent`], [`EmbedderCommand`],
//!   [`SurfaceDescriptor`], [`ImeState`], [`BackendCapabilities`]).
//! - Rust adapter traits ([`EmbedderRuntime`], [`EmbedderWindow`],
//!   [`SurfaceHandle`], [`PlatformServices`]).
//!
//! Ordinary applications should continue to use [`crate::launch`].
//!
//! The protocol does not depend on `winit`: backends translate their native
//! events into [`EmbedderEvent`] and execute the returned [`EmbedderCommand`]
//! values. The default desktop path (`floem::launch`) is the first official
//! implementation of this contract.

pub mod capabilities;
pub mod command;
pub mod event;
pub mod headless;
pub mod id;
pub mod ime;
pub mod runtime;
pub mod services;
pub mod surface;

#[cfg(target_os = "android")]
pub mod android;

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_env = "ohos")]
pub mod ohos;

mod window_ext;

pub use capabilities::BackendCapabilities;
pub use command::{CommandBatch, CommandSink, EmbedderCommand, VecCommandSink};
pub use event::{
    DragDropEvent, EmbedderEvent, FocusEvent, GestureEvent, Theme, WindowLifecycleEvent,
};
pub use headless::HeadlessEmbedder;
pub use id::{EmbedderPointerId, EmbedderWindowId};
pub use ime::{ImeEvent, ImeHint, ImePurpose, ImeState};
pub use runtime::EmbedderRuntimeState;
pub use services::{EmbedderRuntime, EmbedderWindow, PlatformServices, SurfaceHandle};
pub use surface::{SurfaceDescriptor, SurfaceEvent, SurfaceKind};
