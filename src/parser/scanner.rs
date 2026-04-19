//! YAML 1.2 lexical scanner.
//!
//! Converts a UTF-8 input string into a stream of [`Token`]s.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 Noyalib. All rights reserved.

// VecDeque replaced with Vec + consumed index for better cache locality.

/// Byte-offset span in the source input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Span {
    pub start: usize,
    pub end: usize,
}

/// The style of a scalar token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

/// Token kinds emitted by the scanner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum TokenKind {
    #[default]
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    BlockSequenceStart,
    BlockMappingStart,
    BlockEnd,
    FlowSequenceStart,
    FlowSequenceEnd,
    FlowMappingStart,
    FlowMappingEnd,
    BlockEntry,
    FlowEntry,
    Key,
    Value,
    Anchor(String),
    Alias(String),
    Tag(String, String),
    Scalar(ScalarStyle, String),
}

/// A token with its source span.
#[derive(Debug, Clone, Default)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Error from the scanner.
#[derive(Debug, Clone)]
pub(crate) struct ScanError {
    pub message: String,
    pub index: usize,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at byte offset {}", self.message, self.index)
    }
}

type ScanResult<T> = Result<T, ScanError>;

/// Tracks potential simple keys.
#[derive(Debug, Clone)]
struct SimpleKey {
    possible: bool,
    required: bool,
    token_number: usize,
    index: usize,
}

/// YAML 1.2 lexical scanner.
#[derive(Debug)]
pub(crate) struct Scanner<'a> {
    input: &'a [u8],
    /// The original input as a `&str` — avoids `from_utf8_lossy` on slices.
    input_str: &'a str,
    pos: usize,
    /// The mark position for the current token start.
    mark: usize,
    /// Current column (tracked incrementally to avoid O(n) backward scan).
    col: usize,
    /// Output token buffer (contiguous for cache locality).
    tokens: Vec<Token>,
    /// Index of the next token to consume from `tokens`.
    tokens_consumed: usize,
    /// Total tokens produced (including consumed ones).
    tokens_produced: usize,
    /// Block indentation level stack.
    indent: i32,
    indents: Vec<i32>,
    /// Flow nesting level (0 = block context).
    flow_level: u32,
    /// Simple key tracking stack.
    simple_keys: Vec<SimpleKey>,
    /// Whether a simple key is allowed at the current position.
    simple_key_allowed: bool,
    /// True once we've emitted StreamStart.
    stream_started: bool,
    /// True once we've emitted StreamEnd.
    stream_ended: bool,
}

impl<'a> Scanner<'a> {
    /// Create a new scanner for the given input.
    pub(super) fn new(input: &'a str) -> Self {
        Scanner {
            input: input.as_bytes(),
            input_str: input,
            pos: 0,
            mark: 0,
            col: 0,
            tokens: Vec::new(),
            tokens_consumed: 0,
            tokens_produced: 0,
            indent: -1,
            indents: Vec::new(),
            flow_level: 0,
            simple_keys: Vec::new(),
            simple_key_allowed: false,
            stream_started: false,
            stream_ended: false,
        }
    }

    /// Fetch the next token from the scanner.
    pub(super) fn next_token(&mut self) -> ScanResult<Token> {
        // Ensure we have at least one token buffered.
        while self.needs_more_tokens() {
            self.fetch_next_token()?;
        }
        if self.tokens_consumed < self.tokens.len() {
            // Move the token out instead of cloning — avoids heap-allocating
            // copies of owned Strings inside Scalar/Anchor/Alias/Tag variants.
            let t = std::mem::take(&mut self.tokens[self.tokens_consumed]);
            self.tokens_consumed += 1;
            self.tokens_produced += 1;
            // Compact when we've consumed enough to avoid unbounded growth.
            if self.tokens_consumed > 64 && self.tokens_consumed > self.tokens.len() / 2 {
                drop(self.tokens.drain(..self.tokens_consumed));
                self.tokens_consumed = 0;
            }
            return Ok(t);
        }
        Err(self.error("unexpected end of token stream"))
    }

    fn needs_more_tokens(&self) -> bool {
        if self.stream_ended {
            return false;
        }
        if self.tokens_consumed >= self.tokens.len() {
            return true;
        }
        // Check if any potential simple key needs to be resolved.
        self.next_simple_key_token_number() == Some(self.tokens_produced)
    }

    fn next_simple_key_token_number(&self) -> Option<usize> {
        let mut min = None;
        for sk in &self.simple_keys {
            if sk.possible {
                match min {
                    None => min = Some(sk.token_number),
                    Some(m) if sk.token_number < m => min = Some(sk.token_number),
                    _ => {}
                }
            }
        }
        min
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    #[inline]
    fn peek(&self) -> u8 {
        if self.pos < self.input.len() {
            self.input[self.pos]
        } else {
            0
        }
    }

    #[inline]
    fn peek_at(&self, offset: usize) -> u8 {
        let idx = self.pos + offset;
        if idx < self.input.len() {
            self.input[idx]
        } else {
            0
        }
    }

    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    #[inline]
    fn advance(&mut self) {
        if self.pos < self.input.len() {
            if self.input[self.pos] == b'\n' {
                self.col = 0;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    #[inline]
    fn advance_by(&mut self, n: usize) {
        let end = (self.pos + n).min(self.input.len());
        for i in self.pos..end {
            if self.input[i] == b'\n' {
                self.col = 0;
            } else {
                self.col += 1;
            }
        }
        self.pos = end;
    }

    #[inline]
    fn column(&self) -> usize {
        self.col
    }

    /// Slice the original `&str` input without allocation.
    #[inline]
    fn slice_str(&self, start: usize, end: usize) -> &'a str {
        &self.input_str[start..end]
    }

    fn error(&self, msg: &str) -> ScanError {
        ScanError {
            message: msg.to_string(),
            index: self.pos,
        }
    }

    fn emit(&mut self, kind: TokenKind) {
        let span = Span {
            start: self.mark,
            end: self.pos,
        };
        self.tokens.push(Token { kind, span });
    }

    fn insert_token(&mut self, index: usize, kind: TokenKind, span: Span) {
        self.tokens
            .insert(self.tokens_consumed + index, Token { kind, span });
    }

    // ── Whitespace / Newline ─────────────────────────────────────────────

    #[inline]
    fn is_blank(c: u8) -> bool {
        c == b' ' || c == b'\t'
    }

    #[inline]
    fn is_break(c: u8) -> bool {
        c == b'\n' || c == b'\r'
    }

    #[inline]
    fn is_blank_or_break(c: u8) -> bool {
        Self::is_blank(c) || Self::is_break(c)
    }

    fn skip_blank(&mut self) {
        while !self.is_eof() && Self::is_blank(self.peek()) {
            self.advance();
        }
    }

    fn skip_line(&mut self) {
        let c = self.peek();
        if c == b'\r' && self.peek_at(1) == b'\n' {
            self.advance_by(2);
        } else if Self::is_break(c) {
            self.advance();
        }
    }

    fn skip_to_next_token(&mut self) -> ScanResult<()> {
        loop {
            // Skip whitespace (tabs are only allowed in some contexts).
            self.skip_blank();

            // Skip comment.
            if self.peek() == b'#' {
                while !self.is_eof() && !Self::is_break(self.peek()) {
                    self.advance();
                }
            }

            // Skip line break.
            if Self::is_break(self.peek()) {
                self.skip_line();
                // In block context, allow simple key at line start.
                if self.flow_level == 0 {
                    self.simple_key_allowed = true;
                    // After a line break in block context, reject tabs as indentation.
                    if self.peek() == b'\t' {
                        return Err(self.error("tab characters are not allowed as indentation"));
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    // ── Indentation ──────────────────────────────────────────────────────

    fn roll_indent(&mut self, column: i32, number: Option<usize>, kind: TokenKind, mark: usize) {
        if self.flow_level > 0 {
            return;
        }
        if self.indent < column {
            self.indents.push(self.indent);
            self.indent = column;
            let span = Span {
                start: mark,
                end: mark,
            };
            match number {
                Some(n) => {
                    let idx = n - self.tokens_produced;
                    self.insert_token(idx, kind, span);
                }
                None => {
                    self.tokens.push(Token { kind, span });
                }
            }
        }
    }

    fn unroll_indent(&mut self, column: i32) {
        if self.flow_level > 0 {
            return;
        }
        while self.indent > column {
            self.indent = self.indents.pop().unwrap_or(-1);
            self.emit(TokenKind::BlockEnd);
        }
    }

    // ── Simple keys ──────────────────────────────────────────────────────

    fn save_simple_key(&mut self) {
        let required = self.flow_level == 0 && self.indent == self.column() as i32;
        if self.simple_key_allowed {
            let sk = SimpleKey {
                possible: true,
                required,
                token_number: self.tokens_produced + (self.tokens.len() - self.tokens_consumed),
                index: self.pos,
            };
            let _ = self.remove_simple_key();
            if let Some(last) = self.simple_keys.last_mut() {
                *last = sk;
            }
        }
    }

    fn remove_simple_key(&mut self) -> ScanResult<()> {
        if let Some(sk) = self.simple_keys.last() {
            if sk.possible && sk.required {
                return Err(ScanError {
                    message: "simple key was required but not found".to_string(),
                    index: sk.index,
                });
            }
        }
        if let Some(sk) = self.simple_keys.last_mut() {
            sk.possible = false;
        }
        Ok(())
    }

    fn stale_simple_keys(&mut self) -> ScanResult<()> {
        // In this implementation we don't enforce the 1024-char limit for
        // simple keys in block context (yaml-rust2 also relaxes this).
        Ok(())
    }

    // ── Main dispatch ────────────────────────────────────────────────────

    fn fetch_next_token(&mut self) -> ScanResult<()> {
        if !self.stream_started {
            return self.fetch_stream_start();
        }

        self.skip_to_next_token()?;
        self.stale_simple_keys()?;
        self.unroll_indent(self.column() as i32);
        self.mark = self.pos;

        if self.is_eof() {
            return self.fetch_stream_end();
        }

        let c = self.peek();

        // Check for document indicators at column 0.
        if self.column() == 0 {
            if c == b'-'
                && self.peek_at(1) == b'-'
                && self.peek_at(2) == b'-'
                && (self.pos + 3 >= self.input.len() || Self::is_blank_or_break(self.peek_at(3)))
            {
                return self.fetch_document_indicator(true);
            }
            if c == b'.'
                && self.peek_at(1) == b'.'
                && self.peek_at(2) == b'.'
                && (self.pos + 3 >= self.input.len() || Self::is_blank_or_break(self.peek_at(3)))
            {
                return self.fetch_document_indicator(false);
            }
        }

        match c {
            b'[' => self.fetch_flow_collection_start(true),
            b'{' => self.fetch_flow_collection_start(false),
            b']' => self.fetch_flow_collection_end(true),
            b'}' => self.fetch_flow_collection_end(false),
            b',' => self.fetch_flow_entry(),
            b'-' if Self::is_blank_or_break(self.peek_at(1))
                || (self.flow_level > 0
                    && (self.peek_at(1) == b','
                        || self.peek_at(1) == b']'
                        || self.peek_at(1) == b'}')) =>
            {
                self.fetch_block_entry()
            }
            b'?' if Self::is_blank_or_break(self.peek_at(1))
                || (self.flow_level > 0
                    && (self.peek_at(1) == b','
                        || self.peek_at(1) == b']'
                        || self.peek_at(1) == b'}')) =>
            {
                self.fetch_key()
            }
            b':' if Self::is_blank_or_break(self.peek_at(1))
                || (self.flow_level > 0
                    && (self.peek_at(1) == b','
                        || self.peek_at(1) == b']'
                        || self.peek_at(1) == b'}'
                        || self.peek_at(1) == b':')) =>
            {
                self.fetch_value()
            }
            b'*' => self.fetch_alias(),
            b'&' => self.fetch_anchor(),
            b'!' => self.fetch_tag(),
            b'|' if self.flow_level == 0 => self.fetch_block_scalar(true),
            b'>' if self.flow_level == 0 => self.fetch_block_scalar(false),
            b'\'' => self.fetch_quoted_scalar(false),
            b'"' => self.fetch_quoted_scalar(true),
            b'%' if self.column() == 0 => self.fetch_directive(),
            // BOM at start
            0xEF if self.pos == 0 && self.peek_at(1) == 0xBB && self.peek_at(2) == 0xBF => {
                self.advance_by(3);
                Ok(())
            }
            _ => self.fetch_plain_scalar(),
        }
    }

    // ── Token fetchers ───────────────────────────────────────────────────

    fn fetch_stream_start(&mut self) -> ScanResult<()> {
        self.stream_started = true;
        self.simple_key_allowed = true;
        self.simple_keys.push(SimpleKey {
            possible: false,
            required: false,
            token_number: 0,
            index: 0,
        });
        // Skip BOM if present.
        if self.pos + 2 < self.input.len()
            && self.input[self.pos] == 0xEF
            && self.input[self.pos + 1] == 0xBB
            && self.input[self.pos + 2] == 0xBF
        {
            self.advance_by(3);
        }
        self.mark = self.pos;
        self.emit(TokenKind::StreamStart);
        Ok(())
    }

    fn fetch_stream_end(&mut self) -> ScanResult<()> {
        // Force-close any open blocks.
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        self.stream_ended = true;
        self.emit(TokenKind::StreamEnd);
        Ok(())
    }

    fn fetch_directive(&mut self) -> ScanResult<()> {
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        // We don't really need to interpret %YAML or %TAG directives for
        // our purposes; just skip to the end of the line.
        while !self.is_eof() && !Self::is_break(self.peek()) {
            self.advance();
        }
        Ok(())
    }

    fn fetch_document_indicator(&mut self, is_start: bool) -> ScanResult<()> {
        self.unroll_indent(-1);
        self.remove_simple_key()?;
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance_by(3);
        if is_start {
            self.emit(TokenKind::DocumentStart);
        } else {
            self.emit(TokenKind::DocumentEnd);
        }
        Ok(())
    }

    fn fetch_flow_collection_start(&mut self, is_seq: bool) -> ScanResult<()> {
        self.save_simple_key();
        self.flow_level += 1;
        self.simple_key_allowed = true;
        self.mark = self.pos;
        self.advance();
        if is_seq {
            self.emit(TokenKind::FlowSequenceStart);
        } else {
            self.emit(TokenKind::FlowMappingStart);
        }
        Ok(())
    }

    fn fetch_flow_collection_end(&mut self, is_seq: bool) -> ScanResult<()> {
        self.remove_simple_key()?;
        if self.flow_level > 0 {
            self.flow_level -= 1;
        }
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance();
        if is_seq {
            self.emit(TokenKind::FlowSequenceEnd);
        } else {
            self.emit(TokenKind::FlowMappingEnd);
        }
        Ok(())
    }

    fn fetch_flow_entry(&mut self) -> ScanResult<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;
        self.advance();
        self.emit(TokenKind::FlowEntry);
        Ok(())
    }

    fn fetch_block_entry(&mut self) -> ScanResult<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed {
                return Err(self.error("block sequence entries are not allowed in this context"));
            }
            let col = self.column() as i32;
            self.roll_indent(col, None, TokenKind::BlockSequenceStart, self.pos);
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;
        self.advance();
        self.emit(TokenKind::BlockEntry);
        Ok(())
    }

    fn fetch_key(&mut self) -> ScanResult<()> {
        if self.flow_level == 0 {
            if !self.simple_key_allowed {
                return Err(self.error("mapping keys are not allowed in this context"));
            }
            let col = self.column() as i32;
            self.roll_indent(col, None, TokenKind::BlockMappingStart, self.pos);
        }
        self.remove_simple_key()?;
        self.simple_key_allowed = self.flow_level > 0;
        self.mark = self.pos;
        self.advance();
        self.emit(TokenKind::Key);
        Ok(())
    }

    fn fetch_value(&mut self) -> ScanResult<()> {
        // Check if there's a pending simple key.
        if let Some(sk) = self.simple_keys.last().cloned() {
            if sk.possible {
                // Insert Key token before the simple key.
                let idx = sk.token_number - self.tokens_produced;
                let span = Span {
                    start: sk.index,
                    end: sk.index,
                };
                self.insert_token(idx, TokenKind::Key, span);

                // Roll indent for block mapping.
                if self.flow_level == 0 {
                    let col = {
                        // Find column of the simple key start.
                        let slice = &self.input[..sk.index];
                        match slice.iter().rposition(|&b| b == b'\n') {
                            Some(nl) => sk.index - nl - 1,
                            None => sk.index,
                        }
                    } as i32;
                    self.roll_indent(
                        col,
                        Some(sk.token_number),
                        TokenKind::BlockMappingStart,
                        sk.index,
                    );
                }

                if let Some(last) = self.simple_keys.last_mut() {
                    last.possible = false;
                }
                self.simple_key_allowed = false;
            } else {
                // No simple key. Must be a complex value indicator.
                if self.flow_level == 0 {
                    if !self.simple_key_allowed {
                        return Err(self.error("mapping values are not allowed in this context"));
                    }
                    let col = self.column() as i32;
                    self.roll_indent(col, None, TokenKind::BlockMappingStart, self.pos);
                }
                self.simple_key_allowed = self.flow_level == 0;
            }
        } else {
            // No simple key tracking. Must be a complex value.
            if self.flow_level == 0 && !self.simple_key_allowed {
                return Err(self.error("mapping values are not allowed in this context"));
            }
            if self.flow_level == 0 {
                let col = self.column() as i32;
                self.roll_indent(col, None, TokenKind::BlockMappingStart, self.pos);
            }
            self.simple_key_allowed = self.flow_level == 0;
        }

        self.mark = self.pos;
        self.advance();
        self.emit(TokenKind::Value);
        Ok(())
    }

    fn fetch_anchor(&mut self) -> ScanResult<()> {
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance(); // skip '&'
        let name = self.scan_anchor_name()?;
        self.emit(TokenKind::Anchor(name));
        Ok(())
    }

    fn fetch_alias(&mut self) -> ScanResult<()> {
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance(); // skip '*'
        let name = self.scan_anchor_name()?;
        self.emit(TokenKind::Alias(name));
        Ok(())
    }

    fn scan_anchor_name(&mut self) -> ScanResult<String> {
        /// Maximum length of an anchor or alias name (in bytes).
        const MAX_ANCHOR_NAME_LEN: usize = 1024;

        let start = self.pos;
        while !self.is_eof() {
            if self.pos - start > MAX_ANCHOR_NAME_LEN {
                return Err(self.error("anchor name exceeds maximum length of 1024 bytes"));
            }
            let c = self.peek();
            if Self::is_blank_or_break(c)
                || c == b','
                || c == b'['
                || c == b']'
                || c == b'{'
                || c == b'}'
                || c == b':'
            {
                break;
            }
            self.advance();
        }
        if self.pos == start {
            return Err(self.error("expected anchor or alias name"));
        }
        Ok(self.slice_str(start, self.pos).to_owned())
    }

    fn fetch_tag(&mut self) -> ScanResult<()> {
        /// Maximum length of a tag URI (in bytes).
        const MAX_TAG_LEN: usize = 1024;

        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;
        self.advance(); // skip '!'

        let mut handle = String::from("!");
        let suffix;

        if self.peek() == b'<' {
            // Verbatim tag: !<...>
            self.advance(); // skip '<'
            let start = self.pos;
            while !self.is_eof() && self.peek() != b'>' {
                if self.pos - start > MAX_TAG_LEN {
                    return Err(self.error("tag URI exceeds maximum length of 1024 bytes"));
                }
                self.advance();
            }
            suffix = self.slice_str(start, self.pos).to_owned();
            if self.peek() == b'>' {
                self.advance();
            }
        } else if self.peek() == b'!' {
            // Secondary tag handle !!
            handle.push('!');
            self.advance();
            let start = self.pos;
            while !self.is_eof()
                && !Self::is_blank_or_break(self.peek())
                && self.peek() != b','
                && self.peek() != b'['
                && self.peek() != b']'
                && self.peek() != b'{'
                && self.peek() != b'}'
            {
                if self.pos - start > MAX_TAG_LEN {
                    return Err(self.error("tag suffix exceeds maximum length of 1024 bytes"));
                }
                self.advance();
            }
            suffix = self.slice_str(start, self.pos).to_owned();
        } else {
            // Primary tag handle !suffix or just !
            let start = self.pos;
            while !self.is_eof()
                && !Self::is_blank_or_break(self.peek())
                && self.peek() != b','
                && self.peek() != b'['
                && self.peek() != b']'
                && self.peek() != b'{'
                && self.peek() != b'}'
            {
                if self.pos - start > MAX_TAG_LEN {
                    return Err(self.error("tag suffix exceeds maximum length of 1024 bytes"));
                }
                self.advance();
            }
            suffix = self.slice_str(start, self.pos).to_owned();
        }

        self.emit(TokenKind::Tag(handle, suffix));
        Ok(())
    }

    // ── Scalars ──────────────────────────────────────────────────────────

    fn fetch_plain_scalar(&mut self) -> ScanResult<()> {
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;

        let mut string = String::new();
        let mut leading_blanks = false;
        let mut whitespace = String::new();
        let indent = self.indent + 1;

        loop {
            // Skip to the end of the current run.
            let mut length = 0;

            loop {
                let c = self.peek_at(length);

                if c == 0 && self.pos + length >= self.input.len() {
                    break;
                }

                // Check for ':' followed by blank/flow-indicator, or flow indicators.
                if c == b':' {
                    let next = self.peek_at(length + 1);
                    if Self::is_blank_or_break(next)
                        || (self.flow_level > 0 && (next == b',' || next == b']' || next == b'}'))
                    {
                        break;
                    }
                }

                if self.flow_level > 0
                    && (c == b',' || c == b'[' || c == b']' || c == b'{' || c == b'}')
                {
                    break;
                }

                if Self::is_blank_or_break(c) || c == b'#' {
                    break;
                }

                // Check for document indicators at start of line.
                if length == 0 && self.column() == 0 {
                    if c == b'-'
                        && self.peek_at(1) == b'-'
                        && self.peek_at(2) == b'-'
                        && (self.pos + 3 >= self.input.len()
                            || Self::is_blank_or_break(self.peek_at(3)))
                    {
                        break;
                    }
                    if c == b'.'
                        && self.peek_at(1) == b'.'
                        && self.peek_at(2) == b'.'
                        && (self.pos + 3 >= self.input.len()
                            || Self::is_blank_or_break(self.peek_at(3)))
                    {
                        break;
                    }
                }

                length += 1;
            }

            if length == 0 && !leading_blanks {
                break;
            }

            // Append characters.
            if length > 0 {
                if leading_blanks {
                    // Handle line joins.
                    if let Some(stripped) = whitespace.strip_prefix('\n') {
                        if stripped.is_empty() {
                            string.push(' ');
                        } else {
                            // Multiple line breaks.
                            string.push_str(stripped);
                        }
                    } else {
                        string.push_str(&whitespace);
                    }
                    whitespace.clear();
                } else if !whitespace.is_empty() {
                    string.push_str(&whitespace);
                    whitespace.clear();
                }

                string.push_str(self.slice_str(self.pos, self.pos + length));
                self.advance_by(length);
            }

            // Skip whitespace/newlines between plain scalar content.
            if !Self::is_blank_or_break(self.peek()) {
                break;
            }

            whitespace.clear();

            // Consume blanks and breaks.
            while Self::is_blank(self.peek()) {
                whitespace.push(self.peek() as char);
                self.advance();
            }

            if Self::is_break(self.peek()) {
                leading_blanks = true;
                whitespace.clear();
                // Consume the break.
                let c = self.peek();
                if c == b'\r' && self.peek_at(1) == b'\n' {
                    whitespace.push('\n');
                    self.advance_by(2);
                } else {
                    whitespace.push('\n');
                    self.advance();
                }

                // Consume subsequent line breaks.
                while Self::is_break(self.peek()) {
                    let c = self.peek();
                    if c == b'\r' && self.peek_at(1) == b'\n' {
                        whitespace.push('\n');
                        self.advance_by(2);
                    } else {
                        whitespace.push('\n');
                        self.advance();
                    }
                }

                // Skip leading blanks on the new line, check indent.
                while Self::is_blank(self.peek()) {
                    self.advance();
                }

                if self.flow_level == 0 && (self.column() as i32) < indent {
                    break;
                }
            }
            // else: inline blanks between words — continue scanning.
        }

        if string.is_empty() {
            return Err(self.error("unexpected character in YAML stream"));
        }

        // If the plain scalar ended with the scanner at a new line in block
        // context (e.g. the value was followed by a newline that was consumed
        // during line folding), simple keys must be allowed again so that a
        // following key at the same indent level can be recognised.
        if self.flow_level == 0 && leading_blanks {
            self.simple_key_allowed = true;
        }

        self.emit(TokenKind::Scalar(ScalarStyle::Plain, string));
        Ok(())
    }

    fn fetch_quoted_scalar(&mut self, double: bool) -> ScanResult<()> {
        self.save_simple_key();
        self.simple_key_allowed = false;
        self.mark = self.pos;

        let string = if double {
            self.scan_double_quoted_scalar()?
        } else {
            self.scan_single_quoted_scalar()?
        };

        let style = if double {
            ScalarStyle::DoubleQuoted
        } else {
            ScalarStyle::SingleQuoted
        };
        self.emit(TokenKind::Scalar(style, string));
        Ok(())
    }

    fn scan_single_quoted_scalar(&mut self) -> ScanResult<String> {
        self.advance(); // skip opening '

        let mut string = String::new();
        let mut whitespace = String::new();
        let mut leading_break = false;

        loop {
            if self.is_eof() {
                return Err(self.error("unterminated single-quoted string"));
            }

            match self.peek() {
                b'\'' => {
                    if self.peek_at(1) == b'\'' {
                        // Escaped single quote.
                        string.push_str(&whitespace);
                        whitespace.clear();
                        leading_break = false;
                        string.push('\'');
                        self.advance_by(2);
                    } else {
                        // End of string.
                        self.advance();
                        return Ok(string);
                    }
                }
                c if Self::is_break(c) => {
                    // Line folding.
                    if !whitespace.is_empty() && !leading_break {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                    }

                    leading_break = true;
                    whitespace.clear();

                    if self.peek() == b'\r' && self.peek_at(1) == b'\n' {
                        self.advance_by(2);
                    } else {
                        self.advance();
                    }

                    // Consume subsequent breaks and leading spaces.
                    while Self::is_break(self.peek()) {
                        whitespace.push('\n');
                        if self.peek() == b'\r' && self.peek_at(1) == b'\n' {
                            self.advance_by(2);
                        } else {
                            self.advance();
                        }
                    }

                    while Self::is_blank(self.peek()) {
                        self.advance();
                    }
                }
                _ => {
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        leading_break = false;
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    // Read a character. Handle UTF-8 properly.
                    let start = self.pos;
                    self.advance();
                    // Check for multi-byte UTF-8.
                    while self.pos < self.input.len() && (self.input[self.pos] & 0xC0) == 0x80 {
                        self.advance();
                    }
                    string.push_str(self.slice_str(start, self.pos));
                }
            }
        }
    }

    fn scan_double_quoted_scalar(&mut self) -> ScanResult<String> {
        self.advance(); // skip opening "

        let mut string = String::new();
        let mut whitespace = String::new();
        let mut leading_break = false;

        loop {
            if self.is_eof() {
                return Err(self.error("unterminated double-quoted string"));
            }

            match self.peek() {
                b'"' => {
                    // Flush any pending whitespace before closing.
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                    }
                    self.advance();
                    return Ok(string);
                }
                b'\\' => {
                    // Flush pending whitespace.
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        leading_break = false;
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    self.advance(); // skip '\'
                    if self.is_eof() {
                        return Err(self.error("unexpected end of input in escape sequence"));
                    }
                    let escaped = self.peek();
                    self.advance();
                    match escaped {
                        b'0' => string.push('\0'),
                        b'a' => string.push('\x07'),
                        b'b' => string.push('\x08'),
                        b't' | b'\t' => string.push('\t'),
                        b'n' => string.push('\n'),
                        b'v' => string.push('\x0B'),
                        b'f' => string.push('\x0C'),
                        b'r' => string.push('\r'),
                        b'e' => string.push('\x1B'),
                        b' ' => string.push(' '),
                        b'"' => string.push('"'),
                        b'/' => string.push('/'),
                        b'\\' => string.push('\\'),
                        b'N' => string.push('\u{0085}'), // NEL
                        b'_' => string.push('\u{00A0}'), // NBSP
                        b'L' => string.push('\u{2028}'), // LS
                        b'P' => string.push('\u{2029}'), // PS
                        b'x' => {
                            let ch = self.scan_hex_escape(2)?;
                            string.push(ch);
                        }
                        b'u' => {
                            let ch = self.scan_hex_escape(4)?;
                            string.push(ch);
                        }
                        b'U' => {
                            let ch = self.scan_hex_escape(8)?;
                            string.push(ch);
                        }
                        b'\r' | b'\n' => {
                            // Line break escape — fold.
                            if escaped == b'\r' && self.peek() == b'\n' {
                                self.advance();
                            }
                            // Skip leading whitespace on next line.
                            while Self::is_blank(self.peek()) {
                                self.advance();
                            }
                        }
                        _ => {
                            return Err(ScanError {
                                message: format!(
                                    "unknown escape character '\\{}'",
                                    escaped as char
                                ),
                                index: self.pos - 1,
                            });
                        }
                    }
                }
                c if Self::is_break(c) => {
                    // Line folding in double-quoted scalars.
                    if !whitespace.is_empty() && !leading_break {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                    }

                    leading_break = true;
                    whitespace.clear();

                    if self.peek() == b'\r' && self.peek_at(1) == b'\n' {
                        self.advance_by(2);
                    } else {
                        self.advance();
                    }

                    // Consume subsequent breaks.
                    while Self::is_break(self.peek()) {
                        whitespace.push('\n');
                        if self.peek() == b'\r' && self.peek_at(1) == b'\n' {
                            self.advance_by(2);
                        } else {
                            self.advance();
                        }
                    }

                    // Skip leading blanks on new line.
                    while Self::is_blank(self.peek()) {
                        self.advance();
                    }
                }
                _ => {
                    if leading_break {
                        if whitespace.is_empty() {
                            string.push(' ');
                        } else {
                            string.push_str(&whitespace);
                        }
                        whitespace.clear();
                        leading_break = false;
                    } else if !whitespace.is_empty() {
                        string.push_str(&whitespace);
                        whitespace.clear();
                    }

                    // Handle whitespace chars specially for folding.
                    if Self::is_blank(self.peek()) {
                        let start = self.pos;
                        while Self::is_blank(self.peek()) {
                            self.advance();
                        }
                        if Self::is_break(self.peek())
                            || self.peek() == b'"'
                            || self.peek() == b'\\'
                        {
                            whitespace.push_str(self.slice_str(start, self.pos));
                            continue;
                        }
                        string.push_str(self.slice_str(start, self.pos));
                    } else {
                        let start = self.pos;
                        self.advance();
                        while self.pos < self.input.len() && (self.input[self.pos] & 0xC0) == 0x80 {
                            self.advance();
                        }
                        string.push_str(self.slice_str(start, self.pos));
                    }
                }
            }
        }
    }

    fn scan_hex_escape(&mut self, digits: usize) -> ScanResult<char> {
        let start = self.pos;
        for _ in 0..digits {
            if self.is_eof() || !self.peek().is_ascii_hexdigit() {
                return Err(ScanError {
                    message: format!("expected {} hex digits in escape sequence", digits),
                    index: start,
                });
            }
            self.advance();
        }
        let hex_str = self.slice_str(start, self.pos);
        let code =
            u32::from_str_radix(hex_str, 16).map_err(|_| self.error("invalid hex escape"))?;
        char::from_u32(code).ok_or_else(|| ScanError {
            message: format!("invalid Unicode code point U+{code:04X}"),
            index: start,
        })
    }

    fn fetch_block_scalar(&mut self, literal: bool) -> ScanResult<()> {
        self.remove_simple_key()?;
        self.simple_key_allowed = true;
        self.mark = self.pos;

        let string = self.scan_block_scalar(literal)?;
        let style = if literal {
            ScalarStyle::Literal
        } else {
            ScalarStyle::Folded
        };
        self.emit(TokenKind::Scalar(style, string));
        Ok(())
    }

    fn scan_block_scalar(&mut self, literal: bool) -> ScanResult<String> {
        self.advance(); // skip '|' or '>'

        // Parse optional chomping indicator and indentation indicator.
        let mut chomping: i8 = 0; // 0 = clip, 1 = keep, -1 = strip
        let mut increment: usize = 0;

        // Check for chomping/indent indicators in either order.
        for _ in 0..2 {
            if !self.is_eof() {
                match self.peek() {
                    b'+' => {
                        chomping = 1;
                        self.advance();
                    }
                    b'-' => {
                        chomping = -1;
                        self.advance();
                    }
                    c if c.is_ascii_digit() && c != b'0' => {
                        increment = (c - b'0') as usize;
                        self.advance();
                    }
                    _ => break,
                }
            }
        }

        // Skip to end of line (including optional comment).
        while Self::is_blank(self.peek()) {
            self.advance();
        }
        if self.peek() == b'#' {
            while !self.is_eof() && !Self::is_break(self.peek()) {
                self.advance();
            }
        }

        // Consume the line break.
        if Self::is_break(self.peek()) {
            self.skip_line();
        }

        // Determine the indentation level.
        let block_indent;
        if increment > 0 {
            block_indent = if self.indent >= 0 {
                self.indent as usize + increment
            } else {
                increment
            };
        } else {
            // Auto-detect: find the first non-empty line's indentation.
            let mut detected = 0;
            let save = self.pos;
            loop {
                let mut spaces = 0;
                while self.peek() == b' ' {
                    spaces += 1;
                    self.advance();
                }
                if Self::is_break(self.peek()) {
                    self.skip_line();
                    continue;
                }
                if self.is_eof() {
                    break;
                }
                detected = spaces;
                break;
            }
            self.pos = save;
            let min_indent = if self.indent >= 0 {
                self.indent as usize + 1
            } else {
                1
            };
            block_indent = detected.max(min_indent);
        }

        // Read the block scalar content.
        let mut string = String::new();
        let mut trailing_breaks = String::new();
        let mut leading_blank = false;

        while !self.is_eof() {
            // Count leading spaces.
            let mut spaces = 0;
            while self.peek() == b' ' {
                spaces += 1;
                self.advance();
            }

            if spaces < block_indent && !Self::is_break(self.peek()) && !self.is_eof() {
                // End of block scalar.
                break;
            }

            // Handle extra indentation.
            if spaces > block_indent {
                // More-indented line: include extra spaces in the content.
                if !trailing_breaks.is_empty() {
                    string.push_str(&trailing_breaks);
                    trailing_breaks.clear();
                    if !literal && leading_blank {
                        // nothing special
                    }
                }
                let extra = spaces - block_indent;
                for _ in 0..extra {
                    string.push(' ');
                }
            }

            if Self::is_break(self.peek()) || self.is_eof() {
                // Empty line.
                if !Self::is_break(self.peek()) {
                    break;
                }
                trailing_breaks.push('\n');
                self.skip_line();
                continue;
            }

            // Fold or preserve line breaks.
            if !trailing_breaks.is_empty() {
                if literal {
                    string.push_str(&trailing_breaks);
                } else {
                    // Folded: single break becomes space, multiple breaks preserved.
                    if trailing_breaks.len() == 1 {
                        if leading_blank || self.peek() == b' ' || self.peek() == b'\t' {
                            string.push('\n');
                        } else {
                            string.push(' ');
                        }
                    } else {
                        // Keep all but first break.
                        string.push_str(&trailing_breaks[1..]);
                    }
                }
                trailing_breaks.clear();
            }

            leading_blank = self.peek() == b' ' || self.peek() == b'\t';

            // Read content of the line.
            while !self.is_eof() && !Self::is_break(self.peek()) {
                let start = self.pos;
                self.advance();
                while self.pos < self.input.len() && (self.input[self.pos] & 0xC0) == 0x80 {
                    self.advance();
                }
                string.push_str(self.slice_str(start, self.pos));
            }

            // Consume the line break.
            if Self::is_break(self.peek()) {
                trailing_breaks.push('\n');
                self.skip_line();
            }
        }

        // Apply chomping.
        match chomping {
            1 => {
                // Keep: append all trailing breaks.
                string.push_str(&trailing_breaks);
            }
            // Clip: append single trailing newline.
            0 if !string.is_empty() || !trailing_breaks.is_empty() => {
                string.push('\n');
            }
            _ => {
                // Strip (or clip with empty content): don't append anything.
            }
        }

        Ok(string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_two_key_mapping() {
        let input = "name: test\nitem: Unit\n";
        let mut scanner = Scanner::new(input);
        let mut tokens = Vec::new();
        loop {
            let result = scanner.next_token();
            match result {
                Ok(t) => {
                    let is_end = matches!(t.kind, TokenKind::StreamEnd);
                    tokens.push(format!("{:?}", t.kind));
                    if is_end {
                        break;
                    }
                }
                Err(e) => {
                    panic!("Scanner error after tokens {tokens:#?}: {e}");
                }
            }
        }
        // Expected: StreamStart, BlockMappingStart, Key, Scalar(name),
        //           Value, Scalar(test), Key, Scalar(item), Value, Scalar(Unit),
        //           BlockEnd, StreamEnd
        let expected_contains = vec![
            "StreamStart",
            "BlockMappingStart",
            "Key",
            "Scalar(Plain, \"name\")",
            "Value",
            "Scalar(Plain, \"test\")",
            "Key",
            "Scalar(Plain, \"item\")",
            "Value",
            "Scalar(Plain, \"Unit\")",
            "BlockEnd",
            "StreamEnd",
        ];
        for (i, exp) in expected_contains.iter().enumerate() {
            assert!(
                i < tokens.len() && tokens[i].contains(exp),
                "Token {i}: expected to contain {exp:?}, got {:?}\nAll tokens: {tokens:#?}",
                tokens.get(i)
            );
        }
    }
}
