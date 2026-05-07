//! Regression: `Document::span_at` / `entry().get()` must cover the
//! full tagged-scalar value, not just the `!Tag` prefix. Pre-fix
//! behaviour returned 6..13 ("!Custom") for `name: !Custom 'app-1'`;
//! the fix stretches the span to 6..21 ("!Custom 'app-1'").

#[test]
fn span_covers_tag_plus_scalar_in_mapping_entry() {
    use noyalib::cst::parse_document;
    let src = "name: !Custom 'app-1'\nport: 8080\n";
    let doc = parse_document(src).unwrap();
    let span = doc.span_at("name").unwrap();
    assert_eq!(&src[span.0..span.1], "!Custom 'app-1'");
}

#[test]
fn span_covers_tag_plus_scalar_in_sequence_item() {
    use noyalib::cst::parse_document;
    let src = "list:\n  - !Color '#ff8800'\n  - plain\n";
    let doc = parse_document(src).unwrap();
    let span = doc.span_at("list[0]").unwrap();
    assert_eq!(&src[span.0..span.1], "!Color '#ff8800'");
}

#[test]
fn span_covers_anchor_plus_scalar() {
    use noyalib::cst::parse_document;
    let src = "name: &x foo\nref: *x\n";
    let doc = parse_document(src).unwrap();
    let span = doc.span_at("name").unwrap();
    assert_eq!(&src[span.0..span.1], "&x foo");
}

#[test]
fn entry_get_returns_full_tagged_value() {
    use noyalib::cst::parse_document;
    let src = "name: !Custom 'app-1'\n";
    let mut doc = parse_document(src).unwrap();
    let entry = doc.entry("name");
    assert_eq!(entry.get(), Some("!Custom 'app-1'"));
}
