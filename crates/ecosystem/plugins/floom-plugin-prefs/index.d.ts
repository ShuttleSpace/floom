// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-prefs`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-prefs" {
  export class Preferences {
    static open(store: string): Preferences;
    /** `null` when the key is absent. */
    get(key: string): string | null;
    set(key: string, value: string): void;
    remove(key: string): void;
  }
}
