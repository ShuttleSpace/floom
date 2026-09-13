use floem_scene::{Resource, ResourceKind, ResourceRegistry};

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
    let mut shaper = floem_text::TextShaper::new();
    let options = floem_text::TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 16.0,
        align: floem_text::TextAlign::Start,
    };
    let laid = shaper.shape("Hello", &options);
    assert!(!laid.runs.is_empty(), "shaping produces at least one run");

    let mut reg = ResourceRegistry::new();
    let id = reg.insert(Resource {
        kind: ResourceKind::GlyphRun {
            runs: laid.runs.clone(),
        },
    });

    let Some(ResourceKind::GlyphRun { runs }) = reg.get(id).map(|r| &r.kind) else {
        panic!("glyph run resource does not round-trip");
    };
    assert_eq!(runs, &laid.runs);
    let glyph_cost: u64 = laid
        .runs
        .iter()
        .map(|run| run.glyphs.len() as u64 * 16 + 64)
        .sum();
    assert_eq!(reg.used_units(), glyph_cost);
    assert!(reg.release(id));
    assert!(reg.get(id).is_none());
    assert_eq!(reg.used_units(), 0);
}
