#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floom Authors
#
# Checks every capability plugin against the layout in
# `docs/plugins.md` §"The layout".
#
# The plugin workspace's member list is a glob, so a directory can join it
# without carrying the files the template promises. `gate.sh` lints the
# workspace (`plugins`) and compiles the device arms, but neither of those
# notices a missing README, example, TypeScript facade or ABI header. This is
# that check.
#
# Each failure names the plugin and the file, because "a plugin is incomplete"
# is not actionable.

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLUGINS="$ROOT/crates/ecosystem/plugins"

fail=0
checked=0

require_file() {
  local plugin="$1" path="$2" why="$3"
  if [ ! -s "$path" ]; then
    printf 'FAIL %s: missing %s (%s)\n' "$plugin" "${path#"$PLUGINS"/}" "$why" >&2
    fail=$((fail + 1))
  fi
}

for dir in "$PLUGINS"/floom-plugin-*; do
  [ -d "$dir" ] || continue
  plugin="$(basename "$dir")"
  name="${plugin#floom-plugin-}"
  ident="${name//-/_}"
  checked=$((checked + 1))

  require_file "$plugin" "$dir/Cargo.toml" "the crate manifest"
  require_file "$plugin" "$dir/build.rs" "the native-half build script"
  require_file "$plugin" "$dir/src/lib.rs" "the Rust half"
  require_file "$plugin" "$dir/README.md" "the plugin's own documentation"
  require_file "$plugin" "$dir/index.d.ts" "the TypeScript facade"
  require_file "$plugin" "$dir/example/Cargo.toml" "a runnable example"
  require_file "$plugin" "$dir/example/src/main.rs" "the example's entry point"
  require_file "$plugin" "$dir/native/README.md" "the platform mapping"
  require_file "$plugin" "$dir/native/common/floom_$ident.h" "the C ABI contract"

  header="$dir/native/common/floom_$ident.h"
  if [ -f "$header" ]; then
    # Every plugin implements the lifecycle pair. The third symbol differs by
    # shape: a capability plugin reports a capability id, while a native-view
    # plugin (camera, webview) hands its provider vtable to the Rust half
    # instead and keeps the id in Rust.
    for symbol in "floom_${ident}_enable" "floom_${ident}_disable"; do
      if ! grep -q "$symbol" "$header"; then
        printf 'FAIL %s: %s does not declare %s\n' "$plugin" "${header#"$PLUGINS"/}" "$symbol" >&2
        fail=$((fail + 1))
      fi
    done
    if ! grep -q "floom_${ident}_capability" "$header" \
       && ! grep -q "floom_${ident}_register_provider" "$header"; then
      printf 'FAIL %s: %s declares neither a capability id nor a provider registration\n' \
        "$plugin" "${header#"$PLUGINS"/}" >&2
      fail=$((fail + 1))
    fi
  fi

  dts="$dir/index.d.ts"
  if [ -f "$dts" ] && ! grep -q "declare module \"floom-plugin-$name\"" "$dts"; then
    printf 'FAIL %s: %s does not declare module \"floom-plugin-%s\"\n' \
      "$plugin" "${dts#"$PLUGINS"/}" "$name" >&2
    fail=$((fail + 1))
  fi

  manifest="$dir/example/Cargo.toml"
  if [ -f "$manifest" ] && ! grep -q "floom-plugin-$name = { path = \"..\" }" "$manifest"; then
    printf 'FAIL %s: %s does not depend on the plugin by path\n' \
      "$plugin" "${manifest#"$PLUGINS"/}" >&2
    fail=$((fail + 1))
  fi

  # A plugin that is its own workspace member must stay one: the plugin
  # workspace's `members = ["floom-plugin-*"]` is what makes it buildable.
  if [ -f "$dir/Cargo.toml" ] && ! grep -q '^name = "'"$plugin"'"$' "$dir/Cargo.toml"; then
    printf 'FAIL %s: %s/Cargo.toml does not name the crate %s\n' \
      "$plugin" "$plugin" "$plugin" >&2
    fail=$((fail + 1))
  fi
done

if [ "$checked" -eq 0 ]; then
  printf 'FAIL: no plugins found under %s\n' "$PLUGINS" >&2
  exit 1
fi

if [ "$fail" -ne 0 ]; then
  printf 'plugin templates: %d failure(s) across %d plugin(s)\n' "$fail" "$checked" >&2
  exit 1
fi

printf 'plugin templates: %d plugin(s) conform\n' "$checked"
