// Example app for the `sample` plugin.
//
// Small on purpose: it proves the plugin links, enables, and is callable, and
// nothing else. The plugin does its real work natively, so there is no widget
// state here worth writing a test around.

use floom::prelude::*;

fn main() {
    floom_plugin_sample::register();
    floom_platform::plugin("sample")
        .enable()
        .expect("enable sample");
    println!("sample plugin version: {}", floom_plugin_sample::version());

    if let Err(e) = floom::run_desktop(
        "sample example",
        Size::new(320.0, 180.0),
        |host: &mut WidgetHost| {
            host.mount_root(
                LayoutStyle::default(),
                Stack::new(Axis::Vertical).child(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    || Box::new(Text::new("sample plugin enabled")),
                )),
            );
            host.frame();
        },
    ) {
        eprintln!("sample example: {e}");
        std::process::exit(1);
    }
}
