// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Build the parts of a [`crate::cst::Document`] from input bytes.
//!
//! Currently produces three independently-useful artifacts in two
//! parse passes:
//!   * a flat `SyntaxKind::Document` green tree whose leaves
//!     reproduce the input byte-for-byte (Phase 1's round-trip
//!     property);
//!   * a fully-resolved [`Value`] (the existing AST) for typed
//!     read access;
//!   * a [`SpanTree`] aligned with the `Value` for `path`-based span
//!     resolution.
//!
//! The two passes are deliberately kept separate so that strictness
//! fixes in either path are inherited automatically. Optimising into
//! a single pass is a follow-up.
//!
//! Token leaves are stored as `Range<usize>` into a shared
//! `Arc<str>` source — no per-leaf allocation, no copy of the
//! source bytes.

use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
#[cfg(feature = "std")]
use crate::parser::ParseConfig;
use crate::parser::{
    RecordedToken, RecordedTokenKind, ScannedComment, Scanner, TokenKind, Trivia, TriviaKind,
};
use crate::prelude::*;
#[cfg(feature = "std")]
use crate::span_context::SpanTree;
use crate::value::Value;

/// Outcome of a green-tree-aware parse.
#[cfg(feature = "std")]
pub(crate) struct ParsedDocument {
    pub green: GreenNode,
    pub value: Value,
    pub span_tree: SpanTree,
    pub source: Arc<str>,
}

/// Parse `input` once for `Value` + `SpanTree` and once for the green
/// tree. Returns both — the caller wraps them in a `Document`.
#[cfg(feature = "std")]
pub(crate) fn parse_full(input: &str) -> Result<ParsedDocument> {
    let cfg = ParseConfig::default();
    let (value, span_tree) = crate::parser::parse_one(input, &cfg)?;
    let source: Arc<str> = Arc::from(input);
    let green = build_green_tree(Arc::clone(&source))?;
    Ok(ParsedDocument {
        green,
        value,
        span_tree,
        source,
    })
}

/// Walk the token stream once and report `(start, end)` byte ranges
/// for each logical YAML document in `input`.
///
/// A boundary closes when a `...` (DocEnd) ends or just before a
/// fresh `---` (DocStart) lands while a document is already in
/// progress. Trivia between `...` and the next document begins the
/// next document's prologue. If the input has no recognisable
/// document boundaries, returns a single range covering the whole
/// input — including for inputs that are pure trivia.
#[cfg(feature = "std")]
pub(crate) fn document_boundaries(input: &str) -> Result<Vec<(usize, usize)>> {
    let mut scanner = Scanner::new(input);
    scanner.enable_recording();
    loop {
        let tok = scanner
            .next_token()
            .map_err(|e| Error::Parse(e.message.into_owned()))?;
        if matches!(tok.kind, TokenKind::StreamEnd) {
            break;
        }
    }
    let toks = scanner.take_recorded_tokens();
    drop(scanner);

    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut cur_start = 0usize;
    let mut has_content = false;
    let mut saw_explicit_end = false;

    for t in &toks {
        match t.kind {
            RecordedTokenKind::DocStart => {
                if has_content && !saw_explicit_end {
                    // Implicit close before a fresh `---`.
                    out.push((cur_start, t.start));
                    cur_start = t.start;
                }
                has_content = true;
                saw_explicit_end = false;
            }
            RecordedTokenKind::DocEnd => {
                // The `...` token covers three bytes only — extend
                // through the immediately-following line terminator
                // so the doc's source ends on a line break (round-trip
                // expectation: each emitted doc is a complete line).
                let bytes = input.as_bytes();
                let mut close = t.end;
                if bytes.get(close) == Some(&b'\r') {
                    close += 1;
                }
                if bytes.get(close) == Some(&b'\n') {
                    close += 1;
                }
                out.push((cur_start, close));
                cur_start = close;
                has_content = false;
                saw_explicit_end = true;
            }
            _ => {
                has_content = true;
                saw_explicit_end = false;
            }
        }
    }

    if cur_start < input.len() {
        if has_content || out.is_empty() {
            // Trailing bytes form (or extend) a document.
            out.push((cur_start, input.len()));
        } else if let Some(last) = out.last_mut() {
            // Trailing trivia after a `...` with no further content —
            // attach to the prior document so round-trip holds.
            last.1 = input.len();
        }
    }

    if out.is_empty() {
        out.push((0, input.len()));
    }
    Ok(out)
}

/// Run a recording scanner over the source and assemble its outputs
/// into a flat green tree. The function exhausts the token stream so
/// any scanner-level error surfaces here rather than later.
pub(crate) fn build_green_tree(source: Arc<str>) -> Result<GreenNode> {
    let mut scanner = Scanner::new(&source);
    scanner.enable_recording();

    // Drain the token stream — this exercises every parser-relevant
    // check the scanner performs (indentation, tabs, doc markers,
    // directive validation, comment whitespace, …) and surfaces
    // any error before the green tree is assembled.
    loop {
        let tok = scanner
            .next_token()
            .map_err(|e| Error::Parse(e.message.into_owned()))?;
        if matches!(tok.kind, TokenKind::StreamEnd) {
            break;
        }
    }

    let trivia = scanner.take_trivia();
    let tokens = scanner.take_recorded_tokens();
    let comments = scanner.take_comments();
    drop(scanner);

    Ok(assemble(source, trivia, tokens, comments))
}

/// Merge the three source-bearing streams (trivia, tokens, comments)
/// into a single ordered child list. The streams are individually in
/// source order, so the merge is a three-way ordered iteration.
fn assemble(
    source: Arc<str>,
    trivia: Vec<Trivia>,
    tokens: Vec<RecordedToken>,
    comments: Vec<ScannedComment>,
) -> GreenNode {
    let total_len = trivia.len() + tokens.len() + comments.len();
    let mut children: Vec<GreenChild> = Vec::with_capacity(total_len);

    let mut trivia_iter = trivia.into_iter().peekable();
    let mut token_iter = tokens.into_iter().peekable();
    let mut comment_iter = comments.into_iter().peekable();

    // Three-way merge by `start` byte offset. Synthetic and recorded
    // items never overlap because the scanner emits each only once
    // for the bytes it covers — no two streams claim the same byte.
    loop {
        let nt = trivia_iter.peek().map(|t| t.start);
        let ntok = token_iter.peek().map(|t| t.start);
        let nc = comment_iter.peek().map(|c| c.start);

        match (nt, ntok, nc) {
            (None, None, None) => break,
            (Some(t), tok, c) if min_or_max(tok) >= t && min_or_max(c) >= t => {
                let triv = trivia_iter.next().expect("peeked Some");
                children.push(token_from_trivia(triv));
            }
            (_, Some(tk), c) if min_or_max(c) >= tk => {
                let tok = token_iter.next().expect("peeked Some");
                children.push(token_from_recorded(tok));
            }
            (_, _, Some(_)) => {
                let cmt = comment_iter.next().expect("peeked Some");
                children.push(token_from_comment(cmt));
            }
            _ => unreachable!(),
        }
    }

    GreenNode::new(SyntaxKind::Document, source, children)
}

#[inline]
fn min_or_max(opt: Option<usize>) -> usize {
    opt.unwrap_or(usize::MAX)
}

fn token_from_trivia(t: Trivia) -> GreenChild {
    let kind = match t.kind {
        TriviaKind::Whitespace => SyntaxKind::Whitespace,
        TriviaKind::Newline => SyntaxKind::Newline,
        TriviaKind::Bom => SyntaxKind::Bom,
        TriviaKind::Directive => SyntaxKind::Directive,
    };
    GreenChild::Token {
        kind,
        range: t.start..t.end,
    }
}

fn token_from_recorded(t: RecordedToken) -> GreenChild {
    let kind = match t.kind {
        RecordedTokenKind::DocStart => SyntaxKind::DocStart,
        RecordedTokenKind::DocEnd => SyntaxKind::DocEnd,
        RecordedTokenKind::DashIndicator => SyntaxKind::DashIndicator,
        RecordedTokenKind::QuestionIndicator => SyntaxKind::QuestionIndicator,
        RecordedTokenKind::ColonIndicator => SyntaxKind::ColonIndicator,
        RecordedTokenKind::Comma => SyntaxKind::Comma,
        RecordedTokenKind::OpenBracket => SyntaxKind::OpenBracket,
        RecordedTokenKind::CloseBracket => SyntaxKind::CloseBracket,
        RecordedTokenKind::OpenBrace => SyntaxKind::OpenBrace,
        RecordedTokenKind::CloseBrace => SyntaxKind::CloseBrace,
        RecordedTokenKind::AnchorMark => SyntaxKind::AnchorMark,
        RecordedTokenKind::AliasMark => SyntaxKind::AliasMark,
        RecordedTokenKind::TagMark => SyntaxKind::TagMark,
        RecordedTokenKind::PlainScalar => SyntaxKind::PlainScalar,
        RecordedTokenKind::SingleQuotedScalar => SyntaxKind::SingleQuotedScalar,
        RecordedTokenKind::DoubleQuotedScalar => SyntaxKind::DoubleQuotedScalar,
        RecordedTokenKind::LiteralScalar => SyntaxKind::LiteralScalar,
        RecordedTokenKind::FoldedScalar => SyntaxKind::FoldedScalar,
    };
    GreenChild::Token {
        kind,
        range: t.start..t.end,
    }
}

fn token_from_comment(c: ScannedComment) -> GreenChild {
    GreenChild::Token {
        kind: SyntaxKind::Comment,
        range: c.start..c.end,
    }
}
