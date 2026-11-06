// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-share`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-share" {
  export function share_text(text: string): void;
  /** Throws when a path cannot be shared or the host has no share sheet. */
  export function share_files(paths: string[]): void;
}
