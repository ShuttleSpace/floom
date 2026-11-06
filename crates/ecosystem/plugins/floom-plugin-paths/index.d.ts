// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-paths`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-paths" {
  export type AppDirectory =
    | "ApplicationSupport"
    | "Documents"
    | "Cache"
    | "Downloads";

  /** `null` when the platform has no such directory. */
  export function app_dir(dir: AppDirectory): string | null;
  /** Android's external files dir; `null` elsewhere. */
  export function external_files_dir(): string | null;
}
