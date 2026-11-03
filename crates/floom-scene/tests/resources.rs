use floom_scene::{Resource, ResourceKind, ResourceRegistry};

#[test]
fn retain_release_refcount() {
    let mut reg = ResourceRegistry::new();
    let id = reg.insert(Resource {
        kind: ResourceKind::Blob { bytes: 100 },
    });
    assert_eq!(reg.refs(id), Some(1));
    assert!(reg.retain(id));
    assert_eq!(reg.refs(id), Some(2));
    assert!(reg.release(id));
    assert_eq!(reg.refs(id), Some(1));
    assert!(reg.release(id));
    assert!(reg.get(id).is_none());
}

#[test]
fn shared_resource_survives_one_release() {
    let mut reg = ResourceRegistry::new();
    let id = reg.insert(Resource {
        kind: ResourceKind::Image {
            width: 10,
            height: 10,
            pixels: std::sync::Arc::from(vec![0u8; 10 * 10 * 4].into_boxed_slice()),
        },
    });
    reg.retain(id);
    reg.release(id);
    assert!(reg.get(id).is_some());
    reg.release(id);
    assert!(reg.get(id).is_none());
}

#[test]
fn budget_evicts_single_ref_resources() {
    let mut reg = ResourceRegistry::new().with_budget(150);
    let a = reg.insert(Resource {
        kind: ResourceKind::Blob { bytes: 100 },
    });
    let b = reg.insert(Resource {
        kind: ResourceKind::Blob { bytes: 100 },
    });
    // Inserting b should have evicted a (refs==1).
    assert!(reg.get(a).is_none());
    assert!(reg.get(b).is_some());
    assert!(reg.eviction_count() >= 1);
}

/// A glyph run now carries the fully shaped runs (font blob + glyph ids and
/// positions), and its registry cost accounts every glyph.
#[test]
fn glyph_run_resource_carries_shaped_runs() {
    let mut shaper = floom_text::TextShaper::new();
    let options = floom_text::TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 16.0,
        align: floom_text::TextAlign::Start,
        weight: floom_text::FontWeight::Normal,
        family: floom_types::FontFamily::SansSerif,
    };
    let laid = shaper.shape("Hello", &options);
    assert!(!laid.runs.is_empty(), "shaping produces at least one run");

    // The retained string is part of the resource, not a side table: shaping is
    // lossy, so nothing downstream can reconstruct it from the glyph ids.
    let expected: std::sync::Arc<str> = std::sync::Arc::from("Hello");
    let mut reg = ResourceRegistry::new();
    let id = reg.insert(Resource {
        kind: ResourceKind::glyph_runs_with_text(laid.runs.clone(), expected.clone()),
    });

    let Some(ResourceKind::GlyphRun { runs, text }) = reg.get(id).map(|r| &r.kind) else {
        panic!("glyph run resource does not round-trip");
    };
    assert_eq!(runs, &laid.runs);
    assert_eq!(text, &expected, "the source string survives into the scene");
    // Text is charged by the byte, on top of the per-glyph accounting: budget
    // accounting that treats retained strings as free would let long labels
    // grow unbounded.
    let glyph_cost: u64 = laid
        .runs
        .iter()
        .map(|run| run.glyphs.len() as u64 * 16 + 64)
        .sum::<u64>()
        + expected.len() as u64;
    assert_eq!(reg.used_units(), glyph_cost);
    assert!(reg.release(id));
    assert!(reg.get(id).is_none());
    assert_eq!(reg.used_units(), 0);
}

/// An image resource round-trips its decoded pixels, and its cost is one unit
/// per pixel (the budget currency for images).
#[test]
fn image_resource_carries_decoded_pixels() {
    let mut reg = ResourceRegistry::new();
    let pixels: std::sync::Arc<[u8]> =
        std::sync::Arc::from(vec![7u8; 4 * 3 * 4].into_boxed_slice());
    let id = reg.insert(Resource {
        kind: ResourceKind::Image {
            width: 4,
            height: 3,
            pixels: pixels.clone(),
        },
    });

    let Some(ResourceKind::Image {
        width,
        height,
        pixels: got,
    }) = reg.get(id).map(|r| &r.kind)
    else {
        panic!("image resource does not round-trip");
    };
    assert_eq!((*width, *height), (4, 3));
    assert_eq!(got.as_ref(), pixels.as_ref());
    assert_eq!(reg.used_units(), 4 * 3);
    assert!(reg.release(id));
    assert_eq!(reg.used_units(), 0);
}
