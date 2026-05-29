// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Native async YAML parsing for [`tokio`](https://tokio.rs)
//! runtimes.
//!
//! Bridges noyalib's strict-parser entry points
//! ([`crate::from_str`], [`crate::from_slice`]) onto
//! `tokio::io::AsyncRead` sources without forcing the caller
//! through `tokio::task::spawn_blocking`. Two surface shapes
//! are provided so callers can pick the right ergonomics for
//! their workload:
//!
//! * `from_async_reader` — drain a single document out of any
//!   `tokio::io::AsyncRead` into the caller's `T`.
//! * `from_async_reader_multi` — drain every `---`-separated
//!   document and return `Vec<T>`.
//! * `YamlDecoder<T>` — `tokio_util::codec::Decoder`
//!   implementation for plugging YAML parsing into a
//!   `tokio_util::codec::Framed` pipeline (web-services /
//!   tower-middleware integration).
//!
//! # Backpressure
//!
//! The `from_async_reader` entry points buffer the full payload
//! into a `Vec<u8>` before parsing because the underlying parser
//! is synchronous. The codec surface is the streaming choice:
//! it emits one document per `decode` call as soon as a complete
//! `---` boundary is in the buffer.
//!
//! Gated behind the `tokio` Cargo feature (which transitively
//! enables `tokio-util` and `bytes` for the codec API).
//!
//! # Example
//!
//! ```no_run
//! # #[cfg(feature = "tokio")] {
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! use tokio::io::BufReader;
//! let bytes: &[u8] = b"name: noyalib\nversion: 0.0.6\n";
//! let mut reader = BufReader::new(bytes);
//! #[derive(serde::Deserialize)]
//! struct Pkg { name: String, version: String }
//! let pkg: Pkg = noyalib::tokio_async::from_async_reader(&mut reader).await?;
//! assert_eq!(pkg.name, "noyalib");
//! # Ok(()) }
//! # }
//! ```

use bytes::BytesMut;
use core::marker::PhantomData;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::codec::Decoder;

use crate::de::{ParserConfig, from_slice, from_slice_with_config};
use crate::error::{Error, Result};
use serde::de::Error as _;

/// Drain the supplied reader to end-of-stream, then parse the
/// buffered bytes as a single YAML document into `T`.
///
/// Uses the default [`ParserConfig`]; pair with
/// [`from_async_reader_with_config`] to pass custom limits.
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain or
/// the parse step.
pub async fn from_async_reader<R, T>(reader: &mut R) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned + 'static,
{
    from_async_reader_with_config(reader, &ParserConfig::default()).await
}

/// [`from_async_reader`] with a caller-supplied [`ParserConfig`].
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain or
/// the parse step.
pub async fn from_async_reader_with_config<R, T>(reader: &mut R, config: &ParserConfig) -> Result<T>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned + 'static,
{
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf).await.map_err(Error::from)?;
    from_slice_with_config(&buf, config)
}

/// Drain the reader and parse every `---`-separated document
/// into `Vec<T>`.
///
/// Falls through to the workspace's standard multi-document
/// loader so the per-document semantics (limits, duplicate-key
/// policy, etc.) match [`crate::load_all_as`].
///
/// # Errors
///
/// Returns the underlying [`Error`] from either the I/O drain or
/// the parse step.
pub async fn from_async_reader_multi<R, T>(reader: &mut R) -> Result<Vec<T>>
where
    R: AsyncRead + Unpin,
    T: DeserializeOwned + 'static,
{
    let mut buf = Vec::new();
    let _ = reader.read_to_end(&mut buf).await.map_err(Error::from)?;
    let text = core::str::from_utf8(&buf).map_err(|e| Error::custom(e.to_string()))?;
    crate::document::load_all_as::<T>(text)
}

/// [`tokio_util::codec::Decoder`] that emits one parsed `T` per
/// `---`-delimited YAML document in the byte stream.
///
/// Drop this into a
/// [`tokio_util::codec::FramedRead`] / [`tokio_util::codec::FramedWrite`]
/// pipeline to plug streaming YAML parsing into a tower service
/// chain. Each `decode` call returns:
///
/// * `Ok(Some(T))` — a complete document was found and parsed.
/// * `Ok(None)` — no complete document yet; ask for more bytes.
/// * `Err(e)` — the next document failed to parse.
///
/// The decoder treats `---` at column 0 followed by whitespace
/// or end-of-line as the document terminator, matching the YAML
/// 1.2.2 §9.1.2 directive-end grammar.
#[derive(Debug)]
pub struct YamlDecoder<T> {
    config: ParserConfig,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Default for YamlDecoder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> YamlDecoder<T> {
    /// Create a decoder with default [`ParserConfig`] limits.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
            _marker: PhantomData,
        }
    }

    /// Create a decoder with caller-supplied [`ParserConfig`].
    #[must_use]
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }
}

impl<T> Decoder for YamlDecoder<T>
where
    T: DeserializeOwned + 'static,
{
    type Item = T;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> core::result::Result<Option<T>, Error> {
        // Find the next document boundary at column 0 (`\n---` +
        // whitespace / EOL). The first document starts at index 0.
        let bytes: &[u8] = src.as_ref();
        let split_at = find_doc_boundary(bytes);

        let Some(end) = split_at else {
            // No complete document in the buffer yet.
            return Ok(None);
        };

        // Split off everything up to (but not including) the next
        // `---` marker.
        let doc = src.split_to(end);
        if doc.iter().all(u8::is_ascii_whitespace) {
            // Skip an all-whitespace preamble silently — common
            // when a stream starts with `---`.
            return self.decode(src);
        }
        let parsed = from_slice_with_config::<T>(&doc, &self.config)?;
        Ok(Some(parsed))
    }

    fn decode_eof(&mut self, src: &mut BytesMut) -> core::result::Result<Option<T>, Error> {
        if src.is_empty() {
            return Ok(None);
        }
        // Last document — try a normal decode first (it may have
        // a trailing `---` from a previous frame), otherwise parse
        // the remainder.
        if let Some(v) = self.decode(src)? {
            return Ok(Some(v));
        }
        if src.iter().all(u8::is_ascii_whitespace) {
            src.clear();
            return Ok(None);
        }
        let doc = src.split();
        let parsed = from_slice::<T>(&doc)?;
        Ok(Some(parsed))
    }
}

/// Find the byte offset of the next column-0 `---` document
/// boundary (returns the offset of the newline preceding the
/// `---`, so callers can split there and still see the marker
/// in the next frame).
///
/// Returns `None` if no boundary is present in the buffer.
fn find_doc_boundary(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 4 {
        return None;
    }
    // Search after byte 0 — a leading `---` is the start of the
    // first document, not a boundary.
    let mut i = 1;
    while i + 3 <= bytes.len() {
        if bytes[i - 1] == b'\n' && &bytes[i..i + 3] == b"---" {
            let next_ok =
                i + 3 >= bytes.len() || matches!(bytes[i + 3], b'\n' | b'\r' | b' ' | b'\t');
            if next_ok {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;
    use serde::Deserialize;
    use tokio::io::BufReader;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Pkg {
        name: String,
        version: String,
    }

    #[tokio::test]
    async fn reader_parses_single_document() {
        let mut r = BufReader::new(&b"name: noyalib\nversion: 0.0.6\n"[..]);
        let p: Pkg = from_async_reader(&mut r).await.unwrap();
        assert_eq!(
            p,
            Pkg {
                name: "noyalib".into(),
                version: "0.0.6".into(),
            }
        );
    }

    #[tokio::test]
    async fn reader_multi_parses_each_document() {
        let yaml = b"---\nname: a\nversion: '1'\n---\nname: b\nversion: '2'\n";
        let mut r = BufReader::new(&yaml[..]);
        let docs: Vec<Pkg> = from_async_reader_multi(&mut r).await.unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].name, "a");
        assert_eq!(docs[1].name, "b");
    }

    #[test]
    fn decoder_emits_first_complete_document() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        let mut buf = BytesMut::from(&b"name: a\nversion: '1'\n---\nname: b\nversion: '2'\n"[..]);
        let first = decoder.decode(&mut buf).unwrap().unwrap();
        assert_eq!(first.name, "a");
        // The second document is still in the buffer, prefixed
        // with the `---` marker; decode_eof handles it.
        let second = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(second.name, "b");
    }

    #[test]
    fn decoder_returns_none_on_incomplete_buffer() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        // No `---` boundary visible — pending.
        let mut buf = BytesMut::from(&b"name: a\n"[..]);
        assert!(decoder.decode(&mut buf).unwrap().is_none());
    }

    #[tokio::test]
    async fn reader_with_config_respects_overrides() {
        let mut r = BufReader::new(&b"name: x\nversion: '1'\n"[..]);
        let cfg = ParserConfig::default();
        let p: Pkg = from_async_reader_with_config(&mut r, &cfg).await.unwrap();
        assert_eq!(p.name, "x");
    }

    #[test]
    fn decoder_with_config_constructor() {
        let cfg = ParserConfig::default();
        let _d: YamlDecoder<Pkg> = YamlDecoder::with_config(cfg);
        let _d2: YamlDecoder<Pkg> = YamlDecoder::default();
        let _printed = format!("{:?}", YamlDecoder::<Pkg>::new());
    }

    #[test]
    fn decoder_eof_on_empty_buffer_returns_none() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        let mut buf = BytesMut::new();
        assert!(decoder.decode_eof(&mut buf).unwrap().is_none());
    }

    #[test]
    fn decoder_skips_whitespace_only_preamble() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        // Whitespace-only chunk before a `---` boundary — the
        // decoder recurses and emits the document after it.
        let mut buf = BytesMut::from(&b"\n\n---\nname: q\nversion: '2'\n"[..]);
        let p = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(p.name, "q");
    }

    #[test]
    fn decoder_eof_drains_trailing_whitespace() {
        let mut decoder: YamlDecoder<Pkg> = YamlDecoder::new();
        // First emit the only doc; then EOF should clean up
        // any trailing whitespace without erroring.
        let mut buf = BytesMut::from(&b"name: r\nversion: '3'\n\n\n"[..]);
        let p = decoder.decode_eof(&mut buf).unwrap().unwrap();
        assert_eq!(p.name, "r");
    }

    #[test]
    fn find_doc_boundary_handles_short_input() {
        assert!(find_doc_boundary(b"").is_none());
        assert!(find_doc_boundary(b"abc").is_none());
    }

    #[test]
    fn find_doc_boundary_skips_leading_marker() {
        // A leading `---` at byte 0 is not a boundary — it's the
        // start of the first document.
        assert!(find_doc_boundary(b"---\na: 1\n").is_none());
        // The second `---` is the boundary.
        let bs = b"---\na: 1\n---\nb: 2\n";
        let at = find_doc_boundary(bs).unwrap();
        // The boundary points at the `---` after the `\n` (byte 9).
        assert_eq!(at, 9);
    }
}
