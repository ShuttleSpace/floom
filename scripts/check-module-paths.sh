#!/usr/bin/env bash
# Modules are declared as `src/name.rs`, never as `src/name/mod.rs`.
#
# Both spellings compile to the same thing, so nothing else catches a
# reintroduced `mod.rs`: the tree drifts back to two conventions and a reader
# has to know which one a given directory uses. This checks the second one is
# gone and keeps it gone.
set -euo pipefail

cd "$(dirname "$0")/.."

# `crates/vendor` holds upstream forks; keeping their layout makes re-syncing a
# fork a copy rather than a merge.
hits=$(find crates examples -name mod.rs -not -path 'crates/vendor/*' | sort || true)

if [ -n "$hits" ]; then
  echo "module paths: a module is declared in a mod.rs:" >&2
  echo "$hits" >&2
  echo "" >&2
  echo "rename <dir>/mod.rs to <dir>.rs; the parent's 'mod <dir>;' keeps working" >&2
  exit 1
fi

echo "module paths: $(find crates examples -name '*.rs' -not -path 'crates/vendor/*' | wc -l | tr -d ' ') files, no mod.rs"
