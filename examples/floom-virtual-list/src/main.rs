//! Golden example: virtual scrolling (Phase 30.1).
//!
//! "Virtualized" is a claim about *cost*, so this example measures one: over a
//! 100 000-item list scrolled end to end, the number of elements ever created
//! stays at the size of the viewport. Everything else is recycled through a
//! small pool keyed by item identity.
//!
//! Run with `cargo run -p floom-virtual-list`.

use floom_core::ElementId;
use floom_virtual::{ItemSizes, Overscan, VirtualViewport};

const ITEM_HEIGHT: f32 = 24.0;
const VIEWPORT_HEIGHT: f32 = 800.0;
const COUNT: usize = 100_000;

fn main() {
    // ~33 rows fit in an 800px viewport, +2 rows of overscan each side.
    let mut viewport = VirtualViewport::<u64, ElementId>::new(
        ItemSizes::uniform(ITEM_HEIGHT, COUNT),
        VIEWPORT_HEIGHT,
    );
    viewport.set_overscan(Overscan::Items(2));

    println!("items        : {COUNT}");
    println!("viewport     : {VIEWPORT_HEIGHT}px @ {ITEM_HEIGHT}px rows");
    println!(
        "visible rows : {}",
        viewport.visible_range().end - viewport.visible_range().start
    );

    let mut pool: Vec<ElementId> = Vec::new();
    let mut next_id = 0u32;
    let mut created = 0usize;
    let mut high_water = 0usize;

    // Scroll the whole list and feed every visible item through the pool.
    for index in 0..COUNT {
        viewport.scroll_to(index as f32 * ITEM_HEIGHT);
        let diff = viewport.update(|item| item as u64);

        for (_, element) in diff.exit {
            pool.push(element);
        }
        for (_, key) in diff.enter {
            let element = pool.pop().unwrap_or_else(|| {
                created += 1;
                let id = ElementId::new(next_id, 1);
                next_id += 1;
                id
            });
            viewport.assign(key, element);
        }

        high_water = high_water.max(viewport.materialized_count());
    }

    let per_viewport = (VIEWPORT_HEIGHT / ITEM_HEIGHT).ceil() as usize + 4; // +overscan both sides
    println!("elements created over the whole scroll : {created}");
    println!("pool high-water mark                   : {high_water}");
    println!("rows materialized at rest              : {}", per_viewport);

    assert!(
        created <= per_viewport * 2,
        "a virtualized list must build roughly one viewport's worth of elements, got {created}"
    );
    assert!(
        high_water <= per_viewport * 2,
        "the live element count must stay bounded, peaked at {high_water}"
    );
    assert!(
        created < COUNT,
        "creating one element per item would mean the viewport is not virtualizing"
    );

    println!(
        "\nvirtual-list golden example: OK ({:.2}% of the list ever materialized)",
        (created as f64 / COUNT as f64) * 100.0
    );
}
