use floem_text::{LayoutCache, TextAlign, TextLayoutOptions, TextShaper};

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
