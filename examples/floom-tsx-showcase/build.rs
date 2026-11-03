//! Build script: compile the TSX, lower it, and emit Rust into `OUT_DIR`.
//!
//! `floom-tsx` turns `src/counter.tsx` into a `floom_ui_ir::UiDocument`, and
//! `floom-dsl` lowers that into a Rust builder program. The generated `ui_gen.rs`
//! is `include!`d by `main.rs`.

use std::fs;

fn main() {
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");

    let src = fs::read_to_string("src/counter.tsx").expect("read counter.tsx");
    let doc = floom_tsx::compile(&src).expect("compile tsx");

    let opts = floom_dsl::EmitOptions {
        widgets: floom_dsl::WidgetSource::Primitives,
        ..Default::default()
    };
    let mut code = floom_dsl::emit(&doc, &opts).expect("emit").code;

    // The generated file opens with a crate-level inner attribute; drop it so the
    // file can be `include!`d inside a module.
    code = code.replacen("#![allow(unused_imports, clippy::all)]\n", "", 1);

    fs::write(format!("{out}/ui_gen.rs"), code).expect("write ui_gen.rs");

    println!("cargo:rerun-if-changed=src/counter.tsx");
    println!("cargo:rerun-if-changed=build.rs");
}
