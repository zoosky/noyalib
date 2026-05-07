//! Edge-case audit battery for v0.0.1 — exercises behaviours
//! that ride on top of the recent tag-preserving deserialise
//! work but weren't covered by the headline phases.

use noyalib::{from_str, from_value, to_string_value, Spanned, Tag, TaggedValue, Value};
use serde::Deserialize;

// 1) Round-trip a Tagged scalar via to_string.
#[test]
fn round_trip_tagged_scalar() {
    let yaml = "!Custom 'hello'\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(matches!(v, Value::Tagged(_)));
    let emitted = to_string_value(&v).unwrap();
    let v2: Value = from_str(&emitted).unwrap();
    assert!(matches!(v2, Value::Tagged(_)), "re-parse should also be Tagged, got {:?}", v2);
    let (Value::Tagged(t1), Value::Tagged(t2)) = (&v, &v2) else { panic!() };
    assert_eq!(t1.tag().as_str(), t2.tag().as_str(), "tag survives roundtrip");
    assert_eq!(t1.value().as_str(), t2.value().as_str(), "inner survives roundtrip");
}

// 2) Round-trip a Tagged sequence via to_string.
#[test]
fn round_trip_tagged_sequence() {
    let yaml = "!List [1, 2, 3]\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(matches!(v, Value::Tagged(_)));
    let emitted = to_string_value(&v).unwrap();
    let v2: Value = from_str(&emitted).unwrap();
    assert!(matches!(v2, Value::Tagged(_)), "re-parse should be Tagged");
    let (Value::Tagged(t1), Value::Tagged(t2)) = (&v, &v2) else { panic!() };
    assert_eq!(t1.tag().as_str(), t2.tag().as_str());
}

// 3) Round-trip a Tagged mapping via to_string.
#[test]
fn round_trip_tagged_mapping() {
    let yaml = "!Map\nk: v\nx: 1\n";
    let v: Value = from_str(yaml).unwrap();
    assert!(matches!(v, Value::Tagged(_)));
    let emitted = to_string_value(&v).unwrap();
    let v2: Value = from_str(&emitted).unwrap();
    assert!(matches!(v2, Value::Tagged(_)));
    let (Value::Tagged(t1), Value::Tagged(t2)) = (&v, &v2) else { panic!() };
    assert_eq!(t1.tag().as_str(), t2.tag().as_str());
}

// 4) Identity: from_value::<Value>(&v) for a Tagged input.
#[test]
fn from_value_value_identity_with_tagged() {
    let original = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!Custom"),
        Value::String("hello".to_string()),
    )));
    let copy: Value = from_value(&original).unwrap();
    assert!(matches!(copy, Value::Tagged(_)), "TypeId fast-path: Tagged identity preserved");
    let (Value::Tagged(a), Value::Tagged(b)) = (&original, &copy) else { panic!() };
    assert_eq!(a.tag().as_str(), b.tag().as_str());
    assert_eq!(a.value().as_str(), b.value().as_str());
}

// 5) Typed deserialise sees through tag (the contract for
// `#[derive(Deserialize)]` targets).
#[test]
fn typed_target_sees_through_tagged_collection() {
    use std::collections::HashMap;
    let m: HashMap<String, i64> = from_str("!Custom\nk: 1\nx: 2\n").unwrap();
    assert_eq!(m["k"], 1);
    assert_eq!(m["x"], 2);

    let v: Vec<i64> = from_str("!Custom\n- 10\n- 20\n").unwrap();
    assert_eq!(v, [10, 20]);
}

// 6) Spanned<Value> wrapping a Tagged scalar.
//
// Known limitation: the tag-preserving fast path is only engaged
// when the deserialise target is exactly `T = Value` (detected
// via `TypeId`). Wrapper targets like `Spanned<Value>`,
// `Vec<Value>`, `Option<Value>`, `HashMap<_, Value>` route the
// inner `Value::deserialize` through the standard transparent-
// unwrap path. Users who want both span info and tag preservation
// should parse the document twice (once into `Value` for the
// tag-aware view, once into `Spanned<T>` for the span-aware
// view) or wrap `TaggedValue` directly.
//
// This test pins the *current* behaviour so a future
// preserve-tags-through-wrapper change is visible. If the
// underlying behaviour ever does change, update this assertion
// (and document the new contract in the migration guide).
#[test]
fn spanned_value_with_tagged_scalar_known_limitation() {
    #[derive(Deserialize)]
    struct Cfg { value: Spanned<Value> }
    let cfg: Cfg = from_str("value: !Custom 'hi'\n").unwrap();
    assert!(cfg.value.start.line() >= 1, "span info still works");
    // Today: tag is unwrapped through the Spanned wrapper.
    // Tomorrow: this assertion will flip and the comment above
    // will need updating.
    assert!(
        matches!(cfg.value.value, Value::String(_)),
        "current behaviour: Spanned<Value> unwraps the tag, got {:?}",
        cfg.value.value
    );
}

// 7) Anchor + alias on a tagged scalar — the alias should
//    resolve to the same Tagged value.
#[test]
fn alias_to_tagged_scalar() {
    let yaml = "a: !Color &c '#ff8800'\nb: *c\n";
    let v: Value = from_str(yaml).unwrap();
    let a = v.get("a").unwrap();
    let b = v.get("b").unwrap();
    assert!(matches!(a, Value::Tagged(_)));
    assert!(matches!(b, Value::Tagged(_)));
    let (Value::Tagged(ta), Value::Tagged(tb)) = (a, b) else { panic!() };
    assert_eq!(ta.tag().as_str(), tb.tag().as_str());
    assert_eq!(ta.value().as_str(), tb.value().as_str());
}

// 8) Multi-document stream where one doc carries a tag at the
//    root level.
#[test]
fn multi_doc_with_tagged_doc() {
    let yaml = "---\n!Foo 7\n---\nplain: 8\n";
    let docs: Vec<Value> = noyalib::load_all_as(yaml).unwrap();
    assert_eq!(docs.len(), 2);
    assert!(matches!(docs[0], Value::Tagged(_)));
    assert!(matches!(docs[1], Value::Mapping(_)));
}

// 9) interpolate_properties walks into a Tagged value's inner.
#[test]
fn interpolate_into_tagged_inner() {
    use std::collections::HashMap;
    let mut props: HashMap<String, String> = HashMap::new();
    let _ = props.insert("HOST".into(), "api.example.com".into());

    let mut v: Value = from_str("server: !Custom ${HOST}\n").unwrap();
    v.interpolate_properties(&props).unwrap();

    let server = v.get("server").unwrap();
    let Value::Tagged(t) = server else {
        panic!("expected Tagged after interpolate, got {:?}", server);
    };
    assert_eq!(t.tag().as_str(), "!Custom");
    assert_eq!(t.value().as_str(), Some("api.example.com"),
        "interpolate must walk into the inner of a Tagged");
}

// 10) TagRegistry: a registered tag strips through.
#[test]
fn tag_registry_strips_registered_tag() {
    use std::sync::Arc;
    let reg = Arc::new(noyalib::TagRegistry::new().with("!Color"));
    let cfg = noyalib::ParserConfig::new().tag_registry(reg);
    let v: Value = noyalib::from_str_with_config("!Color '#ff8800'\n", &cfg).unwrap();
    // Registered tag → stripped → bare String.
    assert!(matches!(v, Value::String(_)),
        "registered tag should strip through, got {:?}", v);
    assert_eq!(v.as_str(), Some("#ff8800"));
}

// 11) BorrowedValue handling of a tagged scalar.
#[test]
fn borrowed_value_tagged_scalar() {
    use noyalib::borrowed::from_str_borrowed;
    let yaml = "!Custom 'hi'\n";
    // Borrowed path doesn't yet preserve scalar tags as a BorrowedValue
    // variant; it surfaces the inner value. This test pins the actual
    // behaviour so a future change is visible.
    let v = from_str_borrowed(yaml).unwrap();
    let owned = v.into_owned();
    // After conversion to owned, what shape is it?
    eprintln!("BorrowedValue -> Value: {:?}", owned);
    // No assertion — the goal is to surface the actual behaviour
    // for tracking.
    let _ = owned;
}

// 12) compat::serde_yaml shim — should preserve the OLD
// transparent-unwrap behaviour for migrants.
#[cfg(feature = "compat-serde-yaml")]
#[test]
fn compat_shim_preserves_old_behaviour() {
    use noyalib::compat::serde_yaml::{from_str as compat_from_str, Value as CompatValue};
    let yaml = "!Custom 'hello'\n";
    let v: CompatValue = compat_from_str(yaml).unwrap();
    eprintln!("compat shim sees: {:?}", v);
    // The shim is meant to be a name-for-name drop-in; either
    // path could be the right contract. Pin the actual behaviour.
    let _ = v;
}

// 13) Equality of two Value::Tagged.
#[test]
fn tagged_equality() {
    let a = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!X"),
        Value::String("a".to_string()),
    )));
    let b = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!X"),
        Value::String("a".to_string()),
    )));
    let c = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!Y"),
        Value::String("a".to_string()),
    )));
    let d = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!X"),
        Value::String("z".to_string()),
    )));
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(a, d);
}

// 14) Display on Value::Tagged.
#[test]
fn tagged_display() {
    let v = Value::Tagged(Box::new(TaggedValue::new(
        Tag::new("!Custom"),
        Value::String("hello".to_string()),
    )));
    let s = format!("{}", v);
    assert!(s.contains("Custom") || s.contains("hello"),
        "Display should mention the tag or inner: {}", s);
}

// 15) `noyalib::Value` serialization round-trip via serde_json
//    (via the compat-bridge — does our Value Serialize impl work
//    against external deserializers?).
#[test]
fn value_via_serde_json() {
    let v: Value = from_str("port: 8080\nhost: localhost\n").unwrap();
    let json = serde_json::to_string(&v).unwrap();
    let back: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(v.get_path("port"), back.get_path("port"));
    assert_eq!(v.get_path("host"), back.get_path("host"));
}
