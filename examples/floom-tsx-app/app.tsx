// The whole application, written in TSX and driven like nsh: import the runtime
// entry, declare the window and the platform guarantees, then `mount` the root
// component. There is no `Cargo.toml`, no `main.rs` and no `floom::run` call in
// this file — `floom run app.tsx` turns it into a binary.
//
//   floom run app.tsx

import { app, mount, platform, window } from "floom";

// The window title and size. `window.*` calls win over the old `<App title
// size>` wrapper, so an author only ever touches TSX.
app.setName("floom + tsx");
window.setSize(640, 420);

// Gate on a capability the way nsh lets an app decide for itself: if the
// backend cannot offer an input method, warn and carry on rather than failing
// to paint. `Warn` / `Fail` / `Exit` are the three policies.
platform.require("ime", "warn");

function App() {
  return (
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
  );
}

// The entry point. `mount(App)` names the root component; the compiler reads it
// and the window/platform calls above into Rust.
mount(App);
