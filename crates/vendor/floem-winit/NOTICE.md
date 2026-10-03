# NOTICE — floem-winit (forked third-party crate)

## Upstream source

- **Project:** [lapce/winit](https://github.com/lapce/winit)
- **Forked from rev:** `133268de675a4fda70816d78a7a1435eb972fea1`
  (published as `floem-winit` 25.10.0; same rev, same version — no `[patch]` drift)
- **Upstream license:** Apache-2.0 OR MIT (per lapce/winit)

## Why it is vendored

lapce/winit @ `133268de` owns the `WinitApplicationDelegate` and used to drop both
cold-start (`application:didFinishLaunchingWithOptions:`) and warm-start
(`application:openURL:options:`) shared URLs/files. Floom needs those shares
(floom #109), so this in-tree fork adds a `set_on_open_url` C callback that Floom
registers at startup; the delegate forwards every shared `NSURL` through it.

## Local delta (all in this tree)

- `winit/` — `WinitApplicationDelegate` forwards open-URL shares via a new C callback
  (`set_on_open_url`); Floom registers it at startup.
- Otherwise byte-identical to upstream `133268de` (same version 25.10.0).

## Exit criteria

Delete this tree (and the `[patch."https://github.com/lapce/winit"]` entry in the
root `Cargo.toml`) once lapce/winit merges the open-URL forwarding upstream and the
iOS share gate passes against the published crate.

## See also

- `docs/ios.md` — "Step 8b" describes the fork and the `set_on_open_url` contract.
