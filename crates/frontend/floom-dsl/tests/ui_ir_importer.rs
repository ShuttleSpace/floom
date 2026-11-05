// SPDX-License-Identifier: MIT
//! The design-importer boundary (Phase 26.2), driven from a fixture.
//!
//! The fixture is *normalized* design data — the vocabulary an upstream
//! Figma-specific converter is expected to produce, not Figma's REST shapes.
//! The importer's whole job is the mapping table:
//! auto-layout → Flex, spacing → gap, padding → padding, variants → semantic
//! variants, text styles → typography tokens — and everything it emits must
//! survive the IR's own validator, because a document an agent cannot load is
//! worse than no document at all.

use floom_dsl::ui_ir::{
    DesignAxis, DesignContract, DesignFile, DesignNode, NodeKind, PropValue, Rgba,
    compare, import,
};

const FIXTURE: &str = include_str!("../fixtures/normalized_design.json");

fn fixture_file() -> DesignFile {
    serde_json::from_str(FIXTURE).expect("the fixture is normalized design data")
}

fn imported() -> floom_dsl::ui_ir::ImportedDesign {
    import(&fixture_file()).expect("the fixture imports")
}

#[test]
fn the_fixture_imports_to_a_document_that_validates() {
    let out = imported();
    out.document
        .validate()
        .expect("everything the importer emits must load");
    assert_eq!(
        out.document.root.as_str(),
        "card",
        "the root key is the root frame's name"
    );
}

#[test]
fn auto_layout_maps_to_flex_and_padding_maps_to_padding() {
    let out = imported();
    let card = out.document.node("card").expect("root node");
    assert!(matches!(card.kind, NodeKind::Stack));
    let prop = |name: &str| card.prop(name).expect(name);
    assert_eq!(prop("axis"), &PropValue::Ident("vertical".into()));
    assert_eq!(prop("gap"), &PropValue::Number(8.0));
    // The plan names this mapping twice because it is the one tools get wrong:
    // spacing between children is `gap`, spacing inside the box is `padding`.
    assert_eq!(prop("padding"), &PropValue::Number(16.0));
}

#[test]
fn a_frame_without_layout_is_a_container_and_keeps_its_style() {
    let out = imported();
    let avatar = out.document.node("avatar").expect("avatar node");
    assert!(matches!(avatar.kind, NodeKind::Container));
    assert_eq!(
        avatar.prop("background"),
        Some(&PropValue::Color(Rgba::from_hex("#2266ff").expect("hex")))
    );
    assert_eq!(avatar.prop("radius"), Some(&PropValue::Number(24.0)));
    assert_eq!(avatar.prop("width"), Some(&PropValue::Number(48.0)));
}

#[test]
fn text_styles_map_to_typography_tokens() {
    let out = imported();
    let name = out.document.node("name").expect("text node");
    assert!(matches!(name.kind, NodeKind::Text));
    assert_eq!(
        name.prop("label"),
        Some(&PropValue::Text("Ada Lovelace".into()))
    );
    assert_eq!(name.prop("size"), Some(&PropValue::Number(16.0)));
    assert_eq!(
        name.prop("foreground"),
        Some(&PropValue::Color(Rgba::from_hex("#111111").expect("hex")))
    );

    // The same style lands in the contract as a named ramp entry.
    let contract = &out.contract;
    let body = contract
        .typography
        .iter()
        .find(|t| t.name == "name")
        .expect("the text style becomes a token");
    assert_eq!(body.font_size, 16.0);
}

#[test]
fn variants_map_to_semantic_variants_within_the_domain() {
    let out = imported();
    let follow = out.document.node("follow").expect("button node");
    assert!(matches!(follow.kind, NodeKind::Button));
    assert_eq!(
        follow.prop("label"),
        Some(&PropValue::Text("Follow".into()))
    );
    assert_eq!(
        follow.prop("variant"),
        Some(&PropValue::Ident("filled".into()))
    );

    // The declared variants become contract states, so a checker can demand
    // each one is actually styled.
    let names: Vec<&str> = out
        .contract
        .states
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(names, vec!["filled", "outlined"]);
}

#[test]
fn an_unknown_variant_is_refused_with_the_allowed_set() {
    let mut file = fixture_file();
    file.root.children[2].variants = vec!["ghost".into()];
    let error = import(&file).expect_err("ghost is not a semantic variant");
    match error {
        floom_dsl::ui_ir::ImporterError::UnknownVariant {
            node,
            variant,
            allowed,
        } => {
            assert_eq!(node, "follow");
            assert_eq!(variant, "ghost");
            assert!(
                allowed.contains(&"filled".to_string())
                    && allowed.contains(&"outlined".to_string()),
                "the error names what IS allowed: {allowed:?}"
            );
        }
        other => panic!("expected UnknownVariant, got {other:?}"),
    }
}

#[test]
fn the_contract_matches_the_design_it_was_emitted_from() {
    let out = imported();
    let contract = &out.contract;
    assert_eq!(contract.dimensions.width, 320.0);
    assert_eq!(contract.dimensions.height, 160.0);
    assert_eq!(contract.spacing.gap, 8.0);
    assert_eq!(contract.spacing.padding, [16.0; 4]);
    assert_eq!(contract.radius.corner, 12.0);

    // Every distinct fill is a named role: the two #2266ff uses share one
    // (named after its first user), and the text's #111111 is its own.
    let names: Vec<&str> = contract.colors.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["card", "avatar", "name"]);
    let accent = contract
        .colors
        .iter()
        .find(|c| c.name == "avatar")
        .expect("the shared fill");
    assert_eq!(accent.value.to_hex(), "#2266ff");
}

#[test]
fn importing_is_deterministic_and_self_consistent() {
    let first = imported();
    let second = imported();
    assert_eq!(first.document, second.document);
    assert_eq!(first.contract, second.contract);

    // And the contract the importer emits matches itself: the comparison the
    // contract exists for must say so on its own output.
    let diff = compare(&first.contract, &second.contract);
    assert!(diff.matches, "{:?}", diff.entries);
}

#[test]
fn a_design_and_a_drifted_implementation_disagree_at_named_paths() {
    // The loop this phase exists to close: the design is the expected side,
    // a hand-re-implemented "actual" is the other, and the diff names where
    // they part ways.
    let expected: DesignContract = imported().contract;
    let mut actual = expected.clone();
    actual.spacing.gap = 12.0;
    actual.radius.corner = 0.0;
    let diff = compare(&expected, &actual);
    let paths: Vec<&str> = diff.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(
        paths,
        vec!["spacing.gap", "radius.corner"],
        "field order, not discovery order"
    );
    assert!(!diff.matches);
}

#[test]
fn axis_names_come_from_the_design_not_the_ir() {
    // The normalized vocabulary says "horizontal"; the IR's domain says the
    // same word. The mapping is a rename check, and this pins it.
    let mut file = fixture_file();
    file.root.auto_layout = Some(floom_dsl::ui_ir::AutoLayout {
        axis: DesignAxis::Horizontal,
        gap: 4.0,
        padding: 0.0,
    });
    let out = import(&file).expect("imports");
    assert_eq!(
        out.document
            .node("card")
            .expect("root")
            .prop("axis")
            .cloned(),
        Some(PropValue::Ident("horizontal".into()))
    );
}

#[test]
fn a_child_node_needs_a_name_its_key_can_be_built_from() {
    let mut file = fixture_file();
    file.root.children[0].name = "has space!".into();
    let error = import(&file).expect_err("not a legal key segment");
    assert!(matches!(error, floom_dsl::ui_ir::ImporterError::BadName { .. }));
}

/// A node needs `text` exactly when its role says so.
#[test]
fn a_button_without_text_is_refused() {
    let mut file = fixture_file();
    file.root.children[2].text = None;
    let error = import(&file).expect_err("a button has nothing to announce");
    assert!(matches!(
        error,
        floom_dsl::ui_ir::ImporterError::MissingText { node } if node == "follow"
    ));
}

/// Duplicate names would collide on keys; the importer refuses rather than
/// uniquifying silently.
#[test]
fn duplicate_names_are_refused() {
    let mut file = fixture_file();
    file.root.children[0].name = "name".into();
    let error = import(&file).expect_err("two nodes named `name`");
    assert!(matches!(
        error,
        floom_dsl::ui_ir::ImporterError::DuplicateKey { key } if key == "name"
    ));
}

/// A full DesignNode is never constructed by hand in these tests beyond the
/// fixture; this documents the shape once for readers of the boundary.
#[allow(dead_code)]
fn shape_documentation() {
    let _ = DesignNode {
        name: "x".into(),
        role: floom_dsl::ui_ir::DesignRole::Frame,
        auto_layout: None,
        size: None,
        fill: None,
        corner_radius: 0.0,
        stroke: None,
        text: None,
        variants: vec![],
        children: vec![],
    };
}
