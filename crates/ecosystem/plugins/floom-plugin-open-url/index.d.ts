// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-open-url`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-open-url" {
  /** Throws when the string has no scheme or the host has no URL router. */
  export function open_url(url: string): void;
}
