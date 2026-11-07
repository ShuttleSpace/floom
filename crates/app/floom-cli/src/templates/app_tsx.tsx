// The whole application: `floom run app.tsx` builds it, with no Rust by hand.
import { createSignal, mount, window } from "floom";

window.setTitle("{{TITLE}}");
window.setSize(480, 320);

function App() {
  const [message, setMessage] = createSignal("Hello from {{TITLE}}");
  return (
    <Stack axis="vertical" gap={12} padding={24}>
      <Text label={message()} size={24} foreground="#202124" />
      <Button
        label="Say hello"
        variant="filled"
        onClick={() => setMessage("It works.")}
      />
    </Stack>
  );
}

mount(App);
