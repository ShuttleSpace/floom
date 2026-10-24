gate-desktop:
    cargo check -p floom
    cargo test -p floom-legacy-test --test access_tree_harness
    cargo test -p floom-legacy-test --test access_testing_api

gate-docs:
    test -f docs/gates/windows.md
    test -f docs/gates/macos.md
    test -f docs/gates/linux.md
    test -f docs/gates/android.md
    test -f docs/gates/ios.md
    test -f docs/gates/ohos.md

arch-deps:
    ./scripts/check-architecture-deps.sh

# The whole local gate: fmt, clippy, tests, docs, architecture, probe, plugins,
# Android, iOS, goldens. Each check runs on its own and a skip always prints its
# reason. Pass check names to run a subset: `just gate clippy test`.
#
# This exists because CI here never runs: the origin is Gitee and every workflow
# in `.github/workflows/` is unreachable from it, so a red check that only CI
# would have caught stays red forever.
gate *CHECKS:
    ./scripts/gate.sh {{CHECKS}}

# One command to build an example for a platform:
#   just build floom2-demo macos
#   just build floom2-demo android --release
#   just build ios ios
build example platform *ARGS:
    ./scripts/build.sh {{example}} {{platform}} {{ARGS}}
