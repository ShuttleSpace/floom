// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-file-dialog`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-file-dialog" {
  /** Whether this host has a file dialog at all. */
  export const SUPPORTED: boolean;

  export class Options {
    static new(): Options;
    /** The dialog title, or `null` for the platform's default. */
    title: string | null;
    /** A suggested file name, or `null`. */
    suggested_name: string | null;
  }

  /** Resolves to `null` when the user cancels; throws when the host refuses. */
  export function pick_file(options: Options): string | null;
  export function pick_save_file(options: Options): string | null;
  export function pick_directory(options: Options): string | null;
}
