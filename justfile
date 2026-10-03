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

# One command to build an example for a platform:
#   just build floom2-demo macos
#   just build floom2-demo android --release
#   just build ios ios
build example platform *ARGS:
    ./scripts/build.sh {{example}} {{platform}} {{ARGS}}
