// The whole application. There is no Rust file, no Cargo.toml, no build script:
// this one TSX file is the app.
//
//   floom run app.tsx
//
// `floom` turns it into a binary and launches it. The root `<App>` wrapper
// declares the window; everything inside it is a Floom widget, so the layout
// and the platform are expressed here rather than in Rust.

export default function App() {
  return (
    <App title="floom + tsx" size="640x420">
      <Stack axis="vertical" gap={16} padding={24}>
        <Text label="floom + tsx" size={32} foreground="#1a73e8" />
        <Text
          label="This window was built from a single .tsx file."
          foreground="#5f6368"
        />
        <Divider />
        <Stack axis="horizontal" gap={12}>
          <Button label="Filled" variant="filled" />
          <Button label="Text" variant="text" />
        </Stack>
        <Text label="controls" foreground="#202124" />
        <Stack axis="horizontal" gap={12}>
          <Switch label="enabled" checked={true} />
          <Progress value={64} min={0} max={100} />
        </Stack>
      </Stack>
    </App>
  );
}
