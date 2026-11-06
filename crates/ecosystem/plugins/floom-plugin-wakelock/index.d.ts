// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-wakelock`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-wakelock" {
  /** Throws when this host has no keep-awake API (Linux) or the call fails. */
  export function keep_screen_on(on: boolean): void;
  export function is_keep_screen_on(): boolean;
}
