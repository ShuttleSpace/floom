//! Parse and lower errors carry a 1-based line/column.

use floom_tsx::compile;

#[test]
fn unknown_tag_reports_line_and_column() {
    // A bad component is a lowering error, so the message should carry a
    // line/col pointing at the offending tag.
    let src = r#"
        <Stack>
            <Carousel />
        </Stack>
    "#;
    let err = compile(src).expect_err("unknown component");
    assert_eq!(err.line, Some(3), "error is on line 3 (the <Carousel />)");
    assert_eq!(err.col, Some(13), "column of the tag");
    assert!(
        err.message.contains("unknown component") || err.message.contains("Carousel"),
        "message names the bad component: {}",
        err.message
    );
}

#[test]
fn parse_error_carries_line_and_column() {
    // A malformed JSX attribute value should be reported with a line/column
    // rather than a bare byte offset.
    let src = r#"
        <Stack>
            <Text label={ } />
        </Stack>
    "#;
    let err = compile(src).expect_err("malformed attribute");
    assert!(
        err.line.is_some(),
        "parse error carries a line: {}",
        err.message
    );
    assert!(
        err.col.is_some(),
        "parse error carries a column: {}",
        err.message
    );
}
