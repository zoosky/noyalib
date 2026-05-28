// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! `sval` adapter — stream noyalib values through any
//! [`sval::Stream`] consumer.
//!
//! Provides an alternative to the default serde route for callers
//! who want to avoid `serde_derive`'s compile-time overhead or the
//! binary-size cost of serde monomorphisation. `sval` is a small,
//! streaming serialization framework: instead of materialising a
//! data graph, the producer walks the source and emits events on
//! a [`sval::Stream`] (similar in spirit to YAML's own event-driven
//! parser).
//!
//! Gated behind the `sval` Cargo feature.
//!
//! # API surface
//!
//! * `impl sval::Value for [crate::Value]` — stream a noyalib
//!   value graph to any [`sval::Stream`].
//! * `impl sval::Value for [crate::Number]` — stream a single
//!   number.
//! * [`to_sval_writer`] — high-level helper that streams a
//!   noyalib [`crate::Value`] to a writer that implements
//!   [`sval::Stream`].
//!
//! serde remains the default route for typed deserialise; `sval`
//! is an additive, opt-in surface for callers that prefer the
//! streaming framework. The two routes share `Value`, so a
//! roundtrip-via-`Value` works as expected.
//!
//! # Example
//!
//! ```
//! use noyalib::Value;
//! let v: Value = noyalib::from_str("name: noyalib").unwrap();
//! // The `impl sval::Value for Value` lets you hand any
//! // noyalib-parsed graph to any `sval::Stream` consumer.
//! // Concrete stream impls are supplied by ecosystem crates
//! // such as `sval_fmt`, `sval_json`, or your own — noyalib
//! // does not pull those in.
//! assert!(matches!(v, Value::Mapping(_)));
//! ```

use crate::value::{Mapping, MappingAny, Number, TaggedValue, Value};

impl sval::Value for Value {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        match self {
            Value::Null => stream.null(),
            Value::Bool(b) => stream.bool(*b),
            Value::Number(n) => n.stream(stream),
            Value::String(s) => stream.value(s.as_str()),
            Value::Sequence(items) => stream_seq(items, stream),
            Value::Mapping(m) => m.stream(stream),
            Value::Tagged(t) => t.stream(stream),
        }
    }
}

impl sval::Value for Number {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        match self {
            Number::Integer(i) => stream.i64(*i),
            Number::Float(f) => stream.f64(*f),
        }
    }
}

impl sval::Value for Mapping {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.map_begin(Some(self.len()))?;
        for (k, v) in self.iter() {
            stream.map_key_begin()?;
            stream.value(k.as_str())?;
            stream.map_key_end()?;
            stream.map_value_begin()?;
            stream.value(v)?;
            stream.map_value_end()?;
        }
        stream.map_end()
    }
}

impl sval::Value for MappingAny {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        stream.map_begin(Some(self.len()))?;
        for (k, v) in self.iter() {
            stream.map_key_begin()?;
            stream.value(k)?;
            stream.map_key_end()?;
            stream.map_value_begin()?;
            stream.value(v)?;
            stream.map_value_end()?;
        }
        stream.map_end()
    }
}

impl sval::Value for TaggedValue {
    fn stream<'sval, S: sval::Stream<'sval> + ?Sized>(&'sval self, stream: &mut S) -> sval::Result {
        // sval has no first-class "YAML tag" concept; surface the
        // tag as a sval `tag` annotation around the inner value
        // so consumers that care can introspect it, then stream
        // the inner value normally.
        let label = sval::Label::new_computed(self.tag().as_str());
        stream.tagged_begin(None, Some(&label), None)?;
        stream.value(self.value())?;
        stream.tagged_end(None, Some(&label), None)
    }
}

fn stream_seq<'sval, S: sval::Stream<'sval> + ?Sized>(
    items: &'sval [Value],
    stream: &mut S,
) -> sval::Result {
    stream.seq_begin(Some(items.len()))?;
    for item in items {
        stream.seq_value_begin()?;
        stream.value(item)?;
        stream.seq_value_end()?;
    }
    stream.seq_end()
}

/// Stream a [`Value`] into a writer that implements
/// [`sval::Stream`].
///
/// Convenience wrapper around the [`sval::Value`] impl on
/// [`Value`]: just calls `value.stream(stream)`. Useful as a
/// named entry point so call sites read as
/// `noyalib::sval_adapter::to_sval_writer(&mut stream, &value)`.
///
/// # Errors
///
/// Returns the underlying [`sval::Error`] from the stream
/// implementation.
pub fn to_sval_writer<'sval, S: sval::Stream<'sval> + ?Sized>(
    stream: &mut S,
    value: &'sval Value,
) -> sval::Result {
    sval::Value::stream(value, stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Mapping, Number, Sequence};

    /// Minimal `sval::Stream` that just records the event
    /// sequence as strings — enough to assert structure.
    #[derive(Default)]
    struct Recorder(Vec<String>);

    impl sval::Stream<'_> for Recorder {
        fn null(&mut self) -> sval::Result {
            self.0.push("null".into());
            Ok(())
        }
        fn bool(&mut self, v: bool) -> sval::Result {
            self.0.push(format!("bool({v})"));
            Ok(())
        }
        fn i64(&mut self, v: i64) -> sval::Result {
            self.0.push(format!("i64({v})"));
            Ok(())
        }
        fn f64(&mut self, v: f64) -> sval::Result {
            self.0.push(format!("f64({v})"));
            Ok(())
        }
        fn text_begin(&mut self, _: Option<usize>) -> sval::Result {
            self.0.push("text_begin".into());
            Ok(())
        }
        fn text_fragment_computed(&mut self, fragment: &str) -> sval::Result {
            self.0.push(format!("text({fragment})"));
            Ok(())
        }
        fn text_end(&mut self) -> sval::Result {
            self.0.push("text_end".into());
            Ok(())
        }
        fn map_begin(&mut self, _: Option<usize>) -> sval::Result {
            self.0.push("map_begin".into());
            Ok(())
        }
        fn map_end(&mut self) -> sval::Result {
            self.0.push("map_end".into());
            Ok(())
        }
        fn map_key_begin(&mut self) -> sval::Result {
            self.0.push("map_key_begin".into());
            Ok(())
        }
        fn map_key_end(&mut self) -> sval::Result {
            self.0.push("map_key_end".into());
            Ok(())
        }
        fn map_value_begin(&mut self) -> sval::Result {
            self.0.push("map_value_begin".into());
            Ok(())
        }
        fn map_value_end(&mut self) -> sval::Result {
            self.0.push("map_value_end".into());
            Ok(())
        }
        fn seq_begin(&mut self, _: Option<usize>) -> sval::Result {
            self.0.push("seq_begin".into());
            Ok(())
        }
        fn seq_end(&mut self) -> sval::Result {
            self.0.push("seq_end".into());
            Ok(())
        }
        fn seq_value_begin(&mut self) -> sval::Result {
            self.0.push("seq_value_begin".into());
            Ok(())
        }
        fn seq_value_end(&mut self) -> sval::Result {
            self.0.push("seq_value_end".into());
            Ok(())
        }
    }

    #[test]
    fn null_streams_null_event() {
        let mut r = Recorder::default();
        sval::Value::stream(&Value::Null, &mut r).unwrap();
        assert_eq!(r.0, vec!["null".to_string()]);
    }

    #[test]
    fn bool_streams_bool_event() {
        let mut r = Recorder::default();
        sval::Value::stream(&Value::Bool(true), &mut r).unwrap();
        assert_eq!(r.0, vec!["bool(true)".to_string()]);
    }

    #[test]
    fn integer_streams_i64_event() {
        let mut r = Recorder::default();
        sval::Value::stream(&Value::Number(Number::Integer(42)), &mut r).unwrap();
        assert_eq!(r.0, vec!["i64(42)".to_string()]);
    }

    #[test]
    fn float_streams_f64_event() {
        let mut r = Recorder::default();
        sval::Value::stream(&Value::Number(Number::Float(2.5)), &mut r).unwrap();
        assert_eq!(r.0, vec!["f64(2.5)".to_string()]);
    }

    #[test]
    fn sequence_streams_seq_events() {
        let mut r = Recorder::default();
        let v: Sequence = vec![Value::Bool(true), Value::Bool(false)];
        sval::Value::stream(&Value::Sequence(v), &mut r).unwrap();
        let s = r.0.join(",");
        assert!(s.contains("seq_begin"));
        assert!(s.contains("bool(true)"));
        assert!(s.contains("bool(false)"));
        assert!(s.contains("seq_end"));
    }

    #[test]
    fn mapping_streams_map_events() {
        let mut r = Recorder::default();
        let mut m = Mapping::new();
        let _ = m.insert("k".to_string(), Value::Bool(true));
        sval::Value::stream(&Value::Mapping(m), &mut r).unwrap();
        let s = r.0.join(",");
        assert!(s.contains("map_begin"));
        assert!(s.contains("map_key_begin"));
        assert!(s.contains("text(k)"));
        assert!(s.contains("map_value_begin"));
        assert!(s.contains("bool(true)"));
        assert!(s.contains("map_end"));
    }
}
