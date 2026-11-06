// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-package-info`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-package-info" {
  export interface AppInfo {
    name: string | null;
    package_name: string | null;
    version: string | null;
    build_number: string | null;
  }

  export function app_info(): AppInfo;
}
