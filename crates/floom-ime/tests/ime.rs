use floem_ime::{CompositionState, ImeEvent, ImeState};

fn preedit(text: &str, caret: usize) -> ImeEvent {
    ImeEvent::Preedit {
        text: text.to_string(),
        caret,
        selection: None,
    }
}

#[test]
fn preedit_starts_composition() {
    let mut ime = ImeState::new();
    assert!(!ime.is_composing());

    let result = ime.apply(preedit("ni", 2));
    assert!(ime.is_composing());
    assert_eq!(ime.composition, CompositionState::Composing);
    assert_eq!(ime.preedit, "ni");
    assert_eq!(ime.preedit_caret, 2);
    assert!(result.preedit_changed);
}

#[test]
fn empty_preedit_ends_composition() {
    let mut ime = ImeState::new();
    ime.apply(preedit("ni", 2));
    ime.apply(preedit("", 0));

    assert!(!ime.is_composing());
    assert_eq!(ime.composition, CompositionState::Inactive);
    assert!(ime.preedit.is_empty());
}

#[test]
fn caret_is_clamped_into_preedit() {
    let mut ime = ImeState::new();
    ime.apply(preedit("ni", 99));
    assert_eq!(ime.preedit_caret, 2, "caret must clamp to preedit length");
}

#[test]
fn caret_clamps_to_utf8_char_boundary() {
    let mut ime = ImeState::new();
    // "你好" is 6 bytes with boundaries at 0, 3 and 6.
    ime.apply(preedit("你好", 4));
    assert_eq!(
        ime.preedit_caret, 3,
        "a mid-codepoint caret must back up to a char boundary"
    );

    ime.apply(preedit("你好", 100));
    assert_eq!(ime.preedit_caret, 6);
}

#[test]
fn candidate_selection_is_validated_against_list() {
    let mut ime = ImeState::new();
    ime.apply(ImeEvent::Candidates {
        list: vec!["你".into(), "好".into()],
        selected: Some(5),
    });
    assert_eq!(
        ime.selected_candidate, None,
        "an out-of-range selection must be dropped"
    );

    assert!(ime.select_candidate(1));
    assert_eq!(ime.selected_candidate_text(), Some("好"));
    assert!(!ime.select_candidate(9));
    assert_eq!(ime.selected_candidate_text(), Some("好"));
}

#[test]
fn commit_reports_text_and_clears_composition() {
    let mut ime = ImeState::new();
    ime.apply(preedit("nihao", 5));
    ime.apply(ImeEvent::Candidates {
        list: vec!["你好".into()],
        selected: Some(0),
    });

    let result = ime.apply(ImeEvent::Commit {
        text: "你好".to_string(),
    });

    assert_eq!(result.committed.as_deref(), Some("你好"));
    assert!(ime.preedit.is_empty());
    assert!(ime.candidates.is_empty());
    assert_eq!(ime.selected_candidate, None);
    assert!(!ime.is_composing());
    assert!(result.preedit_changed);
    assert!(result.candidates_changed);
}

#[test]
fn committing_direct_text_reports_no_preedit_change() {
    let mut ime = ImeState::new();
    // No composition in flight: a plain keypress commit.
    let result = ime.apply(ImeEvent::Commit {
        text: "a".to_string(),
    });
    assert_eq!(result.committed.as_deref(), Some("a"));
    assert!(!result.preedit_changed);
    assert!(!result.candidates_changed);
}

#[test]
fn cancel_discards_composition() {
    let mut ime = ImeState::new();
    ime.begin_composition();
    ime.apply(preedit("ni", 2));
    ime.apply(ImeEvent::Candidates {
        list: vec!["你".into()],
        selected: Some(0),
    });

    ime.cancel();
    assert!(ime.preedit.is_empty());
    assert_eq!(ime.preedit_caret, 0);
    assert!(ime.candidates.is_empty());
    assert_eq!(ime.selected_candidate, None);
    assert!(!ime.is_composing());
}

#[test]
fn open_changes_are_reported_only_on_transitions() {
    let mut ime = ImeState::new();

    let first = ime.apply(ImeEvent::OpenChanged(true));
    assert_eq!(first.open_changed, Some(true));
    assert!(ime.open);

    let repeat = ime.apply(ImeEvent::OpenChanged(true));
    assert_eq!(repeat.open_changed, None);
    assert!(repeat.is_noop());

    let close = ime.apply(ImeEvent::OpenChanged(false));
    assert_eq!(close.open_changed, Some(false));
    assert!(!ime.open);
}

#[test]
fn editor_selection_changes_are_tracked() {
    let mut ime = ImeState::new();
    let result = ime.apply(ImeEvent::SelectionChanged {
        caret: 5,
        selection: Some((2, 5)),
    });

    assert_eq!(ime.caret, 5);
    assert_eq!(ime.selection, Some((2, 5)));
    assert!(result.selection_changed);
}

#[test]
fn full_fake_ime_session() {
    let mut ime = ImeState::new();
    ime.apply(ImeEvent::SelectionChanged {
        caret: 0,
        selection: None,
    });

    // User starts typing pinyin.
    ime.apply(ImeEvent::OpenChanged(true));
    ime.apply(preedit("ni", 2));
    assert!(ime.is_composing());

    ime.apply(preedit("nihao", 5));
    ime.apply(ImeEvent::Candidates {
        list: vec!["你好".into(), "尼好".into()],
        selected: Some(0),
    });
    assert_eq!(ime.selected_candidate_text(), Some("你好"));

    // User picks the second candidate, then confirms.
    assert!(ime.select_candidate(1));
    let result = ime.apply(ImeEvent::Commit {
        text: "尼好".to_string(),
    });
    assert_eq!(result.committed.as_deref(), Some("尼好"));

    ime.apply(ImeEvent::OpenChanged(false));
    assert!(!ime.open);
    assert!(!ime.is_composing());
    assert!(ime.preedit.is_empty());

    // Direct text is allowed again once the session closes.
    assert!(ime.text_allowed);
    ime.apply(ImeEvent::TextAllowed(false));
    assert!(!ime.text_allowed);
}
