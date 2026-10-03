//! Golden example: native view composition, and how to write a plugin (Phase 30.1).
//!
//! A "native plugin" is a type implementing [`NativeViewProvider`]: it declares
//! what it can composite, and the host's controller negotiates a mode, applies
//! state, moves focus and tears participants down. This example writes a small
//! plugin (a text-input bridge) and drives it headless, because every one of
//! those promises is checkable without a window:
//!
//! 1. **Negotiation** — a participant is granted only a mode the provider
//!    offers. Asking for more is *downgraded*, never silently granted: a
//!    participant granted a mode its backend cannot compose draws nothing, and
//!    that looks exactly like a participant that was never asked for.
//! 2. **Lifecycle** — bind → apply → focus → destroy, with the controller
//!    tracking what is outstanding so nothing leaks across a rebind.
//! 3. **Pooling** — slots are recycled, so scrolling a list of embedded views
//!    does not allocate one OS handle per row. A recycled slot comes back with
//!    a *bumped generation*, which is how a backend knows to rebind.
//!
//! Run with `cargo run -p floom2-native-plugin`.

use std::collections::HashMap;

use floom_native::{
    ClipSupport, InputMode, InputModes, NativeCapabilities, NativeCompositionMode, NativeError,
    NativeSnapshot, NativeSlot, NativeState, NativeViewController, NativeViewKind,
    NativeViewPool, NativeViewProvider, Rect,
};

/// A plugin: a provider that can composite text inputs as overlays, and nothing
/// else. It records the calls it received so the example can assert on them.
#[derive(Default)]
struct TextInputBridge {
    live: HashMap<NativeSlot, NativeViewKind>,
    focused: Option<NativeSlot>,
    applies: Vec<NativeState>,
    created: usize,
    destroyed: usize,
}

impl NativeViewProvider for TextInputBridge {
    fn capabilities(&self) -> NativeCapabilities {
        NativeCapabilities {
            // Overlay only: exactly the shape of an X11/Win32 bridge (Phase 16).
            modes: &[NativeCompositionMode::Overlay],
            snapshot: false,
            input: InputModes::PASSTHROUGH_ONLY,
            focusable: true,
            clip: ClipSupport::Platform,
        }
    }

    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        self.created += 1;
        self.live.insert(slot, kind);
        Ok(())
    }

    fn destroy(&mut self, slot: NativeSlot) {
        self.destroyed += 1;
        self.live.remove(&slot);
        if self.focused == Some(slot) {
            self.focused = None;
        }
    }

    fn apply(&mut self, _slot: NativeSlot, state: NativeState) {
        self.applies.push(state);
    }

    fn focus(&mut self, slot: NativeSlot) {
        self.focused = Some(slot);
    }

    fn blur(&mut self, slot: NativeSlot) {
        if self.focused == Some(slot) {
            self.focused = None;
        }
    }

    fn snapshot(&mut self, _slot: NativeSlot) -> Option<NativeSnapshot> {
        // This bridge cannot freeze a participant into a texture.
        None
    }

    fn restore(&mut self, _slot: NativeSlot) {}
}

fn area() -> Rect {
    Rect {
        x0: 0.0,
        y0: 0.0,
        x1: 240.0,
        y1: 32.0,
    }
}

fn state(mode: NativeCompositionMode) -> NativeState {
    NativeState {
        rect: area(),
        clip: None,
        scale: 2.0,
        visible: true,
        z: 0,
        input: InputMode::Passthrough,
        mode,
    }
}

fn main() {
    // --- 1. negotiation -----------------------------------------------------
    let mut controller = NativeViewController::new(TextInputBridge::default());
    let caps = controller.capabilities();
    println!("plugin offers : {:?}", caps.modes);
    println!("focusable     : {} clip={:?}", caps.focusable, caps.clip);

    // Ask for the strongest mode first: the plugin downgrades to what it has.
    let preferred = [
        NativeCompositionMode::Texture,
        NativeCompositionMode::Overlay,
    ];
    let negotiated = controller.capabilities().negotiate(&preferred);
    println!("requested     : {preferred:?}");
    println!("negotiated    : {negotiated:?}");
    assert_eq!(
        negotiated,
        Some(NativeCompositionMode::Overlay),
        "a plugin grants only a mode it advertises — Texture was never on offer"
    );

    // --- 2. lifecycle -------------------------------------------------------
    let mut pool = NativeViewPool::<u32>::new();
    let slot = pool.acquire(1);
    let granted = controller
        .bind(slot, NativeViewKind::TextInput, &preferred)
        .expect("the plugin binds a text input");
    println!("\nbound         : {slot:?} mode={granted:?}");
    assert_eq!(granted, negotiated.unwrap());
    assert!(controller.is_bound(slot));
    assert_eq!(controller.bound_count(), 1);

    controller
        .apply(slot, state(granted))
        .expect("applying state to a bound participant works");
    controller.focus(slot).expect("the plugin is focusable");
    println!("focused       : {:?}", controller.focused());
    assert_eq!(controller.focused(), Some(slot));

    controller.blur(slot).expect("blur follows focus");
    assert_eq!(controller.focused(), None);

    // A plugin that cannot snapshot says so; the controller reports `None`
    // rather than fabricating a texture.
    let frozen = controller.snapshot(slot).expect("snapshot is not an error");
    println!("snapshot      : {:?}", frozen.map(|snap| snap.resource));
    assert!(frozen.is_none(), "this plugin advertises snapshot: false");

    controller.destroy(slot).expect("destroy works");
    assert!(!controller.is_bound(slot));
    assert_eq!(controller.bound_count(), 0);
    println!("destroyed     : bound_count={}", controller.bound_count());

    let bridge = controller.into_provider();
    println!(
        "plugin counts : created={} destroyed={} live={} applies={}",
        bridge.created,
        bridge.destroyed,
        bridge.live.len(),
        bridge.applies.len()
    );
    assert_eq!(bridge.created, 1);
    assert_eq!(bridge.destroyed, 1);
    assert!(bridge.live.is_empty(), "destroying a participant releases it");
    assert_eq!(bridge.applies.len(), 1);

    // --- 3. pooling: many participants, bounded live slots ------------------
    let mut pool = NativeViewPool::<u32>::new();
    let mut live: Vec<u32> = Vec::new();
    let mut generations: Vec<u32> = Vec::new();
    for row in 0..1_000u32 {
        let slot = pool.acquire(row);
        live.push(row);
        generations.push(slot.generation);
        if live.len() > 8 {
            let scrolled_away = live.remove(0);
            pool.release(&scrolled_away);
        }
    }
    println!(
        "\npool          : live={} free={} distinct generations={}",
        pool.live_count(),
        pool.free_count(),
        generations.iter().max().unwrap() + 1
    );
    assert!(
        pool.live_count() <= 8,
        "a pooled list of embedded views keeps live slots bounded, got {}",
        pool.live_count()
    );

    // Re-acquiring a released key hands back a slot at a NEW generation, so a
    // backend can tell a recycled slot from one that is still its own.
    let reused = pool.acquire(0);
    println!(
        "recycled slot : id={:?} generation={} (was {})",
        reused.id,
        reused.generation,
        generations[0]
    );
    assert!(
        reused.generation > generations[0],
        "a recycled slot must bump its generation"
    );

    println!("\nnative-plugin golden example: OK");
}
