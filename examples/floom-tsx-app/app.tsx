// The whole application, written in TSX and driven like nsh: import the runtime
// entry, declare the window and the platform guarantees, then `mount` the root
// component. There is no `Cargo.toml`, no `main.rs` and no `floom::run` call in
// this file — `floom run app.tsx` turns it into a binary.
//
//   floom run app.tsx

import { app, mount, platform, window } from "floom";
import { createSignal } from "floom";

// The window title and size. `window.*` calls win over the old `<App title
// size>` wrapper, so an author only ever touches TSX.
app.setName("floom + tsx");
window.setSize(680, 720);

// Gate on a capability the way nsh lets an app decide for itself: if the
// backend cannot offer an input method, warn and carry on rather than failing
// to paint. `Warn` / `Fail` / `Exit` are the three policies.
platform.require("ime", "warn");

function App() {
  // A reactive string signal. The compiler lowers `createSignal` to a
  // `SyncBaseSignal` seeded with this literal, so the first paint already reads
  // the text rather than starting empty.
  const [status, setStatus] = createSignal("Click the button to flip the signal.");
  // A reactive boolean signal for the Switch.
  const [checked, setChecked] = createSignal(false);
  // A reactive number signal driving the Progress bar. The Slider writes it
  // through `onChange={(v) => setProgress(v)}`; the Progress reads it through
  // `value={progress()}`. Both sides are the same signal, so dragging the
  // slider thumb moves the bar.
  const [progress, setProgress] = createSignal(64);
  // A checkbox and a radio group, each its own boolean signal.
  const [agree, setAgree] = createSignal(false);
  const [monthly, setMonthly] = createSignal(true);
  const [yearly, setYearly] = createSignal(false);
  return (
    <Stack axis="vertical" gap={16} padding={24}>
      <Text label="floom + tsx" size={32} foreground="#1a73e8" />
      <Text label={status()} foreground="#5f6368" />
      <Divider />

      <Text label="Buttons & text" foreground="#202124" />
      <Stack axis="horizontal" gap={12}>
        <Button
          label="Flip signal"
          variant="filled"
          onClick={() => setStatus("signal flipped — reactive state works!")}
        />
        <Button label="Reset" variant="text" onClick={() => setStatus("back to start")} />
      </Stack>

      <Text label="Toggle & slider" foreground="#202124" />
      <Stack axis="horizontal" gap={12}>
        <Switch label={checked() ? "enabled" : "disabled"} checked={checked()} onToggle={() => setChecked(!checked())} />
        <Progress value={progress()} min={0} max={100} />
      </Stack>
      <Text label="drag the slider to move the progress bar (and watch the bar follow)" foreground="#5f6368" />
      <Slider value={progress()} min={0} max={100} onChange={(v) => setProgress(v)} />

      <Divider />
      <Text label="Checkbox & radio" foreground="#202124" />
      <Checkbox label="I agree to the terms" checked={agree()} onToggle={() => setAgree(!agree())} />
      <Radio label="Monthly plan" checked={monthly()} onToggle={() => { setMonthly(true); setYearly(false); }} />
      <Radio label="Yearly plan" checked={yearly()} onToggle={() => { setYearly(true); setMonthly(false); }} />
      <Text label={agree() ? "terms accepted" : "terms not accepted"} foreground="#5f6368" />
      <Text label={monthly() ? "billing: monthly" : "billing: yearly"} foreground="#5f6368" />
    </Stack>
  );
}

// The entry point. `mount(App)` names the root component; the compiler reads it
// and the window/platform calls above into Rust.
mount(App);
