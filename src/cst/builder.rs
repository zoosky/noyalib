// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

//! Build a [`GreenNode`] from a fully-parsed input.
//!
//! Phase 1 of the green-tree migration: produces a flat
//! `SyntaxKind::Document` node whose children are every source-bearing
//! token, every piece of inter-token trivia, and every comment, in
//! document order. The concatenation of those children's text equals
//! the input byte-for-byte (the round-trip property).
//!
//! Hierarchical nesting (per-mapping / per-sequence parent nodes) is
//! intentionally deferred to a follow-up phase; the typed mutation
//! API will sit on top of it.

use crate::cst::green::{GreenChild, GreenNode};
use crate::cst::syntax::SyntaxKind;
use crate::error::{Error, Result};
use crate::parser::{
    RecordedToken, RecordedTokenKind, ScannedComment, Scanner, TokenKind, Trivia, TriviaKind,
};
use crate::prelude::*;

/// Run a recording scanner over `input` and assemble its outputs into
/// a flat green tree. The function exhausts the token stream so any
/// scanner-level error surfaces here rather than later.
pub(crate) fn build_document(input: &str) -> Result<GreenNode> {
    let mut scanner = Scanner::new(input);
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

    Ok(assemble(input, trivia, tokens, comments))
}

/// Merge the three source-bearing streams (trivia, tokens, comments)
/// into a single ordered child list. The streams are individually in
/// source order, so the merge is a three-way ordered iteration.
fn assemble(
    input: &str,
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
                children.push(token_from_trivia(input, triv));
            }
            (_, Some(tk), c) if min_or_max(c) >= tk => {
                let tok = token_iter.next().expect("peeked Some");
                children.push(token_from_recorded(input, tok));
            }
            (_, _, Some(_)) => {
                let cmt = comment_iter.next().expect("peeked Some");
                children.push(token_from_comment(input, cmt));
            }
            _ => unreachable!(),
        }
    }

    GreenNode::new(SyntaxKind::Document, children)
}

#[inline]
fn min_or_max(opt: Option<usize>) -> usize {
    opt.unwrap_or(usize::MAX)
}

fn token_from_trivia(input: &str, t: Trivia) -> GreenChild {
    let kind = match t.kind {
        TriviaKind::Whitespace => SyntaxKind::Whitespace,
        TriviaKind::Newline => SyntaxKind::Newline,
        TriviaKind::Bom => SyntaxKind::Bom,
        TriviaKind::Directive => SyntaxKind::Directive,
    };
    GreenChild::Token {
        kind,
        text: input[t.start..t.end].to_string().into_boxed_str(),
    }
}

fn token_from_recorded(input: &str, t: RecordedToken) -> GreenChild {
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
        text: input[t.start..t.end].to_string().into_boxed_str(),
    }
}

fn token_from_comment(input: &str, c: ScannedComment) -> GreenChild {
    GreenChild::Token {
        kind: SyntaxKind::Comment,
        text: input[c.start..c.end].to_string().into_boxed_str(),
    }
}
