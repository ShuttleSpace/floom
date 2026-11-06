// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-tray`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-tray" {
  export class MenuItem {
    static new(id: number, label: string): MenuItem;
    static separator(): MenuItem;
    disabled(): MenuItem;
  }

  export type TrayEvent =
    | { kind: "Menu"; id: number }
    | { kind: "Activate" };

  export class Tray {
    static show(label: string, items: MenuItem[]): Tray;
    events(): TrayEvent[];
  }
}
