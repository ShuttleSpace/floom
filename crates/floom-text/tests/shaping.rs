use floem_text::{LayoutCache, TextAlign, TextLayoutOptions, TextShaper};

/// A glyph run is a *positioned* sequence: the pen advances across the run and
/// each glyph carries its own offset from that pen.
///
/// Parley hands back the two separately, and copying its per-glyph offset
/// straight into the position field leaves every glyph at the run origin: the
/// text paints as one overlapping blob. Nothing else notices, because the
/// measured width is the *sum of the advances* and stays correct — the layout
/// reserves exactly the right box for text that renders as a smudge.
#[test]
fn glyph_positions_advance_across_the_run() {
    let mut shaper = TextShaper::new();
    let opts = TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 20.0,
        align: TextAlign::Start,
    };
    let laid = shaper.shape("click me", &opts);
    let run = &laid.runs[0];

    let xs: Vec<f32> = run.glyphs.iter().map(|g| g.x).collect();
    assert!(
        xs.windows(2).all(|w| w[1] > w[0]),
        "each glyph sits further along the run than the one before it: {xs:?}"
    );
    let last = run.glyphs.last().expect("glyphs");
    assert!(
        (last.x + last.advance - laid.size.width as f32).abs() < 0.01,
        "the run ends where the layout says it does: {} vs {}",
        last.x + last.advance,
        laid.size.width
    );
}

#[test]
fn ascii_wraps_to_multiple_lines() {
    let mut shaper = TextShaper::new();
    let opts = TextLayoutOptions {
        max_width: 40.0,
        font_size: 14.0,
        align: TextAlign::Start,
    };
    let laid = shaper.shape("hello world from floem text", &opts);
    assert!(
        laid.lines.len() >= 2,
        "expected wrapping, got {} lines",
        laid.lines.len()
    );
    assert!(laid.size.width <= opts.max_width + 1.0);
    assert!(!laid.runs.is_empty());
}

#[test]
fn bidi_arabic_produces_runs() {
    let mut shaper = TextShaper::new();
    let opts = TextLayoutOptions::default();
    // Arabic "Hello" + Latin mix exercises bidi.
    let laid = shaper.shape("Hello مرحبا World", &opts);
    assert!(!laid.lines.is_empty());
    assert!(laid.size.height > 0.0);
    // Mixed direction typically yields ≥1 glyph run.
    assert!(laid.run_count >= 1);
}

#[test]
fn emoji_and_empty_do_not_panic() {
    let mut shaper = TextShaper::new();
    let opts = TextLayoutOptions::default();
    let empty = shaper.shape("", &opts);
    assert_eq!(empty.lines.len(), 1.min(empty.lines.len())); // may be 0 or 1
    let emoji = shaper.shape("😀🎉", &opts);
    assert!(emoji.size.height >= 0.0);
}

#[test]
fn selection_caret_advances() {
    let mut shaper = TextShaper::new();
    let opts = TextLayoutOptions {
        font_size: 16.0,
        ..Default::default()
    };
    let x0 = shaper.caret_x("abcd", &opts, 0);
    let x2 = shaper.caret_x("abcd", &opts, 2);
    assert!(x2 > x0);
}

#[test]
fn layout_cache_hits() {
    let mut shaper = TextShaper::new();
    let mut cache = LayoutCache::new();
    let opts = TextLayoutOptions::default();
    let text = "cached shaping";

    cache.get_or_insert_with(text, &opts, || shaper.shape(text, &opts));
    cache.get_or_insert_with(text, &opts, || shaper.shape(text, &opts));
    assert_eq!(cache.misses(), 1);
    assert_eq!(cache.hits(), 1);
    assert_eq!(cache.len(), 1);
}

#[test]
fn layout_cache_stays_within_its_capacity() {
    let mut shaper = TextShaper::new();
    let mut cache = LayoutCache::with_capacity(4);
    let opts = TextLayoutOptions::default();

    for i in 0..20 {
        let text = format!("label {i}");
        cache.get_or_insert_with(&text, &opts, || shaper.shape(&text, &opts));
        assert!(cache.len() <= 4, "cache grew past its cap: {}", cache.len());
    }
    assert_eq!(cache.len(), 4);
    assert_eq!(cache.capacity(), 4);
}

#[test]
fn layout_cache_evicts_least_recently_used() {
    let mut shaper = TextShaper::new();
    let mut cache = LayoutCache::with_capacity(2);
    let opts = TextLayoutOptions::default();

    let shape = |cache: &mut LayoutCache, text: &str, shaper: &mut TextShaper| {
        cache.get_or_insert_with(text, &opts, || shaper.shape(text, &opts));
    };

    shape(&mut cache, "a", &mut shaper);
    shape(&mut cache, "b", &mut shaper);
    // Touch "a" so "b" becomes the least-recently-used entry.
    shape(&mut cache, "a", &mut shaper);
    shape(&mut cache, "c", &mut shaper);

    assert_eq!(cache.len(), 2, "full, but not over");
    // "b" should have been evicted; "a" and "c" remain cached.
    let hits_before = cache.hits();
    shape(&mut cache, "a", &mut shaper);
    shape(&mut cache, "c", &mut shaper);
    assert_eq!(cache.hits(), hits_before + 2, "a and c still cached");
    let hits_before = cache.hits();
    shape(&mut cache, "b", &mut shaper);
    assert_eq!(cache.hits(), hits_before, "b was evicted, so it re-shaped");
}

#[test]
fn shrinking_capacity_evicts_immediately() {
    let mut shaper = TextShaper::new();
    let mut cache = LayoutCache::with_capacity(8);
    let opts = TextLayoutOptions::default();

    for i in 0..8 {
        let text = format!("row {i}");
        cache.get_or_insert_with(&text, &opts, || shaper.shape(&text, &opts));
    }
    assert_eq!(cache.len(), 8);

    cache.set_capacity(3);
    assert_eq!(cache.len(), 3, "shrinking the cap trims on the spot");
    assert_eq!(cache.capacity(), 3);
}

#[test]
fn zero_capacity_is_clamped_to_one() {
    let cache = LayoutCache::with_capacity(0);
    assert_eq!(cache.capacity(), 1, "the cache must hold what it returns");
}
