// This file is read by build.rs at compile time, lowered to a
// `floom_ui_ir::UiDocument`, and emitted as Rust by `floom-dsl`. There is no
// `floom create` step: the build script is the toolchain.
//
// Every property below maps to a declared schema prop (axis, gap, padding,
// label, size, foreground, variant) so the document validates without
// substitutions.
export default function Demo() {
  return (
    <Stack axis="vertical" gap={16} padding={24}>
      <Text label="floom-tsx" size={28} foreground="#1a73e8" />
      <Text label="This UI was compiled from TSX at build time." foreground="#5f6368" />
      <Button label="Hello from TSX" variant="filled" />
    </Stack>
  );
}
