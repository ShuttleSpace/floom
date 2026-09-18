//! The smallest Floem 2.0 app.
//!
//! Everything — host, widgets, geometry, layout style — comes from the single
//! `floem2::prelude` glob. This is the Phase 29.1 surface exercised end to
//! end, and the Phase 29.3 release-size floor.
//!
//! Run with `cargo run -p floem2-hello`.

use floem2::prelude::*;

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(160.0, 80.0));

    let root = Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("hello, floem 2.0")),
    ));
    host.mount_root(LayoutStyle::default(), root);

    let stats = host.frame();
    println!("hello floem 2.0: mounted and painted {} widgets", stats.painted);
}
