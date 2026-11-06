// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-webview`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string. This is a declaration only:
// the capability is implemented by the Rust crate's native half, not by
// JavaScript, and `provider` is a process-wide native-view handle rather than
// something a browser can construct.

declare module "floom-plugin-webview" {
  /** A slot id handed out by the native view pool. */
  export type NativeSlot = number;

  export function version(): string;
  /** Throws when the native half refuses to enable. */
  export function enable(): void;
  export function disable(): void;
  /** The capability id, stable across releases. */
  export function capability(): number;
  /** Registers the plugin in the process-wide table. */
  export function register(): void;

  /** Navigate the view in `slot`. Throws when the view is gone. */
  export function load_url(slot: NativeSlot, url: string): void;
  /** Post `json` to the page's JS bridge in `slot`. */
  export function post_message(slot: NativeSlot, json: string): void;
  /** Install the page-to-host message callback (`window.floomNative.postMessage`). */
  export function set_message_callback(
    callback: ((slot: NativeSlot, json: string) => void) | null,
  ): void;

  /**
   * The provider for this plugin, if the native half has handed one over.
   * `hostWindow` is the platform's own window handle (an `NSView *` on macOS,
   * an `ANativeWindow *` on Android); the caller keeps it alive.
   */
  export function provider(hostWindow: unknown): unknown;
}
