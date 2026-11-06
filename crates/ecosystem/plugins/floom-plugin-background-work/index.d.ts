// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-background-work`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-background-work" {
  /** Ask the OS to launch this app at `at`. Throws when the platform cannot. */
  export function schedule_app_launch(at: Date): void;
  /** Withdraw a pending launch request. Throws when the platform never accepted one. */
  export function cancel_scheduled_app_launch(): void;
}
