## What changed

<!-- One paragraph. What behaviour is different after this merges? -->

## Which promise does this touch

`docs/release-policy.md` lists what is semver-locked. Tick every surface this
change moves; an unticked breaking change is the failure this file exists to
prevent.

- [ ] nothing in the semver-locked table
- [ ] `floom::prelude` (46 names, `tests/prelude_surface.rs`)
- [ ] feature-bundle names (`tests/feature_graph.rs`)
- [ ] `Widget` / `WidgetAny` / `WidgetHost`
- [ ] a plugin ABI: `Renderer`, `Platform`, `Window`, `Surface`, `Clipboard`, `Ime`, `NativeViewProvider`
- [ ] `floom-widgets` public API
- [ ] design systems / `floom::advanced` (minor bump + changelog entry is enough)
- [ ] L8 tooling (`floom-testkit`, `floom-devtools`, `floom-accuracy`) — no guarantee

If any of the above is ticked, the PR also updates `CHANGELOG.md` under the next
minor, and says in one line what callers have to do.

## Evidence

- [ ] a new behaviour has a test that fails without the change (paste the failing
      assertion, or the mutation you ran and what went red)
- [ ] `cargo fmt --all --check` in every workspace I touched (root, `floom-editor`,
      `floom-ui`, `plugins` — `cargo fmt` only covers the one it runs in)
- [ ] `floom check` green
- [ ] if a gate was added or loosened: I showed it red once, here: <!-- command + output -->

## Checks a reviewer would run

<!-- The exact commands. "It works" is not a command. -->
