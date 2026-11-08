// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-sample`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-sample" {
  export function version(): string;

  /** Claims the plugin's platform state. Throws when the native half refuses. */
  export function enable(): void;

  /** Releases whatever `enable` claimed. */
  export function disable(): void;

  /** The capability id, or `null` when this target has no native half. */
  export function capability(): number | null;

  /** Registers this plugin with the process-wide table. */
  export function register(): void;
}
