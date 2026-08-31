gate-desktop:
    cargo check -p floem
    cargo test -p floem-test --test access_tree_harness
    cargo test -p floem-test --test access_testing_api

gate-docs:
    test -f docs/gates/windows.md
    test -f docs/gates/macos.md
    test -f docs/gates/linux.md
    test -f docs/gates/android.md
    test -f docs/gates/ios.md
    test -f docs/gates/ohos.md
