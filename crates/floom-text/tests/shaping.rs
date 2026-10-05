use floom_text::{FontWeight, LayoutCache, TextAlign, TextLayoutOptions, TextShaper};

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
        weight: FontWeight::Normal,
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
        weight: FontWeight::Normal,
    };
    let laid = shaper.shape("hello world from floom text", &opts);
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

// --- weight ----------------------------------------------------------------

/// The claim `font_weight` exists to make: a heavier weight draws a wider line,
/// so a title at `Bold` genuinely occupies more room than the same string at
/// `Normal`. Without this there is nothing to port *to* — the feature could have
/// been a silent no-op and every other test in this file would still pass.
///
/// The assertion is on `Bold`, not `Medium`, and that is the point rather than a
/// detail. The default font stack's faces generally ship a regular and a bold and
/// nothing between, so font matching resolves `500` to the *same face* as `400`
/// and the two measure identically — correctly, per CSS. Asserting
/// `Medium > Normal` would therefore be asserting something about the installed
/// fonts rather than about the code, and would fail on a machine with a variable
/// font installed and pass on one without. `Medium >= Normal` holds either way,
/// and `Bold > Normal` is what proves the weight reached the shaper.
#[test]
fn a_heavier_weight_measures_wider() {
    let options = |weight| TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 20.0,
        align: TextAlign::Start,
        weight,
    };
    let mut shaper = TextShaper::new();
    let regular = shaper
        .shape("Toolbar title", &options(FontWeight::Normal))
        .size;
    let medium = shaper
        .shape("Toolbar title", &options(FontWeight::Medium))
        .size;
    let bold = shaper
        .shape("Toolbar title", &options(FontWeight::Bold))
        .size;

    assert!(
        medium.width >= regular.width,
        "heavier is never narrower: Medium {medium:?} vs Normal {regular:?}"
    );
    assert!(
        bold.width > regular.width,
        "Bold {bold:?} should be wider than Normal {regular:?}"
    );
}

/// The mutation this test exists to catch: dropping `weight` from the style the
/// shaper builds. Everything above would still pass — the string, the size and
/// the align are all right — and every weighted label in the library would come
/// out flat, which is invisible in a diff and obvious on screen.
#[test]
fn the_weight_reaches_the_shaper_rather_than_being_defaulted() {
    let options = |weight| TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 32.0,
        align: TextAlign::Start,
        weight,
    };
    let mut shaper = TextShaper::new();
    let normal = shaper.shape("Weight matters here", &options(FontWeight::Normal));
    let black = shaper.shape("Weight matters here", &options(FontWeight::Black));

    assert_ne!(
        normal.runs, black.runs,
        "the shaped runs must differ, or the weight never reached parley"
    );
    assert!(
        black.size.width > normal.size.width * 1.05,
        "and differ by a visible amount: {:?} vs {:?}",
        black.size.width,
        normal.size.width
    );
}

/// The cache is the reason this needed care. A hit returns the *glyph runs*, so a
/// key that forgot the weight would answer a `Bold` request with the `Normal`
/// runs for the same string — same text, same size, wrong weight, and nothing in
/// the result says so.
///
/// Note this exercises `LayoutCache` directly rather than the thread-local one
/// `measure_text` uses, so the two weights cannot be confused by an unrelated
/// entry from an earlier test in this binary.
#[test]
fn the_cache_does_not_answer_one_weight_with_another_weights_runs() {
    let options = |weight| TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 20.0,
        align: TextAlign::Start,
        weight,
    };
    let mut cache = LayoutCache::new();
    let mut shaper = TextShaper::new();

    let text = "cache key";
    let normal_width = cache
        .get_or_insert_with(text, &options(FontWeight::Normal), || {
            shaper.shape(text, &options(FontWeight::Normal))
        })
        .size
        .width;
    let bold_width = cache
        .get_or_insert_with(text, &options(FontWeight::Bold), || {
            shaper.shape(text, &options(FontWeight::Bold))
        })
        .size
        .width;

    assert_ne!(
        normal_width, bold_width,
        "the second request was served the first one's glyphs"
    );

    // And the control in the other direction: the same weight twice really is a
    // hit, so the weight is not being carried by "everything misses now".
    let again_width = cache
        .get_or_insert_with(text, &options(FontWeight::Normal), || {
            panic!("a repeated identical request must not re-shape")
        })
        .size
        .width;
    assert_eq!(again_width, normal_width);
}

/// The hash has to carry the weight too, or two weights land in the same bucket:
/// the retained key would then catch the mismatch and merely re-shape every time,
/// which is a performance bug wearing the costume of a correctness one. Proved by
/// counting shapes rather than by inspecting the hash.
#[test]
fn two_weights_of_one_string_do_not_share_a_bucket() {
    let options = |weight| TextLayoutOptions {
        max_width: f64::INFINITY,
        font_size: 18.0,
        align: TextAlign::Start,
        weight,
    };
    let mut cache = LayoutCache::new();
    let mut shaper = TextShaper::new();
    let text = "bucket";

    let mut shapes = 0usize;
    for _ in 0..3 {
        for weight in [FontWeight::Normal, FontWeight::Medium, FontWeight::Bold] {
            cache.get_or_insert_with(text, &options(weight), || {
                shapes += 1;
                shaper.shape(text, &options(weight))
            });
        }
    }
    assert_eq!(
        shapes, 3,
        "three distinct weights should shape exactly three times, then hit forever"
    );
}
