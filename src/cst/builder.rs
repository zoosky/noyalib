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
/// into a nested green tree.
///
/// The builder is a stack-based bracketer. Source-order events from
/// the scanner — the recorded token stream (which now includes
/// zero-length structural markers `BlockMapStart`, `BlockSeqStart`,
/// `BlockEnd`, `SyntheticKey`), the trivia stream, and the comment
/// stream — are merged by start offset and dispatched to a small
/// frame stack:
///
///   * `BlockMapStart` / `BlockSeqStart` push a new
///     [`SyntaxKind::BlockMapping`] / [`SyntaxKind::BlockSequence`]
///     frame.
///   * `BlockEnd` closes the most recent block container, flushing
///     any in-progress [`SyntaxKind::MappingEntry`] /
///     [`SyntaxKind::SequenceItem`] frame first.
///   * `SyntheticKey` and the explicit `?` indicator open a
///     [`SyntaxKind::MappingEntry`] *inside* a block mapping.
///   * `DashIndicator` opens a [`SyntaxKind::SequenceItem`] inside a
///     block sequence.
///   * `OpenBrace` / `OpenBracket` push
///     [`SyntaxKind::FlowMapping`] / [`SyntaxKind::FlowSequence`]
///     frames; their content is kept flat in this phase (no flow
///     `MappingEntry` subdivision yet).
///
/// Round-trip invariant: the depth-first concatenation of every
/// descendant leaf's text equals the original input. Composite
/// frames carry no source bytes of their own.
fn assemble(
    source: Arc<str>,
    trivia: Vec<Trivia>,
    tokens: Vec<RecordedToken>,
    comments: Vec<ScannedComment>,
) -> GreenNode {
    let mut builder = TreeBuilder::new(source);

    let mut trivia_iter = trivia.into_iter().peekable();
    let mut token_iter = tokens.into_iter().peekable();
    let mut comment_iter = comments.into_iter().peekable();

    // Three-stream merge by `start` offset. At equal offsets the
    // recorded-token stream wins so structural events (zero-length)
    // process before any leaf trivia/comment that share the byte.
    loop {
        let nt = trivia_iter.peek().map(|t| t.start);
        let ntok = token_iter.peek().map(|t| t.start);
        let nc = comment_iter.peek().map(|c| c.start);

        match (nt, ntok, nc) {
            (None, None, None) => break,
            (_, Some(tk), c) if min_or_max(c) >= tk && min_or_max(nt) >= tk => {
                let tok = token_iter.next().expect("peeked Some");
                builder.handle_token(tok);
            }
            (Some(t), _, c) if min_or_max(c) >= t => {
                let triv = trivia_iter.next().expect("peeked Some");
                builder.push_leaf(child_from_trivia(triv));
            }
            (_, _, Some(_)) => {
                let cmt = comment_iter.next().expect("peeked Some");
                builder.push_leaf(child_from_comment(cmt));
            }
            _ => unreachable!(),
        }
    }

    builder.finish()
}

#[inline]
fn min_or_max(opt: Option<usize>) -> usize {
    opt.unwrap_or(usize::MAX)
}

/// Single in-progress composite node in the assembler's frame stack.
struct Frame {
    kind: SyntaxKind,
    children: Vec<GreenChild>,
}

/// Stack-based composite-node builder.
struct TreeBuilder {
    source: Arc<str>,
    stack: Vec<Frame>,
}

impl TreeBuilder {
    fn new(source: Arc<str>) -> Self {
        let mut stack = Vec::with_capacity(8);
        stack.push(Frame {
            kind: SyntaxKind::Document,
            children: Vec::new(),
        });
        Self { source, stack }
    }

    fn top_kind(&self) -> SyntaxKind {
        self.stack.last().expect("non-empty").kind
    }

    /// Push `child` into the current top frame.
    fn push_leaf(&mut self, child: GreenChild) {
        self.stack
            .last_mut()
            .expect("non-empty")
            .children
            .push(child);
    }

    /// Push a new composite frame.
    fn push_frame(&mut self, kind: SyntaxKind) {
        self.stack.push(Frame {
            kind,
            children: Vec::new(),
        });
    }

    /// Pop the top frame into its parent as a `GreenChild::Node`.
    /// Never pops the root (`Document`) frame.
    fn pop_frame(&mut self) {
        if self.stack.len() <= 1 {
            return;
        }
        let frame = self.stack.pop().expect("len > 1");
        let node = GreenNode::new(frame.kind, Arc::clone(&self.source), frame.children);
        self.push_leaf(GreenChild::Node(node));
    }

    /// If the current top frame is a `MappingEntry` or
    /// `SequenceItem`, close it. Used at every entry boundary.
    fn close_open_entry(&mut self) {
        if matches!(
            self.top_kind(),
            SyntaxKind::MappingEntry | SyntaxKind::SequenceItem
        ) {
            self.pop_frame();
        }
    }

    /// Walk the stack from the top to find the nearest container
    /// (`BlockMapping`, `BlockSequence`, `FlowMapping`,
    /// `FlowSequence`, or `Document`). Used to decide whether an
    /// entry-opener should subdivide (only inside block contexts in
    /// Phase 1).
    fn nearest_container_kind(&self) -> SyntaxKind {
        for f in self.stack.iter().rev() {
            match f.kind {
                SyntaxKind::BlockMapping
                | SyntaxKind::BlockSequence
                | SyntaxKind::FlowMapping
                | SyntaxKind::FlowSequence
                | SyntaxKind::Document => return f.kind,
                _ => {}
            }
        }
        SyntaxKind::Document
    }

    fn handle_token(&mut self, tok: RecordedToken) {
        use RecordedTokenKind as R;
        use SyntaxKind as S;

        match tok.kind {
            R::BlockMapStart => {
                self.push_frame(S::BlockMapping);
            }
            R::BlockSeqStart => {
                self.push_frame(S::BlockSequence);
            }
            R::BlockEnd => {
                self.close_open_entry();
                // Pop the BlockMapping / BlockSequence container
                // itself. Defensive: only pop if it is one.
                if matches!(
                    self.top_kind(),
                    S::BlockMapping | S::BlockSequence
                ) {
                    self.pop_frame();
                }
            }
            R::SyntheticKey => {
                if matches!(self.nearest_container_kind(), S::BlockMapping) {
                    self.close_open_entry();
                    self.push_frame(S::MappingEntry);
                }
                // Inside a flow mapping (or anywhere else), the
                // implicit-key marker is structurally redundant.
            }
            R::QuestionIndicator => {
                if matches!(self.nearest_container_kind(), S::BlockMapping) {
                    self.close_open_entry();
                    self.push_frame(S::MappingEntry);
                }
                self.push_leaf(leaf_token(S::QuestionIndicator, tok.start..tok.end));
            }
            R::DashIndicator => {
                if matches!(self.nearest_container_kind(), S::BlockSequence) {
                    self.close_open_entry();
                    self.push_frame(S::SequenceItem);
                }
                self.push_leaf(leaf_token(S::DashIndicator, tok.start..tok.end));
            }
            R::OpenBrace => {
                self.push_frame(S::FlowMapping);
                self.push_leaf(leaf_token(S::OpenBrace, tok.start..tok.end));
            }
            R::CloseBrace => {
                self.push_leaf(leaf_token(S::CloseBrace, tok.start..tok.end));
                if matches!(self.top_kind(), S::FlowMapping) {
                    self.pop_frame();
                }
            }
            R::OpenBracket => {
                self.push_frame(S::FlowSequence);
                self.push_leaf(leaf_token(S::OpenBracket, tok.start..tok.end));
            }
            R::CloseBracket => {
                self.push_leaf(leaf_token(S::CloseBracket, tok.start..tok.end));
                if matches!(self.top_kind(), S::FlowSequence) {
                    self.pop_frame();
                }
            }
            // Pure-leaf kinds — push directly.
            R::DocStart => self.push_leaf(leaf_token(S::DocStart, tok.start..tok.end)),
            R::DocEnd => self.push_leaf(leaf_token(S::DocEnd, tok.start..tok.end)),
            R::ColonIndicator => {
                self.push_leaf(leaf_token(S::ColonIndicator, tok.start..tok.end));
            }
            R::Comma => self.push_leaf(leaf_token(S::Comma, tok.start..tok.end)),
            R::AnchorMark => self.push_leaf(leaf_token(S::AnchorMark, tok.start..tok.end)),
            R::AliasMark => self.push_leaf(leaf_token(S::AliasMark, tok.start..tok.end)),
            R::TagMark => self.push_leaf(leaf_token(S::TagMark, tok.start..tok.end)),
            R::PlainScalar => self.push_leaf(leaf_token(S::PlainScalar, tok.start..tok.end)),
            R::SingleQuotedScalar => {
                self.push_leaf(leaf_token(S::SingleQuotedScalar, tok.start..tok.end));
            }
            R::DoubleQuotedScalar => {
                self.push_leaf(leaf_token(S::DoubleQuotedScalar, tok.start..tok.end));
            }
            R::LiteralScalar => self.push_leaf(leaf_token(S::LiteralScalar, tok.start..tok.end)),
            R::FoldedScalar => self.push_leaf(leaf_token(S::FoldedScalar, tok.start..tok.end)),
        }
    }

    fn finish(mut self) -> GreenNode {
        // Close any frames the scanner failed to balance — defensive.
        while self.stack.len() > 1 {
            self.pop_frame();
        }
        let root = self.stack.pop().expect("Document frame");
        GreenNode::new(SyntaxKind::Document, self.source, root.children)
    }
}

fn leaf_token(kind: SyntaxKind, range: core::ops::Range<usize>) -> GreenChild {
    GreenChild::Token { kind, range }
}

fn child_from_trivia(t: Trivia) -> GreenChild {
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

fn child_from_comment(c: ScannedComment) -> GreenChild {
    GreenChild::Token {
        kind: SyntaxKind::Comment,
        range: c.start..c.end,
    }
}
