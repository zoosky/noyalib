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

/// Indentation / flow context the sub-tree parser needs to reproduce
/// the conditions under which the original sub-tree was scanned.
///
/// Phase A keeps this minimal — anything beyond block-vs-flow and
/// indent column escalates to a `Document`-scope re-parse. Tag
/// directives are not threaded through (an edit that touches them
/// always escalates), so this struct stays free of references that
/// would force a non-`'static` lifetime.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SubtreeContext {
    /// Column at which the sub-tree begins. Block-collection content
    /// must indent strictly past this column.
    pub indent: usize,
    /// `0` for block context, non-zero when the sub-tree is nested
    /// inside flow brackets. Phase A only wraps for block context;
    /// flow-context sub-trees are passed through verbatim. Reserved
    /// for the flow-context entry point being added next.
    #[allow(dead_code)]
    pub flow_level: u32,
}

impl SubtreeContext {
    pub(crate) fn block_at(indent: usize) -> Self {
        Self {
            indent,
            flow_level: 0,
        }
    }
}

/// Re-parse a `fragment` of YAML and return a green sub-tree of
/// `expected` kind.
///
/// The fragment is the post-edit text of the smallest CST ancestor
/// that fully contains the edit. The returned tree is self-contained:
/// its leaves reference an `Arc<str>` whose bytes are exactly the
/// fragment, with leaf ranges relative to that fragment. The caller
/// is responsible for re-shifting those ranges and rewiring the
/// `source` field when splicing the sub-tree into the parent
/// document's tree.
///
/// Errors:
///   * Re-parse failure — the fragment is not syntactically valid.
///   * Kind mismatch — the parsed root does not match `expected`
///     (the splice would invert shape, e.g. scalar → sequence).
#[cfg(feature = "std")]
pub(crate) fn parse_subtree(
    fragment: &str,
    ctx: SubtreeContext,
    expected: SyntaxKind,
) -> Result<GreenNode> {
    use SyntaxKind as S;

    // Phase A only attempts a localised re-parse for block
    // collections and their direct entries. Scalar leaves and flow
    // collections are deferred to enclosing-scope retries — keeping
    // the wrapper logic simple and the failure modes obvious.
    match expected {
        S::BlockMapping | S::BlockSequence => {
            parse_block_collection(fragment, ctx, expected)
        }
        S::MappingEntry | S::SequenceItem => parse_block_entry(fragment, ctx, expected),
        S::Document => {
            // Whole-document re-parse — equivalent to parse_full's
            // green-tree pass but exposed here so the caller can
            // invoke it through the same sub-tree path.
            build_green_tree(Arc::from(fragment))
        }
        _ => Err(Error::Parse(format!(
            "parse_subtree: unsupported expected kind {expected:?}"
        ))),
    }
}

/// Re-parse a block-collection fragment. The fragment is expected to
/// start at column `ctx.indent`; we strip that column-offset of
/// leading whitespace from each line so the parser sees content at
/// column 0.
#[cfg(feature = "std")]
fn parse_block_collection(
    fragment: &str,
    ctx: SubtreeContext,
    expected: SyntaxKind,
) -> Result<GreenNode> {
    let dedented = dedent_lines(fragment, ctx.indent);
    // The dedented bytes are what we actually feed to the parser.
    let dedent_arc: Arc<str> = Arc::from(dedented.as_str());
    let parsed = build_green_tree(Arc::clone(&dedent_arc))?;
    // Locate the expected collection within the parsed Document.
    let extracted = first_node_of_kind(&parsed, expected).ok_or_else(|| {
        Error::Parse(format!(
            "parse_subtree: re-parsed fragment did not contain a {expected:?} at root"
        ))
    })?;
    // Re-shift the extracted sub-tree's leaf ranges so they sit on a
    // fresh `Arc<str>` whose bytes are the *original* (un-dedented)
    // fragment. We rebuild leaf ranges via column re-indentation —
    // see `reindent_node` for the inverse of `dedent_lines`.
    let original_arc: Arc<str> = Arc::from(fragment);
    let shift = compute_dedent_shift(fragment, ctx.indent);
    Ok(reindent_node(&extracted, &shift, Arc::clone(&original_arc)))
}

/// Re-parse a single block-entry fragment (`MappingEntry` /
/// `SequenceItem`). The wrapper appends a sentinel sibling so the
/// scanner sees a complete collection.
#[cfg(feature = "std")]
fn parse_block_entry(
    fragment: &str,
    ctx: SubtreeContext,
    expected: SyntaxKind,
) -> Result<GreenNode> {
    // An empty / whitespace-only fragment can never be an entry —
    // the wrapper would parse only the sentinel sibling and we'd
    // hand the caller back the sentinel as if it were the user's
    // edit. Reject up front.
    if fragment.trim().is_empty() {
        return Err(Error::Parse(
            "parse_subtree: empty fragment cannot stand as a block entry".into(),
        ));
    }
    let dedented = dedent_lines(fragment, ctx.indent);
    let mut wrapped = String::with_capacity(dedented.len() + 32);
    wrapped.push_str(&dedented);
    if !wrapped.ends_with('\n') {
        wrapped.push('\n');
    }
    match expected {
        SyntaxKind::MappingEntry => wrapped.push_str("__noyalib_x__: 0\n"),
        SyntaxKind::SequenceItem => wrapped.push_str("- 0\n"),
        _ => unreachable!("guarded by caller"),
    }
    let arc: Arc<str> = Arc::from(wrapped.as_str());
    let parsed = build_green_tree(Arc::clone(&arc))?;
    let parent_kind = match expected {
        SyntaxKind::MappingEntry => SyntaxKind::BlockMapping,
        SyntaxKind::SequenceItem => SyntaxKind::BlockSequence,
        _ => unreachable!("guarded by caller"),
    };
    let parent = first_node_of_kind(&parsed, parent_kind).ok_or_else(|| {
        Error::Parse(format!(
            "parse_subtree: re-parsed entry did not produce a {parent_kind:?}"
        ))
    })?;
    // The entry we want is the FIRST child of kind `expected`; the
    // sentinel sibling is the last and is discarded.
    let entry = parent
        .children()
        .find_map(|c| match c {
            GreenChild::Node(n) if n.kind() == expected => Some(n.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            Error::Parse(format!(
                "parse_subtree: extraction failed for {expected:?}"
            ))
        })?;
    let original_arc: Arc<str> = Arc::from(fragment);
    let shift = compute_dedent_shift(fragment, ctx.indent);
    Ok(reindent_node(&entry, &shift, Arc::clone(&original_arc)))
}

/// Strip up to `indent` columns of leading whitespace from each
/// non-empty line of `s`. Lines shorter than `indent` columns of
/// leading whitespace are passed through unchanged — the parser
/// will report whatever indent error the original contained.
fn dedent_lines(s: &str, indent: usize) -> String {
    if indent == 0 {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut stripped = 0usize;
        let bytes = line.as_bytes();
        while stripped < indent && bytes.get(stripped) == Some(&b' ') {
            stripped += 1;
        }
        out.push_str(&line[stripped..]);
    }
    out
}

/// Per-line bookkeeping used to map dedented byte offsets back into
/// the original (indented) fragment.
#[derive(Debug)]
struct DedentShift {
    /// For each *original-line* index, the byte offset at which that
    /// line begins inside the original fragment.
    original_line_starts: Vec<usize>,
    /// For each line, the byte offset at which that line begins
    /// inside the dedented fragment.
    dedented_line_starts: Vec<usize>,
    /// Number of stripped leading whitespace bytes per line.
    stripped_per_line: Vec<usize>,
}

impl DedentShift {
    fn map_offset(&self, dedented_off: usize) -> usize {
        // Binary-search the dedented_line_starts for the line index.
        let line = match self.dedented_line_starts.binary_search(&dedented_off) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start_dedented = self.dedented_line_starts[line];
        let line_start_original = self.original_line_starts[line];
        let stripped = self.stripped_per_line[line];
        line_start_original + stripped + (dedented_off - line_start_dedented)
    }
}

fn compute_dedent_shift(s: &str, indent: usize) -> DedentShift {
    let mut original_line_starts = Vec::new();
    let mut dedented_line_starts = Vec::new();
    let mut stripped_per_line = Vec::new();
    let mut orig_off = 0usize;
    let mut dedent_off = 0usize;
    for line in s.split('\n') {
        original_line_starts.push(orig_off);
        dedented_line_starts.push(dedent_off);
        let bytes = line.as_bytes();
        let mut stripped = 0usize;
        while stripped < indent && bytes.get(stripped) == Some(&b' ') {
            stripped += 1;
        }
        stripped_per_line.push(stripped);
        orig_off += line.len() + 1; // include the splitting `\n`
        dedent_off += line.len() - stripped + 1;
    }
    DedentShift {
        original_line_starts,
        dedented_line_starts,
        stripped_per_line,
    }
}

/// Rebuild `node` so every leaf range is mapped from dedented byte
/// offsets back into the original fragment, and every node carries
/// `new_source`. Composite nodes are reconstructed recursively.
fn reindent_node(node: &GreenNode, shift: &DedentShift, new_source: Arc<str>) -> GreenNode {
    let mut new_children = Vec::with_capacity(node.children().count());
    for child in node.children() {
        match child {
            GreenChild::Token { kind, range } => {
                let mapped_start = shift.map_offset(range.start);
                let mapped_end = shift.map_offset(range.end);
                new_children.push(GreenChild::Token {
                    kind: *kind,
                    range: mapped_start..mapped_end,
                });
            }
            GreenChild::Node(inner) => {
                new_children.push(GreenChild::Node(reindent_node(
                    inner,
                    shift,
                    Arc::clone(&new_source),
                )));
            }
        }
    }
    GreenNode::new(node.kind(), new_source, new_children)
}

fn first_node_of_kind(node: &GreenNode, kind: SyntaxKind) -> Option<GreenNode> {
    for child in node.children() {
        if let GreenChild::Node(n) = child {
            if n.kind() == kind {
                return Some(n.clone());
            }
        }
    }
    None
}

/// Rebuild the green tree replacing the unique sub-tree starting at
/// byte offset `splice_old_start` (and ending at `splice_old_end`,
/// both in the *old* source) with `spliced` — whose leaf ranges are
/// relative to the fragment that was re-parsed and which carries
/// its own `Arc<str>`. Pre-edit children retain their ranges and
/// their source pointer is updated to `new_source`. Post-edit
/// children get their ranges shifted by `delta`. Linear in the
/// total number of green nodes.
#[cfg(feature = "std")]
pub(crate) fn rebuild_with_splice(
    old_root: &GreenNode,
    edit_old_start: usize,
    edit_old_end: usize,
    delta: isize,
    spliced: &GreenNode,
    splice_old_start: usize,
    splice_old_end: usize,
    splice_new_start: usize,
    new_source: Arc<str>,
) -> GreenNode {
    rebuild_for_edit(
        old_root,
        edit_old_start,
        edit_old_end,
        delta,
        spliced,
        splice_old_start,
        splice_old_end,
        splice_new_start,
        new_source,
        0,
    )
}

#[cfg(feature = "std")]
#[allow(clippy::too_many_arguments)]
fn rebuild_for_edit(
    old: &GreenNode,
    edit_old_start: usize,
    edit_old_end: usize,
    delta: isize,
    spliced: &GreenNode,
    splice_old_start: usize,
    splice_old_end: usize,
    splice_new_start: usize,
    new_source: Arc<str>,
    base_offset: usize,
) -> GreenNode {
    // The splice classifies each child against the *splice target*
    // boundary, not the user-edit boundary. The splice target N is
    // a node whose entire range is replaced; the edit happens
    // strictly inside N, so any sibling of N (or of N's ancestors)
    // is entirely pre-N or entirely post-N.
    let _ = edit_old_start;
    let _ = edit_old_end;

    let mut new_children = Vec::with_capacity(old.children().count());
    let mut pos = base_offset;
    for child in old.children() {
        let len = child.text_len();
        let child_start = pos;
        let child_end = pos + len;

        if child_start == splice_old_start
            && child_end == splice_old_end
            && matches!(child, GreenChild::Node(n) if n.kind() == spliced.kind())
        {
            // Exact match — replace this Node child with the
            // spliced sub-tree, shifting its leaf ranges from
            // fragment-relative to absolute in `new_source`.
            new_children.push(GreenChild::Node(absolutize_subtree(
                spliced,
                splice_new_start,
                Arc::clone(&new_source),
            )));
        } else if child_start <= splice_old_start && child_end >= splice_old_end {
            // Child contains the splice target. Recurse.
            match child {
                GreenChild::Token { .. } => {
                    // Defensive: leaves are atomic — they can't
                    // contain a node-shaped target. Keep verbatim
                    // and let the caller's compatibility check
                    // detect the unexpected layout.
                    new_children.push(rebuild_with_new_source(child, Arc::clone(&new_source)));
                }
                GreenChild::Node(inner) => {
                    new_children.push(GreenChild::Node(rebuild_for_edit(
                        inner,
                        edit_old_start,
                        edit_old_end,
                        delta,
                        spliced,
                        splice_old_start,
                        splice_old_end,
                        splice_new_start,
                        Arc::clone(&new_source),
                        child_start,
                    )));
                }
            }
        } else if child_end <= splice_old_start {
            // Pre-splice: ranges remain valid in the new source
            // (positions before the splice target are
            // byte-identical between old and new).
            new_children.push(rebuild_with_new_source(child, Arc::clone(&new_source)));
        } else if child_start >= splice_old_end {
            // Post-splice: shift ranges by `delta`.
            new_children.push(rebuild_shifted_with_new_source(
                child,
                delta,
                Arc::clone(&new_source),
            ));
        } else {
            // Disjoint overlap — a child that partially overlaps
            // the splice target without containing it. This should
            // never happen if the splice target is a real CST
            // node, but we keep the child verbatim and let the
            // outer compatibility check escalate if the result is
            // wrong.
            new_children.push(rebuild_with_new_source(child, Arc::clone(&new_source)));
        }
        pos += len;
    }
    GreenNode::new(old.kind(), new_source, new_children)
}

#[cfg(feature = "std")]
fn rebuild_with_new_source(child: &GreenChild, new_source: Arc<str>) -> GreenChild {
    match child {
        GreenChild::Token { kind, range } => GreenChild::Token {
            kind: *kind,
            range: range.clone(),
        },
        GreenChild::Node(n) => {
            let new_children: Vec<_> = n
                .children()
                .map(|c| rebuild_with_new_source(c, Arc::clone(&new_source)))
                .collect();
            GreenChild::Node(GreenNode::new(n.kind(), new_source, new_children))
        }
    }
}

#[cfg(feature = "std")]
fn rebuild_shifted_with_new_source(
    child: &GreenChild,
    delta: isize,
    new_source: Arc<str>,
) -> GreenChild {
    match child {
        GreenChild::Token { kind, range } => GreenChild::Token {
            kind: *kind,
            range: shift_range(range, delta),
        },
        GreenChild::Node(n) => {
            let new_children: Vec<_> = n
                .children()
                .map(|c| rebuild_shifted_with_new_source(c, delta, Arc::clone(&new_source)))
                .collect();
            GreenChild::Node(GreenNode::new(n.kind(), new_source, new_children))
        }
    }
}

#[cfg(feature = "std")]
fn absolutize_subtree(sub: &GreenNode, splice_at: usize, new_source: Arc<str>) -> GreenNode {
    let new_children: Vec<_> = sub
        .children()
        .map(|c| absolutize_child(c, splice_at, Arc::clone(&new_source)))
        .collect();
    GreenNode::new(sub.kind(), new_source, new_children)
}

#[cfg(feature = "std")]
fn absolutize_child(child: &GreenChild, splice_at: usize, new_source: Arc<str>) -> GreenChild {
    match child {
        GreenChild::Token { kind, range } => GreenChild::Token {
            kind: *kind,
            range: (range.start + splice_at)..(range.end + splice_at),
        },
        GreenChild::Node(n) => GreenChild::Node(absolutize_subtree(n, splice_at, new_source)),
    }
}

#[cfg(feature = "std")]
fn shift_range(r: &core::ops::Range<usize>, delta: isize) -> core::ops::Range<usize> {
    let new_start = (r.start as isize + delta) as usize;
    let new_end = (r.end as isize + delta) as usize;
    new_start..new_end
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
